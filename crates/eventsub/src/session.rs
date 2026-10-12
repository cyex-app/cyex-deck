//! One EventSub websocket. Says when a fresh session needs its subscriptions, follows
//! Twitch's reconnect requests (subscriptions carry over), drops repeated messages and
//! starts over with backoff when the connection goes quiet or closes.

use std::collections::VecDeque;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::{mpsc, watch};
use tokio::time::{Instant, sleep, sleep_until, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::frame::{self, Frame};

pub const URL: &str = "wss://eventsub.wss.twitch.tv/ws";
/// Twitch may send a message twice; this many ids are remembered.
const SEEN: usize = 300;
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Extra wait on top of the keepalive Twitch promised.
const SLACK: Duration = Duration::from_secs(5);

type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

#[derive(Debug)]
pub enum Out {
    /// A fresh session: subscribe everything again with this id, within 10 s.
    Welcome(String),
    /// The connection dropped; nothing is subscribed until the next welcome.
    Lost,
    Notification { kind: String, condition: Value, event: Value },
    Revoked { kind: String, status: String, condition: Value },
}

/// Dropping it ends the session.
pub struct Session {
    _stop: watch::Sender<()>,
}

/// The handle and the task to spawn.
pub fn start(out: mpsc::UnboundedSender<Out>) -> (Session, impl Future<Output = ()> + Send) {
    let (tx, rx) = watch::channel(());
    (Session { _stop: tx }, run(rx, out))
}

async fn run(mut stop: watch::Receiver<()>, out: mpsc::UnboundedSender<Out>) {
    let mut backoff = Duration::ZERO;
    let mut seen: VecDeque<String> = VecDeque::new();
    loop {
        if !backoff.is_zero() {
            tokio::select! {
                _ = sleep(backoff) => {}
                _ = stop.changed() => return,
            }
        }
        let Some(mut ws) = connect(URL, &mut stop).await else {
            if stop.has_changed().is_err() {
                return;
            }
            backoff = next(backoff);
            continue;
        };
        let mut keepalive = Duration::from_secs(10);
        // Only Twitch's own messages count as signs of life, not pings.
        let mut deadline = Instant::now() + keepalive + SLACK;
        loop {
            let frame = tokio::select! {
                f = ws.next() => f,
                _ = sleep_until(deadline) => break,
                _ = stop.changed() => return,
            };
            let text = match frame {
                Some(Ok(Message::Text(t))) => t,
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => continue,
            };
            deadline = Instant::now() + keepalive + SLACK;
            match frame::parse(&text) {
                Some(Frame::Welcome { session_id, keepalive_secs }) => {
                    backoff = Duration::ZERO;
                    keepalive = Duration::from_secs(keepalive_secs.max(1));
                    deadline = Instant::now() + keepalive + SLACK;
                    let _ = out.send(Out::Welcome(session_id));
                }
                Some(Frame::Notification {
                    message_id,
                    kind,
                    condition,
                    event,
                }) => {
                    if seen.contains(&message_id) {
                        continue;
                    }
                    seen.push_back(message_id);
                    if seen.len() > SEEN {
                        seen.pop_front();
                    }
                    let _ = out.send(Out::Notification { kind, condition, event });
                }
                Some(Frame::Revocation { kind, status, condition }) => {
                    let _ = out.send(Out::Revoked { kind, status, condition });
                }
                Some(Frame::Reconnect { url }) => match moved(&url, &mut stop).await {
                    Some((next_ws, secs)) => {
                        ws = next_ws;
                        keepalive = Duration::from_secs(secs.max(1));
                        deadline = Instant::now() + keepalive + SLACK;
                    }
                    None => break,
                },
                Some(Frame::Keepalive) | None => {}
            }
        }
        let _ = out.send(Out::Lost);
        backoff = next(backoff);
    }
}

async fn connect(url: &str, stop: &mut watch::Receiver<()>) -> Option<Socket> {
    tokio::select! {
        r = timeout(Duration::from_secs(15), connect_async(url)) => r.ok()?.ok().map(|(ws, _)| ws),
        _ = stop.changed() => None,
    }
}

/// Opens the new connection and waits for its welcome. The old one closes after.
async fn moved(url: &str, stop: &mut watch::Receiver<()>) -> Option<(Socket, u64)> {
    let mut ws = connect(url, stop).await?;
    let welcome = timeout(Duration::from_secs(10), async {
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Text(t) = msg
                && let Some(Frame::Welcome { keepalive_secs, .. }) = frame::parse(&t)
            {
                return Some(keepalive_secs);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()?;
    Some((ws, welcome))
}

fn next(backoff: Duration) -> Duration {
    if backoff.is_zero() {
        Duration::from_secs(1)
    } else {
        (backoff * 2).min(MAX_BACKOFF)
    }
}
