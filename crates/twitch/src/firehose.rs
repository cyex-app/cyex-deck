//! V104-02 logs.cyex.app's firehose (refs 5.16): a websocket that pushes every message the
//! logs keep, from every channel they follow, as raw IRC lines. Read only: the app never
//! sends anything on it (the server would drop it anyway), and nothing about what the
//! user looks for leaves the app; the server has no filters and none are asked for.

use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::watch;
use tokio::time::{Instant, sleep_until, timeout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub const URL: &str = "wss://logs.cyex.app/firehose";
/// The firehose never goes quiet for this long while it works.
const IDLE: Duration = Duration::from_secs(60);
const CONNECT: Duration = Duration::from_secs(20);
const FIRST_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(120);
/// A connection that held this long starts the backoff over.
const STEADY: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Connecting,
    Connected,
    /// Trying again in this many seconds.
    Offline(u64),
}

/// Reads the firehose until `stop` says so (or its sender is gone), reconnecting with
/// backoff. `line` sees every message as it arrives, on this task, so it must be quick.
pub async fn run(
    url: &str,
    mut stop: watch::Receiver<bool>,
    mut line: impl FnMut(&str),
    mut status: impl FnMut(Status),
) {
    let mut backoff = Duration::ZERO;
    loop {
        if *stop.borrow() {
            return;
        }
        if !backoff.is_zero() {
            status(Status::Offline(backoff.as_secs()));
            let until = Instant::now() + backoff;
            tokio::select! {
                _ = sleep_until(until) => {}
                r = stop.changed() => if r.is_err() || *stop.borrow() { return },
            }
        }
        status(Status::Connecting);
        let ws = tokio::select! {
            r = timeout(CONNECT, connect_async(url)) => r.ok().and_then(Result::ok),
            r = stop.changed() => if r.is_err() || *stop.borrow() { return } else { None },
        };
        let Some((ws, _)) = ws else {
            backoff = next(backoff);
            continue;
        };
        status(Status::Connected);
        let started = Instant::now();
        let (_sink, mut stream) = ws.split();
        loop {
            tokio::select! {
                msg = timeout(IDLE, stream.next()) => match msg {
                    Ok(Some(Ok(Message::Text(text)))) => line(&text),
                    Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) | Err(_) => break,
                    Ok(Some(Ok(_))) => {}
                },
                r = stop.changed() => if r.is_err() || *stop.borrow() { return },
            }
        }
        // Closed, dropped for being slow (1008) or silent: start again after a pause.
        backoff = if started.elapsed() >= STEADY {
            FIRST_BACKOFF
        } else {
            next(backoff)
        };
    }
}

fn next(backoff: Duration) -> Duration {
    if backoff.is_zero() {
        FIRST_BACKOFF
    } else {
        (backoff * 2).min(MAX_BACKOFF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_to_a_cap() {
        let mut b = Duration::ZERO;
        let mut seen = Vec::new();
        for _ in 0..8 {
            b = next(b);
            seen.push(b.as_secs());
        }
        assert_eq!(seen, [2, 4, 8, 16, 32, 64, 120, 120]);
    }

    #[tokio::test]
    async fn stops_while_offline() {
        let (tx, rx) = watch::channel(false);
        let mut states = Vec::new();
        // Not a websocket address: fails at once, without the network.
        let task = run("not a url", rx, |_| {}, |s| states.push(s));
        let stopper = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let _ = tx.send(true);
        };
        let (_, ()) = tokio::join!(timeout(Duration::from_secs(10), task), stopper);
        assert_eq!(states.first(), Some(&Status::Connecting));
        assert!(states.contains(&Status::Offline(2)), "{states:?}");
    }
}
