//! One websocket that follows a set of topics: connects while there are any, sends them
//! all again after every reconnect, and retries with backoff. 7TV's EventAPI, BTTV's
//! socket and Twitch's Hermes run on it. `Pool` spreads topics over several connections.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Mutex;
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
    /// Topics wait until `read` answers `Step::Ready`, e.g. until signed in.
    fn gated(&self) -> bool {
        false
    }
}

pub enum Step {
    Continue,
    /// The server asked for a fresh connection.
    Reconnect,
    /// Send this line now, e.g. signing in after a welcome.
    Reply(String),
    /// Signed in: topics go out now.
    Ready,
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
        let gated = p.gated();
        let mut ready = !gated;
        let mut ok = true;
        if ready {
            for t in &topics {
                ok &= sink.send(Message::text(p.subscribe(t))).await.is_ok();
            }
        }
        let mut deadline = Instant::now() + p.idle();
        let mut heard = false;
        while ok {
            tokio::select! {
                msg = stream.next() => match msg {
                    Some(Ok(Message::Text(text))) => {
                        deadline = Instant::now() + p.idle();
                        if !heard && !gated {
                            backoff = Duration::ZERO;
                        }
                        heard = true;
                        match p.read(&text, &events) {
                            Step::Continue => {}
                            Step::Reconnect => break,
                            Step::Reply(line) => ok = sink.send(Message::text(line)).await.is_ok(),
                            Step::Ready if !ready => {
                                ready = true;
                                backoff = Duration::ZERO;
                                for t in &topics {
                                    ok &= sink.send(Message::text(p.subscribe(t))).await.is_ok();
                                }
                            }
                            Step::Ready => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => deadline = Instant::now() + p.idle(),
                },
                c = cmds.recv() => {
                    let Some(c) = c else { return };
                    let line = match &c {
                        Cmd::Add(t) if ready && !topics.contains(t) && topics.len() < p.limit() => Some(p.subscribe(t)),
                        Cmd::Remove(t) if ready && topics.contains(t) => Some(p.unsubscribe(t)),
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

type Spawn = fn(Pin<Box<dyn Future<Output = ()> + Send>>);

/// Topics over as many connections as they need, each holding up to the protocol's
/// limit. A topic added twice needs removing twice.
pub struct Pool<P: Protocol> {
    make: Box<dyn Fn() -> P + Send + Sync>,
    events: mpsc::UnboundedSender<P::Event>,
    spawn: Spawn,
    limit: usize,
    inner: Mutex<PoolInner<P::Topic>>,
}

struct PoolInner<T> {
    conns: Vec<(Topics<T>, usize)>,
    /// Topic → (connection, how many asked for it).
    topics: HashMap<T, (usize, usize)>,
}

impl<P: Protocol> Pool<P> {
    /// `spawn` runs each connection's task, e.g. on the app's runtime.
    pub fn new(make: impl Fn() -> P + Send + Sync + 'static, events: mpsc::UnboundedSender<P::Event>, spawn: Spawn) -> Self {
        let limit = make().limit();
        Self {
            make: Box::new(make),
            events,
            spawn,
            limit,
            inner: Mutex::new(PoolInner {
                conns: Vec::new(),
                topics: HashMap::new(),
            }),
        }
    }

    pub fn add(&self, topic: P::Topic) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(t) = inner.topics.get_mut(&topic) {
            t.1 += 1;
            return;
        }
        let at = match inner.conns.iter().position(|(_, n)| *n < self.limit) {
            Some(at) => at,
            None => {
                let (handle, task) = start((self.make)(), self.events.clone());
                (self.spawn)(Box::pin(task));
                inner.conns.push((handle, 0));
                inner.conns.len() - 1
            }
        };
        inner.conns[at].0.add(topic.clone());
        inner.conns[at].1 += 1;
        inner.topics.insert(topic, (at, 1));
    }

    pub fn remove(&self, topic: &P::Topic) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let Some(t) = inner.topics.get_mut(topic) else { return };
        t.1 -= 1;
        if t.1 > 0 {
            return;
        }
        let at = t.0;
        inner.topics.remove(topic);
        inner.conns[at].0.remove(topic.clone());
        inner.conns[at].1 -= 1;
    }

    pub fn has(&self, topic: &P::Topic) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.topics.contains_key(topic)
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

    struct Dummy;

    impl Protocol for Dummy {
        type Topic = u32;
        type Event = ();
        fn url(&self) -> String {
            String::new()
        }
        fn subscribe(&self, _: &u32) -> String {
            String::new()
        }
        fn unsubscribe(&self, _: &u32) -> String {
            String::new()
        }
        fn limit(&self) -> usize {
            2
        }
        fn idle(&self) -> Duration {
            Duration::from_secs(1)
        }
        fn read(&mut self, _: &str, _: &mpsc::UnboundedSender<()>) -> Step {
            Step::Continue
        }
    }

    fn no_spawn(_: Pin<Box<dyn Future<Output = ()> + Send>>) {}

    #[test]
    fn pool_spreads_topics_and_counts_them() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let pool = Pool::new(|| Dummy, tx, no_spawn);
        for t in [1, 2, 3] {
            pool.add(t);
        }
        let conns = |p: &Pool<Dummy>| p.inner.lock().unwrap().conns.len();
        assert_eq!(conns(&pool), 2);
        pool.add(1);
        pool.remove(&1);
        assert!(pool.has(&1));
        pool.remove(&1);
        assert!(!pool.has(&1));
        pool.add(4);
        assert_eq!(conns(&pool), 2);
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
