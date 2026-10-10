//! Hermes, Twitch's websocket for the old PubSub topics (`docs/research/twitch-api.md`).
//! Channel topics are read anonymously: polls and predictions (C4), celebrations (1.42),
//! pins and redemptions (1.9), and what the miner follows (08). User topics (points,
//! your bets, whispers and the bell) need a signed-in connection: welcome, then `authenticate`, then topics.
//! Subscription ids encode their topic, so a notification names its channel.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::clients;
use crate::clock::now_iso;
use crate::ws::{Protocol, Step};

/// Twitch asks for a keepalive every 15 s; two missed ones end the connection.
const IDLE: Duration = Duration::from_secs(35);
/// PubSub's old limit per connection; Hermes took 75 in a test (2026/10/10). `ws::Pool`
/// opens more connections past it.
const LIMIT: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Polls,
    Predictions,
    Celebrations,
    /// `pin-message`, `update-message`, `unpin-message`.
    Pins,
    /// Redemptions, and community goals (W2).
    Points,
    /// `stream-up`, `stream-down`, `viewcount`.
    Playback,
    Raid,
    Moments,
    /// The user's points: earned, spent, bonus chest ready. Signed in only.
    UserPoints,
    /// The user's bets and their results. Signed in only.
    UserPredictions,
    /// 1.26 The user's whispers, sent and received, and thread changes. Signed in only.
    Whispers,
    /// 1.26 The user's bell (onsite notifications). Signed in only.
    Onsite,
}

