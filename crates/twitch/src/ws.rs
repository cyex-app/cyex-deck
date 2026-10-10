//! One websocket that follows a set of topics: connects while there are any, sends them
//! all again after every reconnect, and retries with backoff. 7TV's EventAPI, BTTV's
//! socket and Twitch's Hermes run on it.

use std::collections::HashSet;
use std::hash::Hash;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const MAX_BACKOFF: Duration = Duration::from_secs(120);

pub trait Protocol: Send + 'static {
    type Topic: Clone + Eq + Hash + Send + Sync + 'static;
    type Event: Send + 'static;

    fn url(&self) -> String;
    fn subscribe(&self, topic: &Self::Topic) -> String;
    fn unsubscribe(&self, topic: &Self::Topic) -> String;
    /// Most topics one connection may hold.
    fn limit(&self) -> usize;
    /// Silence longer than this means the connection is dead.
    fn idle(&self) -> Duration;
    /// Reads one text frame.
    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Self::Event>) -> Step;
}

pub enum Step {
    Continue,
    /// The server asked for a fresh connection.
    Reconnect,
}

pub enum Cmd<T> {
    Add(T),
    Remove(T),
    /// A one-off line, sent only while connected.
    Send(String),
}

/// Handle for adding and removing topics. Dropping it ends the task.
pub struct Topics<T> {
    tx: mpsc::UnboundedSender<Cmd<T>>,
}

impl<T> Topics<T> {
    pub fn add(&self, topic: T) {
        let _ = self.tx.send(Cmd::Add(topic));
    }

    pub fn remove(&self, topic: T) {
        let _ = self.tx.send(Cmd::Remove(topic));
    }

    pub fn send(&self, line: String) {
        let _ = self.tx.send(Cmd::Send(line));
    }
}

/// The handle and the task to spawn.
pub fn start<P: Protocol>(
    protocol: P,
    events: mpsc::UnboundedSender<P::Event>,
) -> (Topics<P::Topic>, impl Future<Output = ()> + Send) {
    let (tx, rx) = mpsc::unbounded_channel();
    (Topics { tx }, run(protocol, rx, events))
}

async fn run<P: Protocol>(
    mut p: P,
    mut cmds: mpsc::UnboundedReceiver<Cmd<P::Topic>>,
    events: mpsc::UnboundedSender<P::Event>,
) {
    let mut topics: HashSet<P::Topic> = HashSet::new();
    let mut backoff = Duration::ZERO;
    loop {
        // Idle until there's something to follow.
        while topics.is_empty() {
            match cmds.recv().await {
                Some(c) => apply(&mut topics, c, p.limit()),
                None => return,
            }
        }
        if !backoff.is_zero() {
            let until = Instant::now() + backoff;
            loop {
                tokio::select! {
                    _ = sleep_until(until) => break,
                    c = cmds.recv() => match c {
                        Some(c) => apply(&mut topics, c, p.limit()),
                        None => return,
                    },
                }
            }
            if topics.is_empty() {
                continue;
            }
        }
        let Ok((ws, _)) = connect_async(p.url()).await else {
            backoff = next_backoff(backoff);
            continue;
        };
        let (mut sink, mut stream) = ws.split();
        let mut ok = true;
        for t in &topics {
            ok &= sink.send(Message::text(p.subscribe(t))).await.is_ok();
        }
        let mut deadline = Instant::now() + p.idle();
        let mut heard = false;
        while ok {
            tokio::select! {
                msg = stream.next() => match msg {
                    Some(Ok(Message::Text(text))) => {
                        deadline = Instant::now() + p.idle();
                        if !heard {
                            heard = true;
                            backoff = Duration::ZERO;
                        }
                        if let Step::Reconnect = p.read(&text, &events) {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => deadline = Instant::now() + p.idle(),
                },
                c = cmds.recv() => {
                    let Some(c) = c else { return };
                    let line = match &c {
                        Cmd::Add(t) if !topics.contains(t) && topics.len() < p.limit() => Some(p.subscribe(t)),
                        Cmd::Remove(t) if topics.contains(t) => Some(p.unsubscribe(t)),
                        Cmd::Send(line) => Some(line.clone()),
                        _ => None,
                    };
                    apply(&mut topics, c, p.limit());
                    if let Some(line) = line {
                        ok = sink.send(Message::text(line)).await.is_ok();
                    }
                    if topics.is_empty() {
                        let _ = sink.send(Message::Close(None)).await;
                        break;
                    }
                }
                _ = sleep_until(deadline) => break,
            }
        }
        if !topics.is_empty() {
            backoff = next_backoff(backoff);
        }
    }
}

fn apply<T: Eq + Hash>(topics: &mut HashSet<T>, c: Cmd<T>, limit: usize) {
    match c {
        Cmd::Add(t) if topics.len() < limit => {
            topics.insert(t);
        }
        Cmd::Add(_) | Cmd::Send(_) => {}
        Cmd::Remove(t) => {
            topics.remove(&t);
        }
    }
}

/// 1-5 s at first (jittered, so clients don't all come back at once), then doubling.
fn next_backoff(prev: Duration) -> Duration {
    if prev.is_zero() {
        let jitter = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_millis() % 4000);
        Duration::from_millis(1000 + u64::from(jitter))
    } else {
        (prev * 2).min(MAX_BACKOFF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_to_a_cap() {
        let first = next_backoff(Duration::ZERO);
        assert!((1000..5000).contains(&(first.as_millis() as u64)));
        assert_eq!(next_backoff(Duration::from_secs(100)), MAX_BACKOFF);
    }

    #[test]
    fn topics_stop_at_the_limit() {
        let mut t = HashSet::new();
        apply(&mut t, Cmd::Add(1), 2);
        apply(&mut t, Cmd::Add(2), 2);
        apply(&mut t, Cmd::Add(3), 2);
        assert_eq!(t.len(), 2);
        apply(&mut t, Cmd::Remove(1), 2);
        assert!(!t.contains(&1));
    }
}
