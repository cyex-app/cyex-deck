//! 7TV EventAPI connection: per channel, cosmetics, entitlements and the channel's set;
//! plus sets and users by id. Resume never works there, so every reconnect subscribes
//! again (research/providers.md). F18 (v1.04 batch 2b) The app runs it as a `ws::Pool`: a
//! full connection (about 80 chats) opens another, so no chat goes without live updates.

use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;

use super::events::{Event, Frame, parse};
use cyex_twitch::ws::{Protocol, Step};

const URL: &str = "wss://events.7tv.io/v3";
/// 7TV allows 500 topics a connection (HELLO's `subscription_limit`, over it closes 4005).
/// A channel's three topics cost 4 there, so `cost` counts its `cosmetic.*` twice; with a
/// chat's user and set that's 6 a chat, 80 a connection. 20 to spare for a channel split
/// over two connections.
const LIMIT: usize = 480;

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

    fn cost(&self, topic: &Topic) -> usize {
        match topic {
            Topic::Channel { kind, .. } if *kind == "cosmetic.*" => 2,
            _ => 1,
        }
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
    use cyex_twitch::ws::Pool;

    fn join(pool: &Pool<EventApi>, i: usize) {
        for t in Topic::channel(&i.to_string()) {
            pool.add(t);
        }
        pool.add(Topic::User(format!("u{i}")));
        pool.add(Topic::Set(format!("s{i}")));
    }

    #[test]
    fn a_full_connection_opens_another() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let pool = Pool::new(EventApi::new, tx, |_| {});
        for i in 0..80 {
            join(&pool, i);
        }
        assert_eq!(pool.loads(), vec![480], "80 chats fill one");
        join(&pool, 80);
        assert_eq!(pool.loads(), vec![480, 6]);
        // A chat that leaves makes room in the first again.
        for t in Topic::channel("3") {
            pool.remove(&t);
        }
        pool.remove(&Topic::User("u3".into()));
        pool.remove(&Topic::Set("s3".into()));
        assert_eq!(pool.loads(), vec![474, 6]);
        join(&pool, 81);
        assert_eq!(pool.loads(), vec![480, 6]);
    }

    #[test]
    fn a_channel_costs_four() {
        let api = EventApi::new();
        assert_eq!(Topic::channel("1").iter().map(|t| api.cost(t)).sum::<usize>(), 4);
        assert_eq!(api.cost(&Topic::Set("s".into())), 1);
    }

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
