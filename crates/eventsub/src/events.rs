//! The events a moderator's session gets, typed. Unknown types and shapes are skipped.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

/// Someone named in an event.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "EsUser")]
pub struct Who {
    pub id: String,
    pub login: String,
    pub name: String,
}

impl Who {
    /// From `<prefix>_id`, `<prefix>_login`, `<prefix>_name` (prefix `user`,
    /// `moderator_user`, …).
    fn read(v: &Value, prefix: &str) -> Option<Self> {
        let s = |k: &str| v.get(format!("{prefix}_{k}"))?.as_str().map(str::to_owned);
        Some(Self {
            id: s("id")?,
            login: s("login").unwrap_or_default(),
            name: s("name").unwrap_or_default(),
        })
    }

    fn user(v: &Value) -> Option<Self> {
        Self::read(v, "user")
    }
}

/// A text, emote or cheer piece of a message.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Fragment {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub emote: Option<FragmentEmote>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct FragmentEmote {
    pub id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub fragments: Vec<Fragment>,
}

/// A character range AutoMod or a blocked term flagged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct Boundary {
    pub start_pos: u32,
    pub end_pos: u32,
}

/// `channel.moderate` v2: one mod action, by `moderator`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moderate {
    pub moderator: Who,
    /// Twitch's action name: `ban`, `timeout`, `delete`, `slow`, `vip`, `warn`, …
    pub action: String,
    /// Who it was done to, if anyone.
    pub target: Option<Who>,
    pub reason: String,
    /// Timeouts: RFC 3339 end.
    pub expires_at: Option<String>,
    /// Deletes.
    pub message_id: Option<String>,
    pub message_body: Option<String>,
    /// Slow mode seconds, followers-only minutes, raid viewers.
    pub amount: Option<u64>,
    /// Blocked or permitted terms added or removed.
    pub terms: Vec<String>,
    /// Shared chat: the channel it happened in, when not this one.
    pub source_channel: Option<String>,
}

/// `automod.message.hold` v2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    pub message_id: String,
    pub user: Who,
    pub message: Message,
    /// `automod` or `blocked_term`.
    pub reason: String,
    /// AutoMod's category, e.g. `swearing`, and its level.
    pub category: Option<String>,
    pub level: Option<u32>,
    pub boundaries: Vec<Boundary>,
    /// Blocked terms found, by term id.
    pub terms: Vec<String>,
    /// RFC 3339.
    pub held_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Moderate(Moderate),
    Held(Held),
    /// `automod.message.update` v2: `Approved`, `Denied` or `Expired`.
    HeldDone {
        message_id: String,
        status: String,
        moderator: Option<Who>,
    },
    Follow {
        user: Who,
        followed_at: String,
    },
    /// M3 A message from a monitored or restricted user.
    Suspicious {
        user: Who,
        /// `active_monitoring`, `restricted` or `none`.
        status: String,
        /// `manually_added`, `ban_evader_detector`, `shared_channel_ban`.
        types: Vec<String>,
        /// `unknown`, `possible`, `likely`.
        evasion: String,
        shared_bans: usize,
        message: Message,
    },
    SuspiciousChanged {
        user: Who,
        moderator: Option<Who>,
        status: String,
    },
    /// M2
    UnbanRequest {
        id: String,
        user: Who,
        text: String,
        created_at: String,
    },
    UnbanResolved {
        id: String,
        user: Who,
        moderator: Option<Who>,
        /// `approved`, `denied`, `canceled`.
        status: String,
        note: String,
    },
    Shield {
        on: bool,
        moderator: Option<Who>,
    },
    WarningRead {
        user: Who,
    },
}

pub fn parse(kind: &str, e: &Value) -> Option<Event> {
    let str_of = |k: &str| e.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    Some(match kind {
        "channel.moderate" => Event::Moderate(moderate(e)?),
        "automod.message.hold" => Event::Held(held(e)?),
        "automod.message.update" => Event::HeldDone {
            message_id: str_of("message_id"),
            status: str_of("status"),
            moderator: Who::read(e, "moderator_user"),
        },
        "channel.follow" => Event::Follow {
            user: Who::user(e)?,
            followed_at: str_of("followed_at"),
        },
        "channel.suspicious_user.message" => Event::Suspicious {
            user: Who::user(e)?,
            status: str_of("low_trust_status"),
            types: serde_json::from_value(e.get("types")?.clone()).unwrap_or_default(),
            evasion: str_of("ban_evasion_evaluation"),
            shared_bans: e
                .get("shared_ban_channel_ids")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            message: serde_json::from_value(e.get("message")?.clone()).ok()?,
        },
        "channel.suspicious_user.update" => Event::SuspiciousChanged {
            user: Who::user(e)?,
            moderator: Who::read(e, "moderator_user"),
            status: str_of("low_trust_status"),
        },
        "channel.unban_request.create" => Event::UnbanRequest {
            id: str_of("id"),
            user: Who::user(e)?,
            text: str_of("text"),
            created_at: str_of("created_at"),
        },
        "channel.unban_request.resolve" => Event::UnbanResolved {
            id: str_of("id"),
            user: Who::user(e)?,
            moderator: Who::read(e, "moderator_user"),
            status: str_of("status"),
            note: str_of("resolution_text"),
        },
        "channel.shield_mode.begin" | "channel.shield_mode.end" => Event::Shield {
            on: kind.ends_with("begin"),
            moderator: Who::read(e, "moderator_user"),
        },
        "channel.warning.acknowledge" => Event::WarningRead { user: Who::user(e)? },
        _ => return None,
    })
}

