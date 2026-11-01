//! V108-N26 Discord Rich Presence: "Watching <channel>" on your Discord profile while a
//! live stream plays. Off unless the setting is on and the build has a Discord app ID
//! (`DISCORD_CLIENT_ID`, read by build.rs). Talks to the Discord client over its named pipe
//! (`\\.\pipe\discord-ipc-0` to `-9`) with plain std I/O on one thread; no new crate.
//! Nothing here shows an error: a Discord that isn't running is retried every 20 s.

mod activity;
mod frame;
mod plan;

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
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

// ---- protocol, over any stream (the tests use a fake one) ----

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
fn hello<T: Read + Write>(t: &mut T, client_id: &str) -> io::Result<()> {
    t.write_all(&frame::encode(frame::HANDSHAKE, &handshake(client_id)))?;
    loop {
        let (op, body) = frame::read_frame(t)?;
        match op {
            frame::FRAME if json(&body)["evt"] == "READY" => return Ok(()),
            frame::FRAME | frame::PONG => {}
            frame::PING => t.write_all(&frame::encode(frame::PONG, &body))?,
            _ => return Err(io::Error::other("Discord refused")),
        }
    }
}

/// Sends one activity (or none) and reads until its answer, answering pings on the way.
fn send<T: Read + Write>(t: &mut T, pid: u32, nonce: &str, act: Option<Value>) -> io::Result<Reply> {
    t.write_all(&frame::encode(frame::FRAME, &set_activity(pid, nonce, act)))?;
    loop {
        let (op, body) = frame::read_frame(t)?;
        match op {
            frame::PING => t.write_all(&frame::encode(frame::PONG, &body))?,
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

struct Conn<T> {
    pipe: T,
    kind: Kind,
    nonce: u64,
}

fn connect(client_id: &str) -> Option<Conn<File>> {
    for n in 0..10 {
        let path = format!(r"\\.\pipe\discord-ipc-{n}");
        let Ok(mut pipe) = OpenOptions::new().read(true).write(true).open(path) else { continue };
        if hello(&mut pipe, client_id).is_ok() {
            return Some(Conn { pipe, kind: Kind::Watching, nonce: 0 });
        }
    }
    None
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Sends the activity. Watching is tried first; if Discord rejects it, Playing is used from then on.
fn deliver<T: Read + Write>(c: &mut Conn<T>, w: &Watch, since: u64) -> io::Result<()> {
    loop {
        c.nonce += 1;
        let nonce = format!("cyex-{}", c.nonce);
        let act = build_activity(w, since, c.kind);
        match send(&mut c.pipe, std::process::id(), &nonce, Some(act))? {
            Reply::Accepted => return Ok(()),
            Reply::Rejected if c.kind == Kind::Watching => c.kind = Kind::Playing,
            // Rejected either way: nothing more to try for this one.
            Reply::Rejected => return Ok(()),
        }
    }
}

/// The start time is when this channel was first wanted, not when Discord got it.
#[derive(Default)]
struct Start {
    login: Option<String>,
    at: u64,
}

impl Start {
    fn note(&mut self, w: Option<&Watch>) {
        match w {
            Some(w) if self.login.as_deref() != Some(&w.login) => {
                self.at = now_secs();
                self.login = Some(w.login.clone());
            }
            Some(_) => {}
            None => self.login = None,
        }
    }
}

fn run(rx: &mpsc::Receiver<Option<Watch>>, client_id: &str) {
    let mut plan = Plan::default();
    let mut conn: Option<Conn<File>> = None;
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
                        plan.connected();
                    }
                    None => plan.connect_failed(now),
                }
                continue;
            }
            Step::Disconnect => {
                conn = None;
                plan.disconnected();
                continue;
            }
            Step::Send(w) => {
                let ok = conn.as_mut().is_some_and(|c| deliver(c, &w, start.at).is_ok());
                if ok {
                    plan.sent(w, now);
                } else {
                    conn = None;
                    plan.lost(now);
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
            start.note(w.as_ref());
            plan.want(w);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    /// A pipe whose answers are written down beforehand.
    struct Fake {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl Fake {
        fn new(frames: &[(u32, Value)]) -> Self {
            let bytes = frames.iter().flat_map(|(op, v)| frame::encode(*op, v.to_string().as_bytes())).collect();
            Self { input: Cursor::new(bytes), output: Vec::new() }
        }

        /// The frames the app wrote, decoded.
        fn written(&self) -> Vec<(u32, Value)> {
            let mut c = Cursor::new(self.output.clone());
            let mut out = Vec::new();
            while let Ok((op, body)) = frame::read_frame(&mut c) {
                out.push((op, json(&body)));
            }
            out
        }
    }

    impl Read for Fake {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn handshake_waits_for_ready() {
        let mut f = Fake::new(&[(frame::PING, json!({})), (frame::FRAME, json!({ "cmd": "DISPATCH", "evt": "READY" }))]);
        hello(&mut f, "42").unwrap();
        let w = f.written();
        assert_eq!(w[0], (frame::HANDSHAKE, json!({ "v": 1, "client_id": "42" })));
        assert_eq!(w[1].0, frame::PONG);
    }

    #[test]
    fn handshake_fails_on_close() {
        let mut f = Fake::new(&[(frame::CLOSE, json!({ "code": 4000 }))]);
        assert!(hello(&mut f, "42").is_err());
        assert!(hello(&mut Fake::new(&[]), "42").is_err());
    }

    #[test]
    fn send_reads_the_matching_answer() {
        let mut f = Fake::new(&[
            (frame::FRAME, json!({ "nonce": "other", "evt": "ERROR" })),
            (frame::PING, json!({})),
            (frame::FRAME, json!({ "cmd": "SET_ACTIVITY", "nonce": "n1", "evt": null })),
        ]);
        assert_eq!(send(&mut f, 1, "n1", None).unwrap(), Reply::Accepted);
        let w = f.written();
        assert_eq!(w[0].1["cmd"], "SET_ACTIVITY");
        assert_eq!(w[1].0, frame::PONG);
    }

    #[test]
    fn send_sees_a_rejection_and_a_close() {
        let mut f = Fake::new(&[(frame::FRAME, json!({ "nonce": "n1", "evt": "ERROR", "data": { "code": 4000 } }))]);
        assert_eq!(send(&mut f, 1, "n1", None).unwrap(), Reply::Rejected);
        let mut g = Fake::new(&[(frame::CLOSE, json!({}))]);
        assert!(send(&mut g, 1, "n1", None).is_err());
    }

    #[test]
    fn watching_falls_back_to_playing() {
        let w = Watch { login: "a".into(), name: "A".into(), avatar: None };
        let mut c = Conn {
            pipe: Fake::new(&[
                (frame::FRAME, json!({ "nonce": "cyex-1", "evt": "ERROR" })),
                (frame::FRAME, json!({ "nonce": "cyex-2", "evt": null })),
            ]),
            kind: Kind::Watching,
            nonce: 0,
        };
        deliver(&mut c, &w, 5).unwrap();
        assert_eq!(c.kind, Kind::Playing);
        let sent = c.pipe.written();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].1["args"]["activity"]["type"], 3);
        assert_eq!(sent[1].1["args"]["activity"]["type"], 0);
        assert_eq!(sent[1].1["args"]["activity"]["timestamps"]["start"], 5);
    }
}
