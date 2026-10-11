//! 7TV EventAPI connection: per channel, cosmetics, entitlements and the channel's set;
//! plus sets and users by id. Resume never works there, so every reconnect subscribes
//! again (research/providers.md).

use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;

use super::events::{Event, Frame, parse};
use cyex_twitch::ws::{Protocol, Step};

const URL: &str = "wss://events.7tv.io/v3";
/// 7TV allows 500 topics; each channel subscription also costs a hidden one.
const LIMIT: usize = 400;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Topic {
    /// `cosmetic.*`, `entitlement.*` or `emote_set.*` for chatters in a channel.
    Channel { kind: &'static str, twitch_id: String },
    /// Changes to one emote set.
    Set(String),
    /// A 7TV user switching sets.
    User(String),
}

impl Topic {
    /// The three channel topics.
    pub fn channel(twitch_id: &str) -> [Topic; 3] {
        ["cosmetic.*", "entitlement.*", "emote_set.*"].map(|kind| Topic::Channel {
            kind,
            twitch_id: twitch_id.to_owned(),
        })
    }

    fn body(&self) -> serde_json::Value {
        match self {
            Topic::Channel { kind, twitch_id } => json!({
                "type": kind,
                "condition": { "ctx": "channel", "platform": "TWITCH", "id": twitch_id },
            }),
            Topic::Set(id) => json!({ "type": "emote_set.*", "condition": { "object_id": id } }),
            Topic::User(id) => json!({ "type": "user.*", "condition": { "object_id": id } }),
        }
    }
}

pub struct EventApi {
    idle: Duration,
}

impl EventApi {
    pub fn new() -> Self {
        Self {
            idle: Duration::from_secs(135),
        }
    }
}

impl Default for EventApi {
    fn default() -> Self {
        Self::new()
    }
}

impl Protocol for EventApi {
    type Topic = Topic;
    type Event = Event;

    fn url(&self) -> String {
        format!("{URL}?app={}&version={}", cyex_core::app::SLUG, env!("CARGO_PKG_VERSION"))
    }

    fn subscribe(&self, topic: &Topic) -> String {
        json!({ "op": 35, "d": topic.body() }).to_string()
    }

    fn unsubscribe(&self, topic: &Topic) -> String {
        json!({ "op": 36, "d": topic.body() }).to_string()
    }

    fn limit(&self) -> usize {
        LIMIT
    }

    fn idle(&self) -> Duration {
        self.idle
    }

    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Event>) -> Step {
        match parse(text) {
            Frame::Hello { heartbeat_ms } => {
                self.idle = Duration::from_millis(heartbeat_ms.clamp(10_000, 120_000) * 3);
            }
            Frame::Event(e) => {
                let _ = out.send(*e);
            }
            Frame::Reconnect => return Step::Reconnect,
            Frame::Error(e) => eprintln!("7tv: {e}"),
            Frame::Heartbeat | Frame::Other => {}
        }
        Step::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriptions_look_like_7tvs() {
        let api = EventApi::new();
        let [c, ..] = Topic::channel("71092938");
        assert_eq!(
            api.subscribe(&c),
            r#"{"d":{"condition":{"ctx":"channel","id":"71092938","platform":"TWITCH"},"type":"cosmetic.*"},"op":35}"#
        );
        assert!(api.unsubscribe(&Topic::Set("s".into())).contains(r#""op":36"#));
    }

    #[test]
    fn hello_sets_the_watchdog() {
        let mut api = EventApi::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        api.read(r#"{"op":1,"d":{"heartbeat_interval":45000}}"#, &tx);
        assert_eq!(api.idle(), Duration::from_secs(135));
        assert!(matches!(api.read(r#"{"op":4,"d":{}}"#, &tx), Step::Reconnect));
    }
}
