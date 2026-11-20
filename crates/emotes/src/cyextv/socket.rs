//! V112-05 cyex.tv's event socket (ADR 0005): channels and sets by id, `user:` and the cosmetics
//! feed for later. It runs as a `ws::Pool`: past 500 topics a second connection opens. Frames
//! are limited to 16 KB and 10 a second, so topics go out batched and paced. After a drop the
//! new connection `resume`s the old session; when that is refused (or there was nothing to
//! resume from) the owner gets [`Event::Resync`] and reads its sets again.

use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;

use super::events::{Ack, Event, Frame, parse};
use cyex_twitch::ws::{Protocol, Step};

const URL: &str = "wss://api.cyex.tv/v1/events";
/// The server's hard cap per connection (`max_subscriptions`).
const LIMIT: usize = 500;
/// Topics in one `subscribe` line (a topic is about 40 bytes; frames may be 16 KB).
const PER_LINE: usize = 100;
/// A line's size, kept well under the 16 KB frame limit.
const MAX_LINE: usize = 12 * 1024;
/// A frame about every 120 ms is under the server's 10 a second, with room for heartbeats.
const GAP: Duration = Duration::from_millis(120);
/// A longest id taken; real ids are 26 characters or Twitch's numbers.
const MAX_ID: usize = 64;

