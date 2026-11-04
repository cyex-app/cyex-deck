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
/// V107-A6 A connect (TCP, TLS, upgrade) that takes longer is given up and retried.
#[cfg(not(test))]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
#[cfg(test)]
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);

pub trait Protocol: Send + 'static {
    type Topic: Clone + Eq + Hash + Send + Sync + 'static;
    type Event: Send + 'static;

    fn url(&self) -> String;
    fn subscribe(&self, topic: &Self::Topic) -> String;
    fn unsubscribe(&self, topic: &Self::Topic) -> String;
    /// Most topics one connection may hold, counted by `cost`.
    fn limit(&self) -> usize;
    /// F18 What a topic counts toward `limit` in a `Pool`; some cost the server more.
    fn cost(&self, _topic: &Self::Topic) -> usize {
        1
    }
    /// Silence longer than this means the connection is dead.
    fn idle(&self) -> Duration;
    /// Reads one text frame.
    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Self::Event>) -> Step;
    /// Topics wait until `read` answers `Step::Ready`, e.g. until signed in.
    fn gated(&self) -> bool {
        false
    }
    /// Topics went out again on a connection after the first one: whatever happened while
    /// it was down was missed, so read it again. Runs on the socket's task; keep it short.
    fn resynced(&mut self) {}
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
    // A connection has had its topics sent before; the next one is a reconnect.
    let mut synced = false;
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
        // V107-A6 Time-boxed, and topics keep coming in while it runs.
        let connect = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(p.url()));
        tokio::pin!(connect);
        let attempt = loop {
            tokio::select! {
                r = &mut connect => break r.ok().and_then(Result::ok),
                c = cmds.recv() => match c {
                    Some(c) => {
                        apply(&mut topics, c, p.limit());
                        if topics.is_empty() {
                            break None;
                        }
                    }
                    None => return,
                },
            }
        };
        let Some((ws, _)) = attempt else {
            if !topics.is_empty() {
                backoff = next_backoff(backoff);
            }
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
            if ok {
                note_synced(&mut p, &mut synced);
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
                                if ok {
                                    note_synced(&mut p, &mut synced);
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

/// Topics are out on a connection: the first time is a plain connect, later ones tell the protocol.
fn note_synced<P: Protocol>(p: &mut P, synced: &mut bool) {
    if std::mem::replace(synced, true) {
        p.resynced();
    }
}

type Spawn = fn(Pin<Box<dyn Future<Output = ()> + Send>>);

/// Topics over as many connections as they need, each holding up to the protocol's
/// limit (by `cost`). A topic added twice needs removing twice.
pub struct Pool<P: Protocol> {
    make: Box<dyn Fn() -> P + Send + Sync>,
    events: mpsc::UnboundedSender<P::Event>,
    spawn: Spawn,
    limit: usize,
    inner: Mutex<PoolInner<P>>,
}

struct PoolInner<P: Protocol> {
    /// Asked what each topic costs.
    probe: P,
    /// Each connection and the cost of what it holds.
    conns: Vec<(Topics<P::Topic>, usize)>,
    /// Topic → (connection, how many asked for it).
    topics: HashMap<P::Topic, (usize, usize)>,
}

impl<P: Protocol> Pool<P> {
    /// `spawn` runs each connection's task, e.g. on the app's runtime.
    pub fn new(make: impl Fn() -> P + Send + Sync + 'static, events: mpsc::UnboundedSender<P::Event>, spawn: Spawn) -> Self {
        let probe = make();
        let limit = probe.limit();
        Self {
            make: Box::new(make),
            events,
            spawn,
            limit,
            inner: Mutex::new(PoolInner {
                probe,
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
        let cost = inner.probe.cost(&topic);
        let at = match inner.conns.iter().position(|(_, n)| n + cost <= self.limit) {
            Some(at) => at,
            None => {
                let (handle, task) = start((self.make)(), self.events.clone());
                (self.spawn)(Box::pin(task));
                inner.conns.push((handle, 0));
                inner.conns.len() - 1
            }
        };
        inner.conns[at].0.add(topic.clone());
        inner.conns[at].1 += cost;
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
        let cost = inner.probe.cost(topic);
        inner.conns[at].0.remove(topic.clone());
        inner.conns[at].1 = inner.conns[at].1.saturating_sub(cost);
    }

    pub fn has(&self, topic: &P::Topic) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.topics.contains_key(topic)
    }

    /// Nothing is followed (an idle pool can be dropped, which ends its connections).
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).topics.is_empty()
    }

    /// What each connection holds, by cost.
    pub fn loads(&self) -> Vec<usize> {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.conns.iter().map(|(_, n)| *n).collect()
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
        let conns = |p: &Pool<Dummy>| p.loads().len();
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

    struct Silent {
        url: String,
    }

    impl Protocol for Silent {
        type Topic = u32;
        type Event = ();
        fn url(&self) -> String {
            self.url.clone()
        }
        fn subscribe(&self, _: &u32) -> String {
            String::new()
        }
        fn unsubscribe(&self, _: &u32) -> String {
            String::new()
        }
        fn limit(&self) -> usize {
            10
        }
        fn idle(&self) -> Duration {
            Duration::from_secs(60)
        }
        fn read(&mut self, _: &str, _: &mpsc::UnboundedSender<()>) -> Step {
            Step::Continue
        }
    }

    struct Counting {
        url: String,
        resyncs: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl Protocol for Counting {
        type Topic = u32;
        type Event = ();
        fn url(&self) -> String {
            self.url.clone()
        }
        fn subscribe(&self, _: &u32) -> String {
            "sub".into()
        }
        fn unsubscribe(&self, _: &u32) -> String {
            String::new()
        }
        fn limit(&self) -> usize {
            10
        }
        fn idle(&self) -> Duration {
            Duration::from_secs(60)
        }
        fn read(&mut self, _: &str, _: &mpsc::UnboundedSender<()>) -> Step {
            Step::Continue
        }
        fn resynced(&mut self) {
            self.resyncs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn only_a_later_connect_resyncs() {
        let mut p = Counting {
            url: String::new(),
            resyncs: Default::default(),
        };
        let mut synced = false;
        note_synced(&mut p, &mut synced);
        assert_eq!(p.resyncs.load(std::sync::atomic::Ordering::SeqCst), 0);
        note_synced(&mut p, &mut synced);
        note_synced(&mut p, &mut synced);
        assert_eq!(p.resyncs.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    /// The first connect gives no signal; the one after a drop gives exactly one.
    #[tokio::test]
    async fn a_drop_and_reconnect_signals_once() {
        use std::sync::atomic::Ordering::SeqCst;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/", listener.local_addr().unwrap());
        let resyncs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (tx, _rx) = mpsc::unbounded_channel();
        let (topics, task) = start(
            Counting {
                url,
                resyncs: resyncs.clone(),
            },
            tx,
        );
        let task = tokio::spawn(task);
        topics.add(1);
        let wait = Duration::from_secs(10);
        let accept = || async {
            let (sock, _) = tokio::time::timeout(wait, listener.accept()).await.unwrap().unwrap();
            let mut ws = tokio_tungstenite::accept_async(sock).await.unwrap();
            let first = tokio::time::timeout(wait, ws.next()).await.unwrap().unwrap().unwrap();
            assert_eq!(first.into_text().unwrap().as_str(), "sub");
            ws
        };
        let first = accept().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(resyncs.load(SeqCst), 0, "a first connect isn't a resync");
        drop(first);
        // Backoff is 1-5 s, then the second connection.
        let second = accept().await;
        let until = Instant::now() + wait;
        while resyncs.load(SeqCst) == 0 && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(resyncs.load(SeqCst), 1);
        drop((second, topics));
        task.abort();
    }

    /// V107-A6 A server that accepts and never answers the upgrade gets a fresh try.
    #[tokio::test]
    async fn a_silent_server_is_given_up_and_retried() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/", listener.local_addr().unwrap());
        let (tx, _rx) = mpsc::unbounded_channel();
        let (topics, task) = start(Silent { url }, tx);
        let task = tokio::spawn(task);
        topics.add(1);
        let wait = Duration::from_secs(10);
        let (_first, _) = tokio::time::timeout(wait, listener.accept()).await.unwrap().unwrap();
        // Held open, never written to. Backoff is 1-5 s, then the second try.
        let second = tokio::time::timeout(wait, listener.accept()).await;
        assert!(second.is_ok_and(|r| r.is_ok()));
        drop(topics);
        task.abort();
    }
}
