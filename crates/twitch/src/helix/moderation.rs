//! 1.49 Mod actions on people and messages: bans and timeouts (`moderator:manage:banned_users`),
//! deleting (`moderator:manage:chat_messages`), warnings (`moderator:manage:warnings`),
//! AutoMod's held messages (1.56, `moderator:manage:automod`), unban requests (M2) and
//! blocked terms (1.48). Bans report the rate limit so mass actions can pace (1.55).

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, Data, Room, get, pages, post, request};
use crate::http::{empty, json, limited};
use crate::{Http, RateLimit, Result};

/// Twitch's longest timeout, two weeks.
pub const MAX_TIMEOUT_SECS: u32 = 1_209_600;

/// Bans, or times out when `secs` is given (1 to two weeks). The reason may be empty.
pub async fn ban(
    http: &Http,
    auth: Auth<'_>,
    room: Room<'_>,
    user_id: &str,
    secs: Option<u32>,
    reason: &str,
) -> (Result<()>, Option<RateLimit>) {
    let mut data = json!({ "user_id": user_id });
    if let Some(s) = secs {
        data["duration"] = s.clamp(1, MAX_TIMEOUT_SECS).into();
    }
    if !reason.is_empty() {
        data["reason"] = reason.chars().take(500).collect::<String>().into();
    }
    let req = post(http, auth, "/moderation/bans")
        .query(&room.query())
        .json(&json!({ "data": data }));
    let (res, limit) = limited(req).await;
    (res.map(drop), limit)
}

/// Lifts a ban or a timeout.
pub async fn unban(http: &Http, auth: Auth<'_>, room: Room<'_>, user_id: &str) -> (Result<()>, Option<RateLimit>) {
    let req = request(http, auth, Method::DELETE, "/moderation/bans")
        .query(&room.query())
        .query(&[("user_id", user_id)]);
    let (res, limit) = limited(req).await;
    (res.map(drop), limit)
}

/// Deletes one message, or the whole chat with `None` (`/clear`).
pub async fn delete(http: &Http, auth: Auth<'_>, room: Room<'_>, message_id: Option<&str>) -> Result<()> {
    let mut req = request(http, auth, Method::DELETE, "/moderation/chat").query(&room.query());
    if let Some(id) = message_id {
        req = req.query(&[("message_id", id)]);
    }
    empty(req).await
}

pub async fn warn(http: &Http, auth: Auth<'_>, room: Room<'_>, user_id: &str, reason: &str) -> Result<()> {
    let req = post(http, auth, "/moderation/warnings")
        .query(&room.query())
        .json(&json!({ "data": { "user_id": user_id, "reason": reason } }));
    empty(req).await
}

/// M3 Monitors or restricts someone (`ACTIVE_MONITORING`, `RESTRICTED`); `None` ends it.
pub async fn treat(http: &Http, auth: Auth<'_>, room: Room<'_>, user_id: &str, status: Option<&str>) -> Result<()> {
    let req = match status {
        Some(s) => post(http, auth, "/moderation/suspicious_users")
            .query(&room.query())
            .json(&json!({ "user_id": user_id, "status": s })),
        None => request(http, auth, Method::DELETE, "/moderation/suspicious_users")
            .query(&room.query())
            .query(&[("user_id", user_id)]),
    };
    empty(req).await
}

/// 1.9 How long a pin may stay, in seconds, for Helix and GQL alike (V107, tested 2026/10/19:
/// any whole second inside works, 29 and 1801 are refused). No duration = no end.
pub const PIN_SECS: (u32, u32) = (30, 1800);

/// 1.9 Pins a message for `secs` (clamped to [`PIN_SECS`]), or until the stream ends.
pub async fn pin(http: &Http, auth: Auth<'_>, room: Room<'_>, message_id: &str, secs: Option<u32>) -> Result<()> {
    let mut req = request(http, auth, Method::PUT, "/chat/pins")
        .query(&room.query())
        .query(&[("message_id", message_id)]);
    if let Some(s) = secs {
        req = req.query(&[("duration_seconds", s.clamp(PIN_SECS.0, PIN_SECS.1))]);
    }
    empty(req).await
}

pub async fn unpin(http: &Http, auth: Auth<'_>, room: Room<'_>, message_id: &str) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/chat/pins")
        .query(&room.query())
        .query(&[("message_id", message_id)]);
    empty(req).await
}

/// 1.56 Lets a held message through, or drops it.
pub async fn automod(http: &Http, auth: Auth<'_>, moderator_id: &str, msg_id: &str, allow: bool) -> Result<()> {
    let action = if allow { "ALLOW" } else { "DENY" };
    let req = post(http, auth, "/moderation/automod/message")
        .json(&json!({ "user_id": moderator_id, "msg_id": msg_id, "action": action }));
    empty(req).await
}