#[cfg(feature = "mocks")]
static MOCK: std::sync::OnceLock<String> = std::sync::OnceLock::new();
/// Test runs with a local mock socket (`CYEX_CYEXTV_WS`): every connection goes there.
#[cfg(feature = "mocks")]
pub fn use_mock(url: &str) {
    let _ = MOCK.set(url.to_owned());
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Topic {
    /// `channel:twitch:<id>`: active set switches and personal changes of its chatters.
    Channel(String),
    /// `set:<id>`: emotes changing in one set.
    Set(String),
    /// `user:twitch:<id>`: one person's cosmetics and personal set (not used yet).
    User(String),
    /// `cosmetics`: new and changed badges and paints (not used yet).
    Cosmetics,
}

impl Topic {
    /// The name on the wire; `None` for an id the server would refuse.
    pub fn name(&self) -> Option<String> {
        let ok = |id: &str| {
            !id.is_empty()
                && id.len() <= MAX_ID
                && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        };
        match self {
            Topic::Channel(id) if ok(id) => Some(format!("channel:twitch:{id}")),
            Topic::Set(id) if ok(id) => Some(format!("set:{id}")),
            Topic::User(id) if ok(id) => Some(format!("user:twitch:{id}")),
            Topic::Cosmetics => Some("cosmetics".to_owned()),
            _ => None,
        }
    }
}

pub struct EventApi {
    /// The server's id for the connection that is open, or the one we resume from.
    session: Option<String>,
    /// The newest outbox id seen.
    last_event: Option<u64>,
    /// The session of this connection, until a `resume` says whether it is kept.
    fresh: Option<String>,
    /// Topics were sent on a connection with nothing to resume: reload what changed meanwhile.
    resync: bool,
    out: Option<mpsc::UnboundedSender<Event>>,
    idle: Duration,
}

impl EventApi {
    pub fn new() -> Self {
        Self {
            session: None,
            last_event: None,
            fresh: None,
            resync: false,
            out: None,
            idle: Duration::from_secs(60),
        }
    }
}

impl Default for EventApi {
    fn default() -> Self {
        Self::new()
    }
}

/// `subscribe` or `unsubscribe` lines for `topics`, at most [`PER_LINE`] topics and [`MAX_LINE`]
/// bytes each; topics the server would refuse are left out.
fn lines(op: &str, topics: &[&Topic]) -> Vec<String> {
    let mut out = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut size = 0;
    let mut flush = |names: &mut Vec<String>, size: &mut usize| {
        if !names.is_empty() {
            out.push(json!({ "op": op, "d": { "topics": std::mem::take(names) } }).to_string());
        }
        *size = 0;
    };
    for name in topics.iter().filter_map(|t| t.name()) {
        if names.len() >= PER_LINE || size + name.len() + 4 > MAX_LINE {
            flush(&mut names, &mut size);
        }
        size += name.len() + 4;
        names.push(name);
    }
    flush(&mut names, &mut size);
    out
}

impl Protocol for EventApi {
    type Topic = Topic;
    type Event = Event;

    fn label(&self) -> &'static str {
        "cyex.tv"
    }

    fn url(&self) -> String {
        #[cfg(feature = "mocks")]
        if let Some(url) = MOCK.get() {
            return url.clone();
        }
        URL.to_owned()
    }

    fn subscribe(&self, topic: &Topic) -> String {
        lines("subscribe", &[topic]).into_iter().next().unwrap_or_default()
    }

    fn unsubscribe(&self, topic: &Topic) -> String {
        lines("unsubscribe", &[topic]).into_iter().next().unwrap_or_default()
    }

    fn subscribe_many(&self, topics: &[&Topic]) -> Vec<String> {
        lines("subscribe", topics)
    }

    fn unsubscribe_many(&self, topics: &[&Topic]) -> Vec<String> {
        lines("unsubscribe", topics)
    }

    fn frame_gap(&self) -> Duration {
        GAP
    }

    fn limit(&self) -> usize {
        LIMIT
    }

    fn idle(&self) -> Duration {
        self.idle
    }

    /// Topics wait for `hello` (and the `resume` after it, when there is a session to take over).
    fn gated(&self) -> bool {
        true
    }

    fn resynced(&mut self) {
        if std::mem::take(&mut self.resync)
            && let Some(out) = &self.out
        {
            let _ = out.send(Event::Resync);
        }
    }

    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Event>) -> Step {
        if self.out.is_none() {
            self.out = Some(out.clone());
        }
        match parse(text) {
            Frame::Hello { session_id, heartbeat_ms } => {
                self.idle = Duration::from_millis(heartbeat_ms.clamp(5_000, 60_000) * 3);
                match (self.session.clone(), self.last_event) {
                    (Some(old), Some(last)) => {
                        self.fresh = Some(session_id);
                        let line = json!({ "op": "resume", "d": { "session_id": old, "last_event_id": last } });
                        return Step::Reply(line.to_string());
                    }
                    (old, _) => {
                        // A first connection has nothing to catch up on; a later one has no place
                        // to resume from, so what changed meanwhile is read again.
                        self.resync |= old.is_some();
                        self.session = Some(session_id);
                        return Step::Ready;
                    }
                }
            }
            Frame::Ack(Ack { op, error, .. }) if op == "resume" => {
                let fresh = self.fresh.take();
                match error {
                    // The old session is ours again, with its place in the outbox.
                    None => {}
                    Some(why) => {
                        eprintln!("cyex.tv: resume refused ({why})");
                        self.session = fresh;
                        self.last_event = None;
                        self.resync = true;
                    }
                }
                return Step::Ready;
            }
            Frame::Ack(Ack { refused, .. }) => {
                for (topic, why) in refused {
                    eprintln!("cyex.tv: {topic} refused ({why})");
                }
            }
            Frame::Dispatch { id, event } => {
                if let Some(id) = id {
                    self.last_event = Some(self.last_event.map_or(id, |l| l.max(id)));
                }
                if let Some(event) = event {
                    let _ = out.send(event);
                }
            }
            Frame::Heartbeat => return Step::Reply(r#"{"op":"heartbeat","d":{}}"#.to_owned()),
            Frame::Reconnect => return Step::Reconnect,
            Frame::Error(code) => {
                eprintln!("cyex.tv: {code}");
                return Step::Reconnect;
            }
            Frame::Other => {}
        }
        Step::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_twitch::ws::Pool;
    use serde_json::Value;

    fn topics_of(line: &str) -> Vec<String> {
        let v: Value = serde_json::from_str(line).unwrap();
        v["d"]["topics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_owned())
            .collect()
    }

    fn read(api: &mut EventApi, text: &str) -> (Step, Vec<Event>) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let step = api.read(text, &tx);
        let mut got = Vec::new();
        while let Ok(e) = rx.try_recv() {
            got.push(e);
        }
        (step, got)
    }

    fn hello(id: &str) -> String {
        format!(r#"{{"op":"hello","d":{{"session_id":"{id}","heartbeat_interval":20000,"max_subscriptions":500}}}}"#)
    }

    fn dispatch(id: u64) -> String {
        format!(r#"{{"op":"dispatch","d":{{"id":"{id}","type":"set.emote_added","data":{{"set_id":"S"}}}}}}"#)
    }

    #[test]
    fn topic_names() {
        assert_eq!(Topic::Channel("71092938".into()).name().unwrap(), "channel:twitch:71092938");
        assert_eq!(Topic::Set("01HX".into()).name().unwrap(), "set:01HX");
        assert_eq!(Topic::User("5".into()).name().unwrap(), "user:twitch:5");
        assert_eq!(Topic::Cosmetics.name().unwrap(), "cosmetics");
        // The server would refuse these; they never go out.
        assert_eq!(Topic::Set(String::new()).name(), None);
        assert_eq!(Topic::Set("a b".into()).name(), None);
        assert_eq!(Topic::Channel("x:y".into()).name(), None);
        assert_eq!(Topic::Set("a".repeat(65)).name(), None);
    }

    #[test]
    fn many_topics_are_batched_and_stay_small() {
        let api = EventApi::new();
        let ts: Vec<Topic> = (0..500).map(|i| Topic::Set(format!("{i:026}"))).collect();
        let refs: Vec<&Topic> = ts.iter().collect();
        let sent = api.subscribe_many(&refs);
        assert_eq!(sent.len(), 5, "100 to a line");
        assert!(sent.iter().all(|l| l.len() < 16 * 1024));
        let all: Vec<String> = sent.iter().flat_map(|l| topics_of(l)).collect();
        assert_eq!(all.len(), 500);
        assert!(sent[0].contains(r#""op":"subscribe""#));
        // Even the longest ids fit a line (the size limit is a second guard behind the count).
        let long: Vec<Topic> = (0..250).map(|i| Topic::Set(format!("{i:064}"))).collect();
        let refs: Vec<&Topic> = long.iter().collect();
        let sent = api.unsubscribe_many(&refs);
        assert_eq!(sent.len(), 3);
        assert!(sent.iter().all(|l| l.len() <= MAX_LINE && l.contains("unsubscribe")));
        assert_eq!(sent.iter().map(|l| topics_of(l).len()).sum::<usize>(), 250);
        // Refused topics are dropped; none is an empty line.
        let bad = Topic::Set("no good".into());
        assert!(api.subscribe_many(&[&bad]).is_empty());
        assert_eq!(api.subscribe(&bad), "");
    }

    #[test]
    fn a_full_connection_opens_another() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let pool = Pool::new(EventApi::new, tx, |_| {});
        for i in 0..500 {
            pool.add(Topic::Set(format!("s{i}")));
        }
        assert_eq!(pool.loads(), vec![500]);
        pool.add(Topic::Channel("1".into()));
        assert_eq!(pool.loads(), vec![500, 1], "the 501st goes to a second socket");
        // Added twice, counted once; it leaves with the second removal.
        pool.add(Topic::Channel("1".into()));
        assert_eq!(pool.loads(), vec![500, 1]);
        pool.remove(&Topic::Channel("1".into()));
        assert_eq!(pool.loads(), vec![500, 1]);
        pool.remove(&Topic::Channel("1".into()));
        assert_eq!(pool.loads(), vec![500, 0]);
    }

    #[test]
    fn a_first_hello_just_opens_the_topics() {
        let mut api = EventApi::new();
        let (step, got) = read(&mut api, &hello("S1"));
        assert!(matches!(step, Step::Ready));
        assert!(got.is_empty());
        assert_eq!(api.idle(), Duration::from_secs(60), "three missed beats");
        assert!(api.gated());
        api.resynced();
        assert!(read(&mut api, r#"{"op":"heartbeat","d":{}}"#).1.is_empty(), "nothing to reload");
    }

    #[test]
    fn heartbeats_are_answered() {
        let mut api = EventApi::new();
        let (step, _) = read(&mut api, r#"{"op":"heartbeat","d":{}}"#);
        assert!(matches!(step, Step::Reply(l) if l.contains("heartbeat")));
    }

    #[test]
    fn a_reconnect_resumes_after_the_last_event() {
        let mut api = EventApi::new();
        read(&mut api, &hello("S1"));
        let (_, got) = read(&mut api, &dispatch(41));
        assert_eq!(got.len(), 1);
        read(&mut api, &dispatch(40));
        // The connection drops; the next hello resumes S1 after 41.
        let (step, _) = read(&mut api, &hello("S2"));
        let Step::Reply(line) = step else { panic!("resume expected") };
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["op"], "resume");
        assert_eq!(v["d"]["session_id"], "S1");
        assert_eq!(v["d"]["last_event_id"], 41);
        // Accepted: topics go out, and nothing is reloaded.
        let (step, _) = read(&mut api, r#"{"op":"ack","d":{"op":"resume","session_id":"S1","topics":[]}}"#);
        assert!(matches!(step, Step::Ready));
        api.resynced();
        // The session is S1 still, with its place.
        let (step, _) = read(&mut api, &hello("S3"));
        let Step::Reply(line) = step else { panic!("resume expected") };
        assert!(line.contains(r#""session_id":"S1""#) && line.contains("41"));
    }

    #[test]
    fn a_refused_resume_reloads_the_sets() {
        let mut api = EventApi::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        api.read(&hello("S1"), &tx);
        api.read(&dispatch(9), &tx);
        let _ = rx.try_recv();
        api.read(&hello("S2"), &tx);
        let (step, _) = {
            let s = api.read(r#"{"op":"ack","d":{"op":"resume","error":"unknown_session"}}"#, &tx);
            (s, ())
        };
        assert!(matches!(step, Step::Ready));
        api.resynced();
        assert_eq!(rx.try_recv().unwrap(), Event::Resync);
        // The fresh session is the one to resume next, and from nowhere yet.
        api.read(&hello("S3"), &tx);
        api.resynced();
        assert_eq!(rx.try_recv().unwrap(), Event::Resync, "no place to resume from");
    }

    #[test]
    fn a_resync_from_the_server_reaches_the_owner() {
        let mut api = EventApi::new();
        read(&mut api, &hello("S1"));
        let (_, got) = read(&mut api, r#"{"op":"dispatch","d":{"id":"0","type":"resync","data":{}}}"#);
        assert_eq!(got, vec![Event::Resync]);
        // A made-up event doesn't move the place to resume from.
        assert_eq!(api.last_event, None);
    }

    #[test]
    fn reconnect_and_errors_end_the_connection() {
        let mut api = EventApi::new();
        assert!(matches!(read(&mut api, r#"{"op":"reconnect","d":{}}"#).0, Step::Reconnect));
        assert!(matches!(
            read(&mut api, r#"{"op":"error","d":{"code":"timeout"}}"#).0,
            Step::Reconnect
        ));
        // Unknown ops and dispatch types are ignored.
        assert!(matches!(read(&mut api, r#"{"op":"new","d":{}}"#).0, Step::Continue));
        let (step, got) = read(&mut api, r#"{"op":"dispatch","d":{"id":"5","type":"cosmetic.created","data":{}}}"#);
        assert!(matches!(step, Step::Continue) && got.is_empty());
    }
}
