//! V108-N26 Discord Rich Presence: "Watching <channel>" on your Discord profile while a
//! live stream plays. Off unless the setting is on and the build has a Discord app ID
//! (`DISCORD_CLIENT_ID`, read by build.rs). Talks to the Discord client over its named pipe
//! (`\\.\pipe\discord-ipc-0` to `-9`) with plain std I/O; no new crate. The pipe lives on its
//! own thread so a Discord that stops answering can't hang the worker.
//! Nothing here shows an error: a Discord that isn't running is retried every 20 s.

mod activity;
mod frame;
mod plan;

use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

pub use activity::Watch;
use activity::{Kind, activity as build_activity, handshake, set_activity};
use plan::{Plan, Step};

/// The Discord app's ID, from the build (public, like the Twitch client IDs; kept out of the code).
const CLIENT_ID: Option<&str> = match option_env!("DISCORD_CLIENT_ID") {
    Some(s) if !s.is_empty() => Some(s),
    _ => None,
};

/// How long Discord gets for one read or write before the pipe counts as lost.
const ANSWER: Duration = Duration::from_secs(5);
/// A channel cleared for less than this keeps its start time when it comes back.
const KEEP_START: Duration = Duration::from_secs(5 * 60);

/// Whether this build can talk to Discord at all.
pub fn available() -> bool {
    CLIENT_ID.is_some()
}

static WORKER: Mutex<Option<Sender<Option<Watch>>>> = Mutex::new(None);

/// What the window plays now, or `None` for nothing (or the setting off). Cheap to call often;
/// the worker coalesces and spaces what reaches Discord.
pub fn watch(w: Option<Watch>) {
    let Some(id) = CLIENT_ID else { return };
    let mut slot = WORKER.lock().unwrap_or_else(|e| e.into_inner());
    if slot.is_none() {
        if w.is_none() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        if std::thread::Builder::new().name("discord-presence".into()).spawn(move || run(&rx, id)).is_err() {
            return;
        }
        *slot = Some(tx);
    }
    if let Some(tx) = slot.as_ref() {
        let _ = tx.send(w);
    }
}

// ---- the pipe, on its own thread ----

/// Pipe threads inside a read or write right now. One left behind by a Discord that stopped
/// answering holds the count up; no new pipe is opened until it ends, so at most one waits.
static BUSY: AtomicUsize = AtomicUsize::new(0);

enum Op {
    Write(Vec<u8>),
    Read,
}

type Done = io::Result<Option<(u32, Vec<u8>)>>;

/// Runs one read or write at a time on its thread and waits `timeout` for each. Between
/// ops the thread waits for the next one, so dropping this ends it (and closes the pipe,
/// which clears the activity). A thread stuck in an op ends when Discord answers or closes.
struct Pipe {
    ops: Sender<Op>,
    done: Receiver<Done>,
    timeout: Duration,
    broken: bool,
}

impl Pipe {
    fn spawn<T: Read + Write + Send + 'static>(mut t: T, timeout: Duration) -> io::Result<Self> {
        let (ops, op_rx) = mpsc::channel::<Op>();
        let (done_tx, done) = mpsc::channel::<Done>();
        std::thread::Builder::new().name("discord-pipe".into()).spawn(move || {
            for op in op_rx {
                BUSY.fetch_add(1, Ordering::SeqCst);
                let r = match op {
                    Op::Write(bytes) => t.write_all(&bytes).and_then(|()| t.flush()).map(|()| None),
                    Op::Read => frame::read_frame(&mut t).map(Some),
                };
                BUSY.fetch_sub(1, Ordering::SeqCst);
                if done_tx.send(r).is_err() {
                    break;
                }
            }
        })?;
        Ok(Self { ops, done, timeout, broken: false })
    }

    /// After any failure (a timeout above all, whose answer may still come) the pipe is done.
    fn run(&mut self, op: Op) -> Done {
        if self.broken {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let r = match self.ops.send(op) {
            Err(_) => Err(io::ErrorKind::BrokenPipe.into()),
            Ok(()) => match self.done.recv_timeout(self.timeout) {
                Ok(r) => r,
                Err(RecvTimeoutError::Timeout) => Err(io::Error::new(io::ErrorKind::TimedOut, "Discord didn't answer")),
                Err(RecvTimeoutError::Disconnected) => Err(io::ErrorKind::BrokenPipe.into()),
            },
        };
        self.broken |= r.is_err();
        r
    }

    fn write(&mut self, bytes: Vec<u8>) -> io::Result<()> {
        self.run(Op::Write(bytes)).map(|_| ())
    }

    fn read(&mut self) -> io::Result<(u32, Vec<u8>)> {
        self.run(Op::Read)?.ok_or_else(|| io::Error::other("no frame"))
    }
}