fn moderate(e: &Value) -> Option<Moderate> {
    let action = e.get("action")?.as_str()?.to_owned();
    // Details sit under the action's name, except for terms and unban requests.
    let key = match action.as_str() {
        a if a.ends_with("_term") => "automod_terms",
        a if a.ends_with("_unban_request") => "unban_request",
        a => a,
    };
    let detail = e.get(key).filter(|v| !v.is_null()).unwrap_or(&Value::Null);
    let text = |k: &str| detail.get(k).and_then(Value::as_str).map(str::to_owned);
    let amount = ["wait_time_seconds", "follow_duration_minutes", "viewer_count"]
        .iter()
        .find_map(|k| detail.get(*k).and_then(Value::as_u64));
    let terms = detail
        .get("terms")
        .and_then(|t| serde_json::from_value::<Vec<String>>(t.clone()).ok())
        .unwrap_or_default();
    let source = e
        .get("source_broadcaster_user_login")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    Some(Moderate {
        moderator: Who::read(e, "moderator_user").unwrap_or_default(),
        target: Who::user(detail),
        reason: text("reason").or_else(|| text("moderator_message")).unwrap_or_default(),
        expires_at: text("expires_at"),
        message_id: text("message_id"),
        message_body: text("message_body"),
        amount,
        terms,
        source_channel: source,
        action,
    })
}