#[derive(Deserialize)]
struct Shield {
    is_active: bool,
}

pub async fn shield(http: &Http, auth: Auth<'_>, room: Room<'_>) -> Result<bool> {
    let req = get(http, auth, "/moderation/shield_mode").query(&room.query());
    let d: Data<Shield> = json(req).await?;
    Ok(d.data.first().is_some_and(|s| s.is_active))
}

pub async fn set_shield(http: &Http, auth: Auth<'_>, room: Room<'_>, on: bool) -> Result<()> {
    let req = request(http, auth, Method::PUT, "/moderation/shield_mode")
        .query(&room.query())
        .json(&json!({ "is_active": on }));
    empty(req).await
}

/// M2 Someone asking to be unbanned.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase")]
#[ts(export)]
pub struct UnbanRequest {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    #[serde(default)]
    pub text: String,
    /// `pending`, `approved`, `denied`, `acknowledged`, `canceled`.
    pub status: String,
    /// RFC 3339.
    pub created_at: String,
}

/// M2 Pending unban requests, oldest first.
pub async fn unban_requests(http: &Http, auth: Auth<'_>, room: Room<'_>) -> Result<Vec<UnbanRequest>> {
    let make = || {
        get(http, auth, "/moderation/unban_requests")
            .query(&room.query())
            .query(&[("status", "pending"), ("first", "100")])
    };
    let (mut list, _) = pages::<UnbanRequest>(5, make).await?;
    list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(list)
}

/// M2 Approves or denies an unban request, with a note they'll see.
pub async fn resolve_unban(
    http: &Http,
    auth: Auth<'_>,
    room: Room<'_>,
    request_id: &str,
    approve: bool,
    note: &str,
) -> Result<()> {
    let status = if approve { "approved" } else { "denied" };
    let mut req = request(http, auth, Method::PATCH, "/moderation/unban_requests")
        .query(&room.query())
        .query(&[("unban_request_id", request_id), ("status", status)]);
    if !note.is_empty() {
        req = req.query(&[("resolution_text", note)]);
    }
    empty(req).await
}

/// 1.48 A blocked term.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase")]
#[ts(export)]
pub struct BlockedTerm {
    pub id: String,
    pub text: String,
    /// RFC 3339; `None` = for good.
    #[serde(default)]
    pub expires_at: Option<String>,
}

pub async fn blocked_terms(http: &Http, auth: Auth<'_>, room: Room<'_>) -> Result<Vec<BlockedTerm>> {
    let make = || {
        get(http, auth, "/moderation/blocked_terms")
            .query(&room.query())
            .query(&[("first", "100")])
    };
    Ok(pages(20, make).await?.0)
}

/// Adds a term (2 to 500 characters, `*` as a wildcard).
pub async fn add_blocked_term(http: &Http, auth: Auth<'_>, room: Room<'_>, text: &str) -> Result<BlockedTerm> {
    let req = post(http, auth, "/moderation/blocked_terms")
        .query(&room.query())
        .json(&json!({ "text": text }));
    let d: Data<BlockedTerm> = json(req).await?;
    d.data.into_iter().next().ok_or(crate::Error::Api {
        status: 200,
        message: "nothing came back".into(),
    })
}

pub async fn remove_blocked_term(http: &Http, auth: Auth<'_>, room: Room<'_>, id: &str) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/moderation/blocked_terms")
        .query(&room.query())
        .query(&[("id", id)]);
    empty(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::Page;

    #[test]
    fn unban_requests_parse() {
        let p: Page<UnbanRequest> = serde_json::from_str(
            r#"{"data":[{"id":"92af127c","broadcaster_name":"b","broadcaster_login":"b","broadcaster_id":"1",
            "moderator_id":null,"moderator_login":null,"moderator_name":null,"user_id":"2","user_login":"u",
            "user_name":"U","text":"sorry","status":"pending","created_at":"2024-05-01T00:00:00Z",
            "resolved_at":null,"resolution_text":null}],"pagination":{}}"#,
        )
        .unwrap();
        assert_eq!((p.data[0].user_login.as_str(), p.data[0].text.as_str()), ("u", "sorry"));
    }

    #[test]
    fn blocked_terms_parse() {
        let p: Page<BlockedTerm> = serde_json::from_str(
            r#"{"data":[{"broadcaster_id":"1","moderator_id":"2","id":"520e4d4e","text":"A phrase",
            "created_at":"2021-09-29T19:45:37Z","updated_at":"2021-09-29T19:45:37Z","expires_at":null}],
            "pagination":{"cursor":"x"}}"#,
        )
        .unwrap();
        assert_eq!(p.data[0].text, "A phrase");
        assert!(p.data[0].expires_at.is_none());
    }
}