// ---- protocol ----

/// Discord's answer to a SET_ACTIVITY.
#[derive(Debug, PartialEq, Eq)]
enum Reply {
    Accepted,
    /// An error event, e.g. an activity type it doesn't know.
    Rejected,
}

fn json(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

/// Handshake, then waits for READY. Anything but READY (a close frame, an error) fails.
fn hello(p: &mut Pipe, client_id: &str) -> io::Result<()> {
    p.write(frame::encode(frame::HANDSHAKE, &handshake(client_id)))?;
    loop {
        let (op, body) = p.read()?;
        match op {
            frame::FRAME if json(&body)["evt"] == "READY" => return Ok(()),
            frame::FRAME | frame::PONG => {}
            frame::PING => p.write(frame::encode(frame::PONG, &body))?,
            _ => return Err(io::Error::other("Discord refused")),
        }
    }
}

/// Sends one activity (or none) and reads until its answer, answering pings on the way.
fn send(p: &mut Pipe, pid: u32, nonce: &str, act: Option<Value>) -> io::Result<Reply> {
    p.write(frame::encode(frame::FRAME, &set_activity(pid, nonce, act)))?;
    loop {
        let (op, body) = p.read()?;
        match op {
            frame::PING => p.write(frame::encode(frame::PONG, &body))?,
            frame::FRAME => {
                let v = json(&body);
                if v["nonce"] == nonce {
                    return Ok(if v["evt"] == "ERROR" { Reply::Rejected } else { Reply::Accepted });
                }
            }
            frame::CLOSE => return Err(io::Error::other("Discord closed")),
            _ => {}
        }
    }
}

// ---- the worker ----

struct Conn {
    pipe: Pipe,
    kind: Kind,
    nonce: u64,
}

fn connect(client_id: &str) -> Option<Conn> {
    if BUSY.load(Ordering::SeqCst) > 0 {
        return None;
    }
    for n in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{n}");
        let Ok(file) = OpenOptions::new().read(true).write(true).open(path) else { continue };
        let Ok(mut pipe) = Pipe::spawn(file, ANSWER) else { continue };
        if hello(&mut pipe, client_id).is_ok() {
            return Some(Conn { pipe, kind: Kind::Watching, nonce: 0 });
        }
    }
    None
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Sends the activity, or clears it for `None`. Watching is tried first; if Discord rejects
/// it, Playing is used from then on.
fn deliver(c: &mut Conn, w: Option<&Watch>, since: u64) -> io::Result<()> {
    loop {
        c.nonce += 1;
        let nonce = format!("cyex-{}", c.nonce);
        let act = w.map(|w| build_activity(w, since, c.kind));
        match send(&mut c.pipe, std::process::id(), &nonce, act)? {
            Reply::Accepted => return Ok(()),
            Reply::Rejected if w.is_some() && c.kind == Kind::Watching => c.kind = Kind::Playing,
            // Rejected either way: nothing more to try for this one.
            Reply::Rejected => return Ok(()),
        }
    }
}

/// The start time is when this channel was first wanted, not when Discord got it. A short
/// clear (a reload, a moment off air) keeps it.
#[derive(Default)]
struct Start {
    login: Option<String>,
    at: u64,
    cleared: Option<Instant>,
}

impl Start {
    fn note(&mut self, w: Option<&Watch>, now: Instant, secs: u64) {
        match w {
            None => {
                self.cleared.get_or_insert(now);
            }
            Some(w) => {
                let long_gone = self.cleared.is_some_and(|t| now.saturating_duration_since(t) > KEEP_START);
                if long_gone || self.login.as_deref() != Some(&w.login) {
                    self.at = secs;
                    self.login = Some(w.login.clone());
                }
                self.cleared = None;
            }
        }
    }
}

fn run(rx: &mpsc::Receiver<Option<Watch>>, client_id: &str) {
    let mut plan = Plan::default();
    let mut conn: Option<Conn> = None;
    let mut start = Start::default();
    loop {
        let now = Instant::now();
        let wait: Option<Duration> = match plan.step(now) {
            Step::Idle => None,
            Step::Wait(d) => Some(d),
            Step::Connect => {
                match connect(client_id) {
                    Some(c) => {
                        conn = Some(c);
                        plan.connected(Instant::now());
                    }
                    None => plan.connect_failed(Instant::now()),
                }
                continue;
            }
            Step::Disconnect => {
                conn = None;
                plan.disconnected();
                continue;
            }
            Step::Send(w) => {
                let ok = conn.as_mut().is_some_and(|c| deliver(c, w.as_ref(), start.at).is_ok());
                if ok {
                    plan.sent(w, Instant::now());
                } else {
                    conn = None;
                    plan.lost(Instant::now());
                }
                continue;
            }
        };
        let got = match wait {
            None => rx.recv().map_err(|_| ()),
            Some(d) => match rx.recv_timeout(d) {
                Ok(m) => Ok(m),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => Err(()),
            },
        };
        let Ok(first) = got else { return };
        for w in std::iter::once(first).chain(rx.try_iter()) {
            start.note(w.as_ref(), Instant::now(), now_secs());
            plan.want(w);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;
    use std::sync::Arc;

    /// A pipe whose answers are written down beforehand; reads can be made slow.
    struct Fake {
        input: Cursor<Vec<u8>>,
        output: Arc<Mutex<Vec<u8>>>,
        delay: Duration,
    }

    /// The frames the app wrote, decoded.
    fn written(out: &Mutex<Vec<u8>>) -> Vec<(u32, Value)> {
        let mut c = Cursor::new(out.lock().unwrap().clone());
        let mut frames = Vec::new();
        while let Ok((op, body)) = frame::read_frame(&mut c) {
            frames.push((op, json(&body)));
        }
        frames
    }

    /// A pipe thread over a fake with these answers, and what it gets written.
    fn fake(frames: &[(u32, Value)]) -> (Pipe, Arc<Mutex<Vec<u8>>>) {
        slow(frames, Duration::ZERO, Duration::from_secs(5))
    }

    fn slow(frames: &[(u32, Value)], delay: Duration, timeout: Duration) -> (Pipe, Arc<Mutex<Vec<u8>>>) {
        let bytes = frames.iter().flat_map(|(op, v)| frame::encode(*op, v.to_string().as_bytes())).collect();
        let output = Arc::new(Mutex::new(Vec::new()));
        let f = Fake { input: Cursor::new(bytes), output: Arc::clone(&output), delay };
        (Pipe::spawn(f, timeout).unwrap(), output)
    }

    impl Read for Fake {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            std::thread::sleep(self.delay);
            self.input.read(buf)
        }
    }

    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn conn(pipe: Pipe) -> Conn {
        Conn { pipe, kind: Kind::Watching, nonce: 0 }
    }

    fn a() -> Watch {
        Watch { login: "a".into(), name: "A".into(), avatar: None }
    }

    #[test]
    fn handshake_waits_for_ready() {
        let (mut p, out) = fake(&[(frame::PING, json!({})), (frame::FRAME, json!({ "cmd": "DISPATCH", "evt": "READY" }))]);
        hello(&mut p, "42").unwrap();
        let w = written(&out);
        assert_eq!(w[0], (frame::HANDSHAKE, json!({ "v": 1, "client_id": "42" })));
        assert_eq!(w[1].0, frame::PONG);
    }

    #[test]
    fn handshake_fails_on_close() {
        let (mut p, _) = fake(&[(frame::CLOSE, json!({ "code": 4000 }))]);
        assert!(hello(&mut p, "42").is_err());
        assert!(hello(&mut fake(&[]).0, "42").is_err());
    }

    #[test]
    fn send_reads_the_matching_answer() {
        let (mut p, out) = fake(&[
            (frame::FRAME, json!({ "nonce": "other", "evt": "ERROR" })),
            (frame::PING, json!({})),
            (frame::FRAME, json!({ "cmd": "SET_ACTIVITY", "nonce": "n1", "evt": null })),
        ]);
        assert_eq!(send(&mut p, 1, "n1", None).unwrap(), Reply::Accepted);
        let w = written(&out);
        assert_eq!(w[0].1["cmd"], "SET_ACTIVITY");
        assert_eq!(w[1].0, frame::PONG);
    }

    #[test]
    fn send_sees_a_rejection_and_a_close() {
        let (mut p, _) = fake(&[(frame::FRAME, json!({ "nonce": "n1", "evt": "ERROR", "data": { "code": 4000 } }))]);
        assert_eq!(send(&mut p, 1, "n1", None).unwrap(), Reply::Rejected);
        let (mut g, _) = fake(&[(frame::CLOSE, json!({}))]);
        assert!(send(&mut g, 1, "n1", None).is_err());
    }

    #[test]
    fn watching_falls_back_to_playing() {
        let (p, out) = fake(&[
            (frame::FRAME, json!({ "nonce": "cyex-1", "evt": "ERROR" })),
            (frame::FRAME, json!({ "nonce": "cyex-2", "evt": null })),
        ]);
        let mut c = conn(p);
        deliver(&mut c, Some(&a()), 5).unwrap();
        assert_eq!(c.kind, Kind::Playing);
        let sent = written(&out);
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].1["args"]["activity"]["type"], 3);
        assert_eq!(sent[1].1["args"]["activity"]["type"], 0);
        assert_eq!(sent[1].1["args"]["activity"]["timestamps"]["start"], 5);
    }

    #[test]
    fn rejected_both_ways_gives_up_for_this_one() {
        let (p, out) = fake(&[
            (frame::FRAME, json!({ "nonce": "cyex-1", "evt": "ERROR" })),
            (frame::FRAME, json!({ "nonce": "cyex-2", "evt": "ERROR" })),
            (frame::FRAME, json!({ "nonce": "cyex-3", "evt": null })),
        ]);
        let mut c = conn(p);
        deliver(&mut c, Some(&a()), 5).unwrap();
        assert_eq!(c.kind, Kind::Playing);
        assert_eq!(written(&out).len(), 2);
        // Playing stays; the next one goes out as Playing at once.
        deliver(&mut c, Some(&a()), 5).unwrap();
        let sent = written(&out);
        assert_eq!(sent.len(), 3);
        assert_eq!(sent[2].1["args"]["activity"]["type"], 0);
    }

    #[test]
    fn a_clear_is_sent_once_even_when_rejected() {
        let (p, out) = fake(&[(frame::FRAME, json!({ "nonce": "cyex-1", "evt": "ERROR" }))]);
        let mut c = conn(p);
        deliver(&mut c, None, 5).unwrap();
        assert_eq!(c.kind, Kind::Watching);
        let sent = written(&out);
        assert_eq!(sent.len(), 1);
        assert!(sent[0].1["args"]["activity"].is_null());
    }

    #[test]
    fn a_silent_discord_times_out_and_the_pipe_stays_broken() {
        let (p, _) = slow(&[(frame::FRAME, json!({ "nonce": "cyex-1", "evt": null }))], Duration::from_millis(300), Duration::from_millis(30));
        let mut c = conn(p);
        let e = deliver(&mut c, Some(&a()), 5).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::TimedOut);
        // The late answer doesn't count as the next one's.
        std::thread::sleep(Duration::from_millis(100));
        assert!(deliver(&mut c, Some(&a()), 5).is_err());
    }

    #[test]
    fn the_pipe_thread_ends_when_dropped() {
        let (p, out) = fake(&[]);
        assert_eq!(Arc::strong_count(&out), 2);
        drop(p);
        // The thread drops the fake (the pipe) on its way out.
        let t0 = Instant::now();
        while Arc::strong_count(&out) > 1 {
            assert!(t0.elapsed() < Duration::from_secs(5), "pipe thread still running");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_short_clear_keeps_the_start_time() {
        let t0 = Instant::now();
        let mut s = Start::default();
        s.note(Some(&a()), t0, 100);
        s.note(None, t0 + Duration::from_secs(10), 110);
        s.note(Some(&a()), t0 + Duration::from_secs(70), 170);
        assert_eq!(s.at, 100);
        s.note(None, t0 + Duration::from_secs(80), 180);
        s.note(Some(&a()), t0 + Duration::from_secs(80) + KEEP_START + Duration::from_secs(1), 500);
        assert_eq!(s.at, 500);
        let b = Watch { login: "b".into(), name: "B".into(), avatar: None };
        s.note(Some(&b), t0 + Duration::from_secs(900), 900);
        assert_eq!(s.at, 900);
    }
}
