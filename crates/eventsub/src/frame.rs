//! The frames an EventSub websocket sends: welcome, keepalive, notification, reconnect
//! and revocation. Each carries `metadata` and `payload`.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum Frame {
    /// A session is open; subscriptions must follow within 10 s.
    Welcome { session_id: String, keepalive_secs: u64 },
    Keepalive,
    Notification {
        message_id: String,
        kind: String,
        condition: Value,
        event: Value,
    },
    /// Move to `url`; subscriptions carry over.
    Reconnect { url: String },
    /// Twitch dropped a subscription, e.g. the account lost the role.
    Revocation { kind: String, status: String, condition: Value },
}

#[derive(Deserialize)]
struct Envelope {
    metadata: Metadata,
    #[serde(default)]
    payload: Value,
}

#[derive(Deserialize)]
struct Metadata {
    message_id: String,
    message_type: String,
}

#[derive(Deserialize)]
struct Session {
    id: String,
    #[serde(default)]
    keepalive_timeout_seconds: Option<u64>,
    #[serde(default)]
    reconnect_url: Option<String>,
}

#[derive(Deserialize)]
struct Subscription {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    condition: Value,
}

pub fn parse(text: &str) -> Option<Frame> {
    let env: Envelope = serde_json::from_str(text).ok()?;
    let p = env.payload;
    let session = || serde_json::from_value::<Session>(p.get("session")?.clone()).ok();
    let subscription = || serde_json::from_value::<Subscription>(p.get("subscription")?.clone()).ok();
    Some(match env.metadata.message_type.as_str() {
        "session_welcome" => {
            let s = session()?;
            Frame::Welcome {
                session_id: s.id,
                keepalive_secs: s.keepalive_timeout_seconds.unwrap_or(10),
            }
        }
        "session_keepalive" => Frame::Keepalive,
        "session_reconnect" => Frame::Reconnect {
            url: session()?.reconnect_url?,
        },
        "notification" => {
            let s = subscription()?;
            Frame::Notification {
                message_id: env.metadata.message_id,
                kind: s.kind,
                condition: s.condition,
                event: p.get("event").cloned().unwrap_or(Value::Null),
            }
        }
        "revocation" => {
            let s = subscription()?;
            Frame::Revocation {
                kind: s.kind,
                status: s.status,
                condition: s.condition,
            }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_and_keepalive() {
        let f = parse(
            r#"{"metadata":{"message_id":"96a3f3b5","message_type":"session_welcome","message_timestamp":"2023-07-19T14:56:51.634234626Z"},
            "payload":{"session":{"id":"AQoQILE98gtqShGmLD7AM6yJThAB","status":"connected","connected_at":"2023-07-19T14:56:51.616329898Z",
            "keepalive_timeout_seconds":10,"reconnect_url":null}}}"#,
        );
        assert_eq!(
            f,
            Some(Frame::Welcome {
                session_id: "AQoQILE98gtqShGmLD7AM6yJThAB".into(),
                keepalive_secs: 10
            })
        );
        let k = parse(r#"{"metadata":{"message_id":"84c1e79a","message_type":"session_keepalive","message_timestamp":"x"},"payload":{}}"#);
        assert_eq!(k, Some(Frame::Keepalive));
    }

    #[test]
    fn notification_reconnect_revocation() {
        let n = parse(
            r#"{"metadata":{"message_id":"befa7b53","message_type":"notification","message_timestamp":"x",
            "subscription_type":"channel.follow","subscription_version":"2"},
            "payload":{"subscription":{"id":"f1c2a387","status":"enabled","type":"channel.follow","version":"2",
            "cost":0,"condition":{"broadcaster_user_id":"12826","moderator_user_id":"12826"},
            "transport":{"method":"websocket","session_id":"AQoQ"},"created_at":"x"},
            "event":{"user_id":"1337","user_login":"awesome_user","user_name":"Awesome_User","followed_at":"2023-07-15T18:16:11.17106713Z"}}}"#,
        )
        .unwrap();
        let Frame::Notification { kind, event, condition, .. } = n else { panic!() };
        assert_eq!(kind, "channel.follow");
        assert_eq!(event["user_login"], "awesome_user");
        assert_eq!(condition["broadcaster_user_id"], "12826");
        let r = parse(
            r#"{"metadata":{"message_id":"84c1e79a","message_type":"session_reconnect","message_timestamp":"x"},
            "payload":{"session":{"id":"AQoQ","status":"reconnecting","keepalive_timeout_seconds":null,
            "reconnect_url":"wss://eventsub.wss.twitch.tv?reconnect=x","connected_at":"x"}}}"#,
        );
        assert_eq!(
            r,
            Some(Frame::Reconnect {
                url: "wss://eventsub.wss.twitch.tv?reconnect=x".into()
            })
        );
        let v = parse(
            r#"{"metadata":{"message_id":"84c1e79a","message_type":"revocation","message_timestamp":"x"},
            "payload":{"subscription":{"id":"f1c2a387","status":"authorization_revoked","type":"channel.follow",
            "version":"1","cost":0,"condition":{"broadcaster_user_id":"1"}}}}"#,
        )
        .unwrap();
        assert!(matches!(v, Frame::Revocation { status, .. } if status == "authorization_revoked"));
        assert_eq!(parse("not json"), None);
    }
}
