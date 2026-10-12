//! BTTV's socket: emotes a channel adds, renames or removes while you watch (C10), and
//! BTTV Pro badges and personal emotes of the people chatting (C9). Name effects are
//! left out (D22).

use std::time::Duration;

use cyex_chat::model::Provider;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::model::Emote;
use cyex_twitch::ws::{Protocol, Step};

const URL: &str = "wss://sockets.betterttv.net/ws";
/// BTTV sends no heartbeat; a quiet socket is renewed now and then.
const IDLE: Duration = Duration::from_secs(1800);
const LIMIT: usize = 100;

#[derive(Debug, PartialEq)]
pub enum Event {
    /// A BTTV Pro user: their badge and personal emotes.
    User {
        twitch_id: String,
        badge: Option<String>,
        emotes: Vec<Emote>,
    },
    Added {
        channel_id: String,
        emote: Emote,
    },
    Renamed {
        channel_id: String,
        id: String,
        name: String,
    },
    Removed {
        channel_id: String,
        id: String,
    },
}

/// The line that tells BTTV we're in a channel (D21).
pub fn broadcast_me(user_id: &str, channel_id: &str) -> String {
    json!({
        "name": "broadcast_me",
        "data": { "provider": "twitch", "providerId": user_id, "channel": format!("twitch:{channel_id}") },
    })
    .to_string()
}

#[derive(Default)]
pub struct Socket;

impl Protocol for Socket {
    /// Twitch channel ids.
    type Topic = String;
    type Event = Event;

    fn url(&self) -> String {
        URL.into()
    }

    fn subscribe(&self, channel_id: &String) -> String {
        json!({ "name": "join_channel", "data": { "name": format!("twitch:{channel_id}") } }).to_string()
    }

    fn unsubscribe(&self, channel_id: &String) -> String {
        json!({ "name": "part_channel", "data": { "name": format!("twitch:{channel_id}") } }).to_string()
    }

    fn limit(&self) -> usize {
        LIMIT
    }

    fn idle(&self) -> Duration {
        IDLE
    }

    fn read(&mut self, text: &str, out: &mpsc::UnboundedSender<Event>) -> Step {
        if let Some(e) = parse(text) {
            let _ = out.send(e);
        }
        Step::Continue
    }
}

pub fn parse(text: &str) -> Option<Event> {
    let frame: Frame = serde_json::from_str(text).ok()?;
    let d = frame.data;
    let channel = || d["channel"].as_str()?.strip_prefix("twitch:").map(str::to_owned);
    match frame.name.as_str() {
        "lookup_user" => {
            let u: RawUser = serde_json::from_value(d).ok()?;
            Some(Event::User {
                badge: u.badge.and_then(|b| b.url),
                emotes: if u.pro {
                    u.emotes.into_iter().map(RawEmote::emote).collect()
                } else {
                    Vec::new()
                },
                twitch_id: u.provider_id,
            })
        }
        "emote_create" => Some(Event::Added {
            channel_id: channel()?,
            emote: serde_json::from_value::<RawEmote>(d["emote"].clone())
                .ok()?
                .emote(),
        }),
        "emote_update" => Some(Event::Renamed {
            channel_id: channel()?,
            id: d.pointer("/emote/id")?.as_str()?.to_owned(),
            name: d.pointer("/emote/code")?.as_str()?.to_owned(),
        }),
        "emote_delete" => Some(Event::Removed {
            channel_id: channel()?,
            id: d["emoteId"].as_str()?.to_owned(),
        }),
        _ => None,
    }
}

#[derive(Deserialize)]
struct Frame {
    name: String,
    #[serde(default)]
    data: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    provider_id: String,
    #[serde(default)]
    pro: bool,
    badge: Option<RawBadge>,
    #[serde(default)]
    emotes: Vec<RawEmote>,
}

#[derive(Deserialize)]
struct RawBadge {
    url: Option<String>,
}

#[derive(Deserialize)]
struct RawEmote {
    id: String,
    code: String,
    #[serde(default)]
    animated: bool,
}

impl RawEmote {
    fn emote(self) -> Emote {
        let mut e = Emote::new(Provider::Bttv, self.id, self.code);
        e.animated = self.animated;
        e.size = Some((28, 28));
        e
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_parse() {
        let shapes: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/bttv/socket_shapes_from_source.json"
        ))
        .unwrap();
        let events: Vec<Event> = shapes["inbound"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| parse(&f.to_string()))
            .collect();
        assert_eq!(events.len(), 4);
        let Event::User {
            twitch_id,
            badge,
            emotes,
        } = &events[0]
        else {
            panic!()
        };
        assert_eq!(twitch_id, "12345678");
        assert!(badge.is_some());
        assert_eq!(emotes[0].name, "forsenBlunder");
        assert!(
            matches!(&events[1], Event::Added { channel_id, emote } if channel_id == "71092938" && emote.name == "RareHug")
        );
        assert!(matches!(&events[2], Event::Renamed { name, .. } if name == "RareHugRenamed"));
        assert!(matches!(&events[3], Event::Removed { .. }));
        assert!(parse("{\"name\":\"other\"}").is_none());
    }

    #[test]
    fn joins_name_the_channel() {
        assert_eq!(
            Socket.subscribe(&"1".to_owned()),
            r#"{"data":{"name":"twitch:1"},"name":"join_channel"}"#
        );
        assert!(broadcast_me("7", "1").contains(r#""providerId":"7""#));
    }
}