fn held(e: &Value) -> Option<Held> {
    let automod = e.get("automod").filter(|v| !v.is_null());
    let mut boundaries: Vec<Boundary> = automod
        .and_then(|a| serde_json::from_value(a.get("boundaries")?.clone()).ok())
        .unwrap_or_default();
    let mut terms = Vec::new();
    if let Some(found) = e
        .get("blocked_term")
        .and_then(|b| b.get("terms_found"))
        .and_then(Value::as_array)
    {
        for t in found {
            if let Some(id) = t.get("term_id").and_then(Value::as_str) {
                terms.push(id.to_owned());
            }
            if let Some(b) = t.get("boundary").and_then(|b| serde_json::from_value(b.clone()).ok()) {
                boundaries.push(b);
            }
        }
    }
    Some(Held {
        message_id: e.get("message_id")?.as_str()?.to_owned(),
        user: Who::user(e)?,
        message: serde_json::from_value(e.get("message")?.clone()).ok()?,
        reason: e.get("reason").and_then(Value::as_str).unwrap_or("automod").to_owned(),
        category: automod.and_then(|a| a.get("category")?.as_str().map(str::to_owned)),
        level: automod.and_then(|a| a.get("level")?.as_u64()).map(|l| l as u32),
        boundaries,
        terms,
        held_at: e.get("held_at").and_then(Value::as_str).unwrap_or_default().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: &str, json: &str) -> Event {
        parse(kind, &serde_json::from_str(json).unwrap()).unwrap()
    }

    #[test]
    fn timeouts_deletes_and_modes() {
        let Event::Moderate(m) = ev(
            "channel.moderate",
            r#"{"broadcaster_user_id":"1","broadcaster_user_login":"b","broadcaster_user_name":"B",
            "source_broadcaster_user_id":null,"source_broadcaster_user_login":null,"source_broadcaster_user_name":null,
            "moderator_user_id":"2","moderator_user_login":"m","moderator_user_name":"M","action":"timeout",
            "followers":null,"slow":null,"vip":null,"unvip":null,"mod":null,"unmod":null,"ban":null,"unban":null,
            "timeout":{"user_id":"3","user_login":"u","user_name":"U","reason":"caps","expires_at":"2024-01-01T00:10:00Z"},
            "untimeout":null,"raid":null,"unraid":null,"delete":null,"automod_terms":null,"unban_request":null,"warn":null}"#,
        ) else {
            panic!()
        };
        assert_eq!((m.action.as_str(), m.moderator.login.as_str()), ("timeout", "m"));
        assert_eq!(m.target.unwrap().login, "u");
        assert_eq!((m.reason.as_str(), m.expires_at.as_deref()), ("caps", Some("2024-01-01T00:10:00Z")));

        let Event::Moderate(d) = ev(
            "channel.moderate",
            r#"{"moderator_user_id":"2","moderator_user_login":"m","moderator_user_name":"M","action":"delete",
            "delete":{"user_id":"3","user_login":"u","user_name":"U","message_id":"abc","message_body":"bad words"}}"#,
        ) else {
            panic!()
        };
        assert_eq!((d.message_id.as_deref(), d.message_body.as_deref()), (Some("abc"), Some("bad words")));

        let Event::Moderate(s) = ev(
            "channel.moderate",
            r#"{"moderator_user_id":"2","action":"slow","slow":{"wait_time_seconds":30}}"#,
        ) else {
            panic!()
        };
        assert_eq!((s.amount, s.target), (Some(30), None));

        let Event::Moderate(t) = ev(
            "channel.moderate",
            r#"{"moderator_user_id":"2","action":"add_blocked_term",
            "automod_terms":{"action":"add","list":"blocked","terms":["bad"],"from_automod":false}}"#,
        ) else {
            panic!()
        };
        assert_eq!(t.terms, ["bad"]);
    }

    #[test]
    fn automod_holds() {
        let Event::Held(h) = ev(
            "automod.message.hold",
            r#"{"broadcaster_user_id":"1","user_id":"3","user_login":"u","user_name":"U","message_id":"m1",
            "message":{"text":"you suck","fragments":[{"type":"text","text":"you suck","cheermote":null,"emote":null}]},
            "reason":"automod","automod":{"category":"aggressive","level":1,"boundaries":[{"start_pos":4,"end_pos":7}]},
            "blocked_term":null,"held_at":"2024-01-01T00:00:00Z"}"#,
        ) else {
            panic!()
        };
        assert_eq!((h.category.as_deref(), h.level), (Some("aggressive"), Some(1)));
        assert_eq!(h.boundaries, [Boundary { start_pos: 4, end_pos: 7 }]);
        assert_eq!(h.message.text, "you suck");

        let Event::Held(b) = ev(
            "automod.message.hold",
            r#"{"user_id":"3","user_login":"u","user_name":"U","message_id":"m2","message":{"text":"bad"},
            "reason":"blocked_term","automod":null,"blocked_term":{"terms_found":[{"term_id":"t1",
            "boundary":{"start_pos":0,"end_pos":2},"owner_broadcaster_user_id":"1"}]},"held_at":"x"}"#,
        ) else {
            panic!()
        };
        assert_eq!((b.terms, b.boundaries.len()), (vec!["t1".to_owned()], 1));

        assert_eq!(
            ev(
                "automod.message.update",
                r#"{"message_id":"m1","status":"Approved","moderator_user_id":"2","moderator_user_login":"m","moderator_user_name":"M"}"#
            ),
            Event::HeldDone {
                message_id: "m1".into(),
                status: "Approved".into(),
                moderator: Some(Who {
                    id: "2".into(),
                    login: "m".into(),
                    name: "M".into()
                })
            }
        );
    }

    #[test]
    fn low_trust_unban_requests_shield() {
        let Event::Suspicious { status, types, message, .. } = ev(
            "channel.suspicious_user.message",
            r#"{"user_id":"3","user_login":"u","user_name":"U","low_trust_status":"restricted",
            "shared_ban_channel_ids":["9"],"types":["ban_evader_detector"],"ban_evasion_evaluation":"likely",
            "message":{"message_id":"x1","text":"hi","fragments":[]}}"#,
        ) else {
            panic!()
        };
        assert_eq!((status.as_str(), types[0].as_str(), message.message_id.as_str()), ("restricted", "ban_evader_detector", "x1"));
        assert!(matches!(
            ev("channel.unban_request.create", r#"{"id":"r1","user_id":"3","user_login":"u","user_name":"U","text":"pls","created_at":"x"}"#),
            Event::UnbanRequest { ref text, .. } if text == "pls"
        ));
        assert_eq!(
            ev("channel.shield_mode.end", r#"{"moderator_user_id":"2","moderator_user_login":"m","moderator_user_name":"M"}"#),
            Event::Shield {
                on: false,
                moderator: Some(Who {
                    id: "2".into(),
                    login: "m".into(),
                    name: "M".into()
                })
            }
        );
        assert!(parse("channel.unknown", &serde_json::json!({})).is_none());
    }
}
