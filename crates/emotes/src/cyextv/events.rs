//! V112-05 cyex.tv event frames (ADR 0005): `{ "op": …, "d": { … } }` in, plain events out.
//! Only what the deck acts on is parsed; other dispatch types are ignored, not errors.

use serde_json::Value;

/// One frame from the server.
#[derive(Debug, PartialEq)]
pub enum Frame {
    Hello {
        session_id: String,
        heartbeat_ms: u64,
    },
    Heartbeat,
    /// The server is restarting: connect again and resume.
    Reconnect,
    /// `error` (the connection closes after it).
    Error(String),
    Ack(Ack),
    Dispatch {
        /// The outbox id (`resume` replays after the last one seen); `None` for a made-up
        /// event such as `resync`.
        id: Option<u64>,
        /// `None` for a type this client doesn't use.
        event: Option<Event>,
    },
    Other,
}

/// The server's answer to `subscribe`, `unsubscribe` or `resume`.
#[derive(Debug, Default, PartialEq)]
pub struct Ack {
    pub op: String,
    /// A refused `resume`: `unknown_session`, `already_resumed`, `too_many_topics`.
    pub error: Option<String>,
    /// Topics a `subscribe` refused, with why.
    pub refused: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetChange {
    Added,
    Removed,
    Renamed,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    /// An emote went into, out of, or changed its name in a set.
    Set {
        kind: SetChange,
        set_id: String,
        emote_id: Option<String>,
        name: Option<String>,
    },
    /// A channel switched its active set (`None`: switched it off).
    ActiveSet { twitch_id: String, set_id: Option<String> },
    /// A personal set changed.
    Personal { set_id: String },
    /// An emote's own data changed (renamed, taken down).
    EmoteUpdated { emote_id: String },
    /// V112-13 A chatter's paint or badge changed (`user:twitch:<id>`); `None` = nothing worn.
    UserCosmetics {
        twitch_id: String,
        badge_id: Option<String>,
        paint_id: Option<String>,
    },
    /// V112-13 The catalog of paints and badges changed (created, updated or deleted).
    Cosmetics,
    /// Read what you hold again.
    Resync,
}

pub fn parse(text: &str) -> Frame {
    let Ok(raw) = serde_json::from_str::<Value>(text) else {
        return Frame::Other;
    };
    let d = &raw["d"];
    match raw["op"].as_str().unwrap_or_default() {
        "hello" => Frame::Hello {
            session_id: d["session_id"].as_str().unwrap_or_default().to_owned(),
            heartbeat_ms: d["heartbeat_interval"].as_u64().unwrap_or(20_000),
        },
        "heartbeat" => Frame::Heartbeat,
        "reconnect" => Frame::Reconnect,
        "error" => Frame::Error(d["code"].as_str().unwrap_or("error").to_owned()),
        "ack" => Frame::Ack(Ack {
            op: d["op"].as_str().unwrap_or_default().to_owned(),
            error: d["error"].as_str().map(str::to_owned),
            refused: d["errors"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(t, e)| (t.clone(), e.as_str().unwrap_or("error").to_owned()))
                .collect(),
        }),
        "dispatch" => Frame::Dispatch {
            id: d["id"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| d["id"].as_u64())
                .filter(|id| *id > 0),
            event: event(d["type"].as_str().unwrap_or_default(), &d["data"]),
        },
        _ => Frame::Other,
    }
}

fn event(kind: &str, data: &Value) -> Option<Event> {
    let text = |key: &str| {
        data[key]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let set = |kind| {
        Some(Event::Set {
            kind,
            set_id: text("set_id")?,
            emote_id: text("emote_id"),
            name: text("name"),
        })
    };
    match kind {
        "set.emote_added" => set(SetChange::Added),
        "set.emote_removed" => set(SetChange::Removed),
        "set.emote_renamed" => set(SetChange::Renamed),
        "channel.active_set_changed" => Some(Event::ActiveSet {
            twitch_id: text("topic")?.strip_prefix("channel:twitch:")?.to_owned(),
            set_id: text("set_id"),
        }),
        "personal.updated" => Some(Event::Personal {
            set_id: text("set_id")?,
        }),
        "emote.updated" => Some(Event::EmoteUpdated {
            emote_id: text("emote_id")?,
        }),
        "user.cosmetics_updated" => Some(Event::UserCosmetics {
            twitch_id: text("topic")?.strip_prefix("user:twitch:")?.to_owned(),
            badge_id: text("badge_id"),
            paint_id: text("paint_id"),
        }),
        "cosmetic.created" | "cosmetic.updated" | "cosmetic.deleted" => Some(Event::Cosmetics),
        "resync" => Some(Event::Resync),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dispatch(id: &str, kind: &str, data: &str) -> String {
        format!(r#"{{"op":"dispatch","d":{{"id":"{id}","type":"{kind}","data":{data}}}}}"#)
    }

    #[test]
    fn hello_carries_the_session_and_the_beat() {
        let f = parse(r#"{"op":"hello","d":{"session_id":"S1","heartbeat_interval":20000,"max_subscriptions":500}}"#);
        assert_eq!(
            f,
            Frame::Hello {
                session_id: "S1".into(),
                heartbeat_ms: 20_000
            }
        );
    }

    #[test]
    fn plain_ops() {
        assert_eq!(parse(r#"{"op":"heartbeat","d":{}}"#), Frame::Heartbeat);
        assert_eq!(parse(r#"{"op":"reconnect","d":{}}"#), Frame::Reconnect);
        assert_eq!(
            parse(r#"{"op":"error","d":{"code":"rate_limited","message":"x"}}"#),
            Frame::Error("rate_limited".into())
        );
        assert_eq!(parse("not json"), Frame::Other);
        assert_eq!(parse(r#"{"op":"later","d":{}}"#), Frame::Other);
        assert_eq!(parse(r#"{"op":3}"#), Frame::Other);
    }

    #[test]
    fn acks_name_what_was_refused() {
        let f = parse(r#"{"op":"ack","d":{"op":"subscribe","topics":["a"],"errors":{"set:x":"invalid_topic"}}}"#);
        assert_eq!(
            f,
            Frame::Ack(Ack {
                op: "subscribe".into(),
                error: None,
                refused: vec![("set:x".into(), "invalid_topic".into())],
            })
        );
        let f = parse(r#"{"op":"ack","d":{"op":"resume","error":"unknown_session"}}"#);
        assert_eq!(
            f,
            Frame::Ack(Ack {
                op: "resume".into(),
                error: Some("unknown_session".into()),
                refused: Vec::new(),
            })
        );
    }

    #[test]
    fn set_changes_carry_the_set() {
        let data = r#"{"topic":"set:S","set_id":"S","emote_id":"E","name":"hi","created_at":"x"}"#;
        for (kind, want) in [
            ("set.emote_added", SetChange::Added),
            ("set.emote_removed", SetChange::Removed),
            ("set.emote_renamed", SetChange::Renamed),
        ] {
            assert_eq!(
                parse(&dispatch("41", kind, data)),
                Frame::Dispatch {
                    id: Some(41),
                    event: Some(Event::Set {
                        kind: want,
                        set_id: "S".into(),
                        emote_id: Some("E".into()),
                        name: Some("hi".into()),
                    })
                }
            );
        }
        // No set, nothing to read again.
        let f = parse(&dispatch("5", "set.emote_added", r#"{"set_id":null}"#));
        assert_eq!(f, Frame::Dispatch { id: Some(5), event: None });
    }

    #[test]
    fn the_active_set_comes_with_the_channel_topic() {
        let f = parse(&dispatch(
            "7",
            "channel.active_set_changed",
            r#"{"topic":"channel:twitch:123","set_id":"S2"}"#,
        ));
        assert_eq!(
            f,
            Frame::Dispatch {
                id: Some(7),
                event: Some(Event::ActiveSet {
                    twitch_id: "123".into(),
                    set_id: Some("S2".into())
                })
            }
        );
        let off = parse(&dispatch(
            "8",
            "channel.active_set_changed",
            r#"{"topic":"channel:twitch:123","set_id":null}"#,
        ));
        assert!(matches!(
            off,
            Frame::Dispatch {
                event: Some(Event::ActiveSet { set_id: None, .. }),
                ..
            }
        ));
        // Another platform's channel isn't one of ours.
        let kick = parse(&dispatch(
            "9",
            "channel.active_set_changed",
            r#"{"topic":"channel:kick:9","set_id":"S"}"#,
        ));
        assert_eq!(kick, Frame::Dispatch { id: Some(9), event: None });
    }

    #[test]
    fn personal_emote_and_resync() {
        let f = parse(&dispatch("1", "personal.updated", r#"{"topic":"set:P","set_id":"P"}"#));
        assert!(matches!(
            f,
            Frame::Dispatch {
                event: Some(Event::Personal { .. }),
                ..
            }
        ));
        let f = parse(&dispatch("2", "emote.updated", r#"{"emote_id":"E"}"#));
        assert!(matches!(
            f,
            Frame::Dispatch {
                event: Some(Event::EmoteUpdated { .. }),
                ..
            }
        ));
        // A made-up resync has id 0, which isn't a place to resume from.
        let f = parse(&dispatch("0", "resync", r#"{"created_at":"x"}"#));
        assert_eq!(
            f,
            Frame::Dispatch {
                id: None,
                event: Some(Event::Resync)
            }
        );
    }

    #[test]
    fn cosmetics_events() {
        let d = dispatch("5", "user.cosmetics_updated", r#"{"topic":"user:twitch:77","badge_id":"B","paint_id":null}"#);
        assert_eq!(
            parse(&d),
            Frame::Dispatch {
                id: Some(5),
                event: Some(Event::UserCosmetics {
                    twitch_id: "77".into(),
                    badge_id: Some("B".into()),
                    paint_id: None
                })
            }
        );
        // Without a user topic there is no one to apply it to.
        let d = dispatch("5", "user.cosmetics_updated", r#"{"badge_id":"B"}"#);
        assert_eq!(parse(&d), Frame::Dispatch { id: Some(5), event: None });
        for kind in ["cosmetic.created", "cosmetic.updated", "cosmetic.deleted"] {
            let d = dispatch("6", kind, r#"{"topic":"cosmetics","cosmetic_kind":"paint","cosmetic_id":"P"}"#);
            assert_eq!(parse(&d), Frame::Dispatch { id: Some(6), event: Some(Event::Cosmetics) }, "{kind}");
        }
    }

    #[test]
    fn unknown_types_are_ignored() {
        for kind in ["set.created", "future.thing"] {
            let f = parse(&dispatch("3", kind, "{}"));
            assert_eq!(f, Frame::Dispatch { id: Some(3), event: None }, "{kind}");
        }
        // A frame with the wrong shapes is not an error either.
        assert_eq!(
            parse(r#"{"op":"dispatch","d":{"id":3,"type":7,"data":[]}}"#),
            Frame::Dispatch { id: Some(3), event: None }
        );
    }
}
