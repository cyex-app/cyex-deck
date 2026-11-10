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
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async_with_config};

use crate::frame::{self, Frame};

pub const URL: &str = "wss://eventsub.wss.twitch.tv/ws";
/// Twitch may send a message twice; this many ids are remembered.
const SEEN: usize = 300;
/// V110-A21 The shared reconnect backoff: 1-3 s at first, doubling to a minute.
const BACKOFF: cyex_core::backoff::Backoff = cyex_core::backoff::Backoff::new(
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(60),
);
/// Extra wait on top of the keepalive Twitch promised.
const SLACK: Duration = Duration::from_secs(5);
/// V110-F52 A session must live this long before its drop restarts the backoff.
const STABLE: Duration = Duration::from_secs(60);

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

/// The handle and the task to spawn, for `account`'s token. V107-R17 The task first waits
/// for a slot in [`crate::gate::GATE`] (never a 4th socket per token) and holds it until it ends.
pub fn start(account: &str, out: mpsc::UnboundedSender<Out>) -> (Session, impl Future<Output = ()> + Send + use<>) {
    let (tx, mut rx) = watch::channel(());
    let account = account.to_owned();
    let task = async move {
        let slot = tokio::select! {
            s = crate::gate::GATE.acquire(&account) => s,
            _ = rx.changed() => return,
        };
        run(rx, out).await;
        drop(slot);
    };
    (Session { _stop: tx }, task)
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
            cyex_core::health::note("EventSub", "connect failed");
            backoff = next(backoff);
            continue;
        };
        let started = Instant::now();
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
                    keepalive = keepalive_for(keepalive_secs);
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
                        keepalive = keepalive_for(secs);
                        deadline = Instant::now() + keepalive + SLACK;
                    }
                    None => break,
                },
                Some(Frame::Keepalive) | None => {}
            }
        }
        let _ = out.send(Out::Lost);
        cyex_core::health::note("EventSub", "dropped");
        backoff = next(if started.elapsed() >= STABLE { Duration::ZERO } else { backoff });
    }
}

/// V110-F94 The most one message may take, as on the app's other sockets (the library takes
/// 64 MiB by default; an EventSub message is a few KB).
const MAX_MESSAGE: usize = 4 << 20;

async fn connect(url: &str, stop: &mut watch::Receiver<()>) -> Option<Socket> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    tokio::select! {
        r = timeout(Duration::from_secs(15), connect_async_with_config(url, Some(config), false)) => r.ok()?.ok().map(|(ws, _)| ws),
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

/// V110-F80 Twitch allows 10-600 s; a huge value from the wire would overflow `Instant` math.
fn keepalive_for(secs: u64) -> Duration {
    Duration::from_secs(secs.clamp(10, 600))
}

/// 1-3 s at first (jittered), then doubling with up to a quarter on top.
fn next(backoff: Duration) -> Duration {
    BACKOFF.next(backoff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keepalive_is_clamped() {
        assert_eq!(keepalive_for(0), Duration::from_secs(10));
        assert_eq!(keepalive_for(30), Duration::from_secs(30));
        assert_eq!(keepalive_for(u64::MAX), Duration::from_secs(600));
        assert!(Instant::now().checked_add(keepalive_for(u64::MAX) + SLACK).is_some());
    }

    #[test]
    fn backoff_starts_jittered_and_grows_to_a_cap() {
        let first = next(Duration::ZERO);
        assert!((1000..3000).contains(&(first.as_millis() as u64)));
        let grown = next(Duration::from_secs(4));
        assert!(grown >= Duration::from_secs(8) && grown < Duration::from_secs(10));
        assert!(next(Duration::from_secs(600)) <= BACKOFF.cap + BACKOFF.cap / 4);
    }

    #[tokio::test]
    async fn v110_f94_a_message_over_the_cap_ends_the_read() {
        use futures_util::SinkExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            let _ = ws.send(Message::text("small")).await;
            let _ = ws.send(Message::binary(vec![0u8; MAX_MESSAGE + 1])).await;
            let _ = ws.next().await;
        });
        let (_keep, mut stop) = watch::channel(());
        let mut ws = connect(&url, &mut stop).await.unwrap();
        assert!(matches!(ws.next().await, Some(Ok(Message::Text(t))) if t.as_str() == "small"));
        assert!(matches!(ws.next().await, Some(Err(_))));
    }
}