const KINDS: [Kind; 12] = [
    Kind::Polls,
    Kind::Predictions,
    Kind::Celebrations,
    Kind::Pins,
    Kind::Points,
    Kind::Playback,
    Kind::Raid,
    Kind::Moments,
    Kind::UserPoints,
    Kind::UserPredictions,
    Kind::Whispers,
    Kind::Onsite,
];

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Kind::Polls => "polls.",
            Kind::Predictions => "predictions-channel-v1.",
            Kind::Celebrations => "bits-rewards-celebration-v1.",
            Kind::Pins => "pinned-chat-updates-v1.",
            Kind::Points => "community-points-channel-v1.",
            Kind::Playback => "video-playback-by-id.",
            Kind::Raid => "raid.",
            Kind::Moments => "community-moments-channel-v1.",
            Kind::UserPoints => "community-points-user-v1.",
            Kind::UserPredictions => "predictions-user-v1.",
            Kind::Whispers => "whispers.",
            Kind::Onsite => "onsite-notifications.",
        }
    }

    fn code(self) -> char {
        match self {
            Kind::Polls => 'p',
            Kind::Predictions => 'r',
            Kind::Celebrations => 'c',
            Kind::Pins => 'n',
            Kind::Points => 'd',
            Kind::Playback => 'v',
            Kind::Raid => 'a',
            Kind::Moments => 'm',
            Kind::UserPoints => 'u',
            Kind::UserPredictions => 'b',
            Kind::Whispers => 'w',
            Kind::Onsite => 'o',
        }
    }

    fn from_code(c: char) -> Option<Self> {
        KINDS.into_iter().find(|k| k.code() == c)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Topic {
    pub kind: Kind,
    /// The channel's user id; for user topics, the user's.
    pub channel_id: String,
}

impl Topic {
    pub fn new(kind: Kind, id: &str) -> Self {
        Self {
            kind,
            channel_id: id.to_owned(),
        }
    }
}

impl Topic {
    /// Short and reversible: `p71092938`.
    fn sub_id(&self) -> String {
        format!("{}{}", self.kind.code(), self.channel_id)
    }

    fn from_sub_id(id: &str) -> Option<Self> {
        let mut chars = id.chars();
        let kind = Kind::from_code(chars.next()?)?;
        Some(Self {
            kind,
            channel_id: chars.as_str().to_owned(),
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Event {
    pub topic: Topic,
    /// The topic's own JSON, e.g. `{"type":"POLL_UPDATE","data":{…}}`.
    pub payload: Value,
}

/// Reads the token fresh on each sign-in, so a refreshed one is used after reconnects.
pub type Token = Box<dyn Fn() -> Option<String> + Send>;

/// One connection. Anonymous for channel topics, or signed in for user topics too.
#[derive(Default)]
pub struct Hermes {
    token: Option<Token>,
}

impl Hermes {
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// `token` is the account's GQL (Android TV) token, without `OAuth `.
    pub fn signed_in(token: Token) -> Self {
        Self { token: Some(token) }
    }
}

impl Protocol for Hermes {
    type Topic = Topic;
    type Event = Event;

    fn url(&self) -> String {
        format!("wss://hermes.twitch.tv/v1?clientId={}", clients::WEB)
    }

    fn subscribe(&self, t: &Topic) -> String {
        let id = t.sub_id();
        json!({
            "type": "subscribe",
            "id": format!("r{id}"),
            "subscribe": { "id": id, "type": "pubsub", "pubsub": { "topic": format!("{}{}", t.kind.prefix(), t.channel_id) } },
            "timestamp": now_iso(),
        })
        .to_string()
    }

    fn unsubscribe(&self, t: &Topic) -> String {
        let id = t.sub_id();
        json!({
            "type": "unsubscribe",
            "id": format!("u{id}"),
            "unsubscribe": { "id": id },
            "timestamp": now_iso(),
        })
        .to_string()
    }

    fn limit(&self) -> usize {
        LIMIT
    }

    fn idle(&self) -> Duration {
        IDLE
    }

    fn gated(&self) -> bool {
        self.token.is_some()
    }

    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Event>) -> Step {
        let Ok(frame) = serde_json::from_str::<Frame>(text) else {
            return Step::Continue;
        };
        match frame.kind.as_str() {
            "welcome" => match self.token.as_ref() {
                None => Step::Continue,
                Some(token) => match token() {
                    Some(t) => Step::Reply(authenticate(&t)),
                    None => Step::Reconnect,
                },
            },
            "authenticateResponse" => match frame.authenticate_response.as_ref().map(|r| r.result.as_str()) {
                Some("ok") => Step::Ready,
                _ => {
                    eprintln!("hermes: sign-in refused: {text}");
                    Step::Reconnect
                }
            },
            "reconnect" => Step::Reconnect,
            "notification" => {
                if let Some(e) = frame.notification.and_then(Notification::event) {
                    let _ = out.send(e);
                }
                Step::Continue
            }
            _ => Step::Continue,
        }
    }
}

fn authenticate(token: &str) -> String {
    json!({
        "type": "authenticate",
        "id": crate::random::hex(10),
        "authenticate": { "token": token },
        "timestamp": now_iso(),
    })
    .to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frame {
    #[serde(rename = "type")]
    kind: String,
    notification: Option<Notification>,
    authenticate_response: Option<AuthResponse>,
}

#[derive(Deserialize)]
struct AuthResponse {
    result: String,
}

#[derive(Deserialize)]
struct Notification {
    subscription: Sub,
    pubsub: Option<String>,
}

#[derive(Deserialize)]
struct Sub {
    id: String,
}

impl Notification {
    fn event(self) -> Option<Event> {
        Some(Event {
            topic: Topic::from_sub_id(&self.subscription.id)?,
            payload: serde_json::from_str(&self.pubsub?).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames() {
        let t = Topic {
            kind: Kind::Polls,
            channel_id: "71092938".into(),
        };
        let mut h = Hermes::anonymous();
        let sub = h.subscribe(&t);
        assert!(sub.contains(r#""topic":"polls.71092938""#) && sub.contains(r#""id":"p71092938""#));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let note = r#"{"type":"notification","notification":{"subscription":{"id":"p71092938"},"type":"pubsub","pubsub":"{\"type\":\"POLL_UPDATE\",\"data\":{\"poll\":{\"poll_id\":\"x\"}}}"}}"#;
        assert!(matches!(h.read(note, &tx), Step::Continue));
        let e = rx.try_recv().unwrap();
        assert_eq!(e.topic, t);
        assert_eq!(e.payload["type"], "POLL_UPDATE");
        assert!(matches!(
            h.read(r#"{"type":"reconnect","reconnect":{"url":"x"}}"#, &tx),
            Step::Reconnect
        ));
        assert!(matches!(h.read(r#"{"type":"keepalive"}"#, &tx), Step::Continue));
    }

    #[test]
    fn signs_in_before_topics() {
        let mut h = Hermes::signed_in(Box::new(|| Some("tok".into())));
        assert!(h.gated());
        let (tx, _rx) = mpsc::unbounded_channel();
        let welcome = r#"{"id":"a","type":"welcome","welcome":{"keepaliveSec":15,"sessionId":"s"}}"#;
        let Step::Reply(line) = h.read(welcome, &tx) else { panic!("no sign-in") };
        assert!(line.contains(r#""type":"authenticate""#) && line.contains(r#""token":"tok""#));
        let ok = r#"{"id":"b","type":"authenticateResponse","authenticateResponse":{"result":"ok"}}"#;
        assert!(matches!(h.read(ok, &tx), Step::Ready));
        let bad = r#"{"id":"c","type":"authenticateResponse","authenticateResponse":{"result":"error","error":"bad"}}"#;
        assert!(matches!(h.read(bad, &tx), Step::Reconnect));
        assert!(!Hermes::anonymous().gated());
    }

    #[test]
    fn every_kind_round_trips() {
        for k in KINDS {
            let t = Topic::new(k, "42");
            assert_eq!(Topic::from_sub_id(&t.sub_id()), Some(t));
        }
    }
}
