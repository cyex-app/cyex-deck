//! Hermes, Twitch's websocket for the old PubSub topics (`docs/research/twitch-api.md`).
//! Read anonymously here: polls and predictions (C4) and on-screen celebrations (1.42).
//! Subscription ids encode their topic, so a notification names its channel.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::clients;
use crate::ws::{Protocol, Step};

/// Twitch asks for a keepalive every 15 s; two missed ones end the connection.
const IDLE: Duration = Duration::from_secs(35);
/// Hermes takes many topics per connection; this keeps it modest.
const LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Polls,
    Predictions,
    Celebrations,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Kind::Polls => "polls.",
            Kind::Predictions => "predictions-channel-v1.",
            Kind::Celebrations => "bits-rewards-celebration-v1.",
        }
    }

    fn code(self) -> char {
        match self {
            Kind::Polls => 'p',
            Kind::Predictions => 'r',
            Kind::Celebrations => 'c',
        }
    }

    fn from_code(c: char) -> Option<Self> {
        [Kind::Polls, Kind::Predictions, Kind::Celebrations]
            .into_iter()
            .find(|k| k.code() == c)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Topic {
    pub kind: Kind,
    pub channel_id: String,
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

#[derive(Default)]
pub struct Hermes;

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

    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Event>) -> Step {
        let Ok(frame) = serde_json::from_str::<Frame>(text) else {
            return Step::Continue;
        };
        match frame.kind.as_str() {
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

/// UTC now as `2026-10-10T08:59:33Z`.
fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[derive(Deserialize)]
struct Frame {
    #[serde(rename = "type")]
    kind: String,
    notification: Option<Notification>,
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
    fn iso_time_looks_right() {
        let t = now_iso();
        assert_eq!((t.len(), &t[4..5], &t[10..11]), (20, "-", "T"));
        assert!(t.starts_with("20"));
    }

    #[test]
    fn frames() {
        let t = Topic {
            kind: Kind::Polls,
            channel_id: "71092938".into(),
        };
        let sub = Hermes.subscribe(&t);
        assert!(sub.contains(r#""topic":"polls.71092938""#) && sub.contains(r#""id":"p71092938""#));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let note = r#"{"type":"notification","notification":{"subscription":{"id":"p71092938"},"type":"pubsub","pubsub":"{\"type\":\"POLL_UPDATE\",\"data\":{\"poll\":{\"poll_id\":\"x\"}}}"}}"#;
        assert!(matches!(Hermes.read(note, &tx), Step::Continue));
        let e = rx.try_recv().unwrap();
        assert_eq!(e.topic, t);
        assert_eq!(e.payload["type"], "POLL_UPDATE");
        assert!(matches!(
            Hermes.read(r#"{"type":"reconnect","reconnect":{"url":"x"}}"#, &tx),
            Step::Reconnect
        ));
        assert!(matches!(
            Hermes.read(r#"{"type":"keepalive"}"#, &tx),
            Step::Continue
        ));
    }
}
