//! V108-C40 The account's own standing in a chat: a ban or timeout on it, whether it may ask
//! for an unban (and its last request), and Twitch's chat warnings. Checked by wrong-typed
//! validation probes 2026/11/04, signed out; neither mutation was ever run.
//! - Read: `chatRoomBanStatus(channelID, userID) { createdAt expiresAt isPermanent reason }`
//!   (no `moderator`: Twitch never tells the account who acted on it, V108-C40e), `channel(id) { unbanRequestsSettings { isEnabled cooldownMinutes } self {
//!   canRequestUnban } }` (`canRequestUnban: Boolean` on `ChannelSelfEdge`).
//! - Own request: `channel.unbanRequests(options: {userID})` is the mod read (V108-C36); for
//!   the requester it may be refused, so it's read apart and a failure means "unknown".
//! - Ask: `createUnbanRequest(input: {channelID: ID!, requesterMessage: String!}) {
//!   unbanRequest { id status createdAt } error { code: CreateUnbanRequestErrorCode! } }`.
//! - Acknowledge: `acknowledgeChatWarning(input: {channelID: ID!}) { error { code:
//!   AcknowledgeChatWarningErrorCode! } }` (the payload has nothing else).
//! - No field reads a pending warning (tried ~40 names on `ChannelSelfEdge`, `User`, `Query`,
//!   `ChatRoomBanStatus`). Live: Hermes `chatrooms-user-v1.<user id>` exists (SUB007 signed
//!   out, so it wants a token); its payloads were never seen, see [`change_of`].

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::modlogs::{self, BanStatus, PastUnban};
use super::{Auth, GqlError, Response, query, request, send_json, unwrap};
use crate::{Error, Http, Result};

/// The longest unban request message the box takes (Twitch's own limit isn't known).
pub const REQUEST_MAX: usize = 500;

/// Whether the channel takes unban requests, and how long after a ban.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "UnbanRules")]
pub struct UnbanRules {
    pub enabled: bool,
    /// Minutes after the ban before one can be sent.
    pub cooldown_minutes: u32,
}

/// One read of the account's standing in a channel.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ChatStanding")]
pub struct Standing {
    /// The ban or timeout on the account now; `None` = free to talk.
    pub ban: Option<BanStatus>,
    /// `None` when Twitch didn't say.
    pub unban: Option<UnbanRules>,
    /// Twitch's own `canRequestUnban`, `None` when it didn't say.
    pub can_request: Option<bool>,
    /// The account's newest request here, `None` when there's none or it can't be read.
    pub request: Option<PastUnban>,
}

const READ: &str = "query S($c: ID!, $u: ID!) {
  ban: chatRoomBanStatus(channelID: $c, userID: $u) {
    createdAt expiresAt isPermanent reason }
  channel(id: $c) { unbanRequestsSettings { isEnabled cooldownMinutes } self { canRequestUnban } } }";

/// The standing of `user_id` (the signed-in account) in `channel_id`.
pub async fn read(http: &Http, auth: Auth<'_>, channel_id: &str, user_id: &str) -> Result<Standing> {
    let vars = json!({ "c": channel_id, "u": user_id });
    let req = request(http, auth).json(&json!({ "query": READ, "variables": vars }));
    let data = answer(send_json(req).await?)?;
    let mut s = standing_of(&data);
    if s.ban.as_ref().is_some_and(|b| b.until.is_none()) {
        s.request = modlogs::unban_past(http, auth, channel_id, user_id)
            .await
            .ok()
            .and_then(|l| l.into_iter().next());
    }
    Ok(s)
}

/// The channel part may be refused to a viewer (`unbanRequestsSettings`), and what came back
/// still counts. But a null ban next to an error that isn't about the channel could be a
/// failed ban read, and "free to talk" there would unlock a banned account: that's an `Err`,
/// so the page keeps what it knew.
fn answer(res: Response<Value>) -> Result<Value> {
    let uncertain = res.data.as_ref().is_some_and(|d| ban_uncertain(d, &res.errors));
    if uncertain {
        return unwrap(Response { data: None, errors: res.errors });
    }
    super::partial(res, |_| true)
}

fn ban_uncertain(data: &Value, errors: &[GqlError]) -> bool {
    data["ban"].is_null() && errors.iter().any(|e| e.path.first().and_then(Value::as_str) != Some("channel"))
}

fn standing_of(data: &Value) -> Standing {
    let rules = &data["channel"]["unbanRequestsSettings"];
    Standing {
        // V108-C40e Never who did it, even if an answer carries it.
        ban: modlogs::ban_of(data).map(|b| BanStatus { by: None, ..b }),
        unban: rules.is_object().then(|| UnbanRules {
            enabled: rules["isEnabled"].as_bool().unwrap_or(false),
            cooldown_minutes: u32::try_from(rules["cooldownMinutes"].as_u64().unwrap_or(0)).unwrap_or(u32::MAX),
        }),
        can_request: data["channel"]["self"]["canRequestUnban"].as_bool(),
        request: None,
    }
}

/// Sends an unban request. Only from the user's click on Send. [`Error::Refused`] carries
/// Twitch's code (see [`explain`]).
pub async fn request_unban(http: &Http, auth: Auth<'_>, channel_id: &str, message: &str) -> Result<()> {
    const Q: &str = "mutation U($input: CreateUnbanRequestInput!) {
      createUnbanRequest(input: $input) { unbanRequest { id } error { code } } }";
    let vars = json!({ "input": { "channelID": channel_id, "requesterMessage": message } });
    let data: Value = query(http, auth, Q, vars).await?;
    answered(&data["createUnbanRequest"], |p| p["unbanRequest"]["id"].is_string())
}

/// Acknowledges the channel's warning, so the account can talk again. Only from the user's
/// click on Acknowledge.
pub async fn acknowledge_warning(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    const Q: &str = "mutation W($input: AcknowledgeChatWarningInput!) {
      acknowledgeChatWarning(input: $input) { error { code } } }";
    let data: Value = query(http, auth, Q, json!({ "input": { "channelID": channel_id } })).await?;
    answered(&data["acknowledgeChatWarning"], |_| true)
}

/// A mutation's payload: its error code, or `done` when there's none.
fn answered(payload: &Value, done: impl Fn(&Value) -> bool) -> Result<()> {
    if let Some(code) = payload["error"]["code"].as_str() {
        return Err(Error::Refused(code.to_owned()));
    }
    if payload.is_object() && done(payload) {
        Ok(())
    } else {
        Err(Error::Refused("UNKNOWN".into()))
    }
}

/// A short human line for Twitch's code (the enums' values aren't known; unknown ones get a
/// plain line).
pub fn explain(code: &str) -> &'static str {
    let c = code.to_ascii_uppercase();
    let has = |words: &[&str]| c.split('_').any(|t| words.contains(&t));
    let has_seq = |a: &str, b: &str| {
        let t: Vec<&str> = c.split('_').collect();
        t.windows(2).any(|w| w[0] == a && w[1] == b)
    };
    if has(&["ALREADY", "PENDING", "EXISTS"]) {
        "You already asked. Wait for an answer."
    } else if has(&["COOLDOWN", "RATE", "THROTTLE", "THROTTLED"]) || has_seq("TOO", "SOON") {
        "Not yet. Try again later."
    } else if has(&["DISABLED"]) || has_seq("NOT", "ENABLED") {
        "This channel doesn't take unban requests."
    } else if has(&["LONG", "LENGTH"]) {
        "That message is too long."
    } else if has_seq("NOT", "BANNED") || has_seq("NO", "WARNING") || has_seq("NOT", "FOUND") {
        "Nothing to do here anymore."
    } else {
        "Twitch said no. Try again later."
    }
}

/// A chat warning, as the live topic gives it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ChatWarning")]
pub struct Warning {
    pub reason: String,
    /// The channel's rules the mod cited.
    pub rules: Vec<String>,
}

/// What the account's `chatrooms-user-v1` topic said about one channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// Warned there; to acknowledge before talking.
    Warned(Warning),
    /// A ban, timeout, unban or anything else: read the standing again.
    Moved,
}

/// The PubSub topic with the account's own chat moderation.
pub fn topic(user_id: &str) -> String {
    format!("chatrooms-user-v1.{user_id}")
}

/// The channel and change in one message of that topic. Unverified live: the family's
/// known type is `user_moderation_action` (`data.action`: ban, timeout, unban, untimeout,
/// `data.channel_id`); a warning's shape was never seen, so any type or action naming one
/// counts, with `reason` and `chat_rules_cited` (EventSub's names) when present.
pub fn change_of(payload: &Value) -> Option<(String, Change)> {
    let data = payload.get("data")?;
    let channel = ["channel_id", "channelID", "channel_login_id"]
        .iter()
        .find_map(|k| data.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()))?
        .to_owned();
    let kind = payload.get("type").and_then(Value::as_str).unwrap_or_default();
    let action = data.get("action").and_then(Value::as_str).unwrap_or_default();
    let warned = [kind, action].iter().any(|t| {
        let t = t.to_ascii_lowercase();
        t.contains("warn") && !t.contains("ack")
    });
    if !warned {
        return Some((channel, Change::Moved));
    }
    let text = |k: &str| data.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    let rules = ["chat_rules_cited", "rules"]
        .iter()
        .find_map(|k| data.get(*k).and_then(Value::as_array))
        .map(|l| l.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    Some((channel, Change::Warned(Warning { reason: text("reason"), rules })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_ban_with_its_unban_rules() {
        let d = json!({
            "ban": { "createdAt": "2026-11-04T10:00:00Z", "expiresAt": null, "isPermanent": true, "reason": "spam",
                     "moderator": { "login": "kuni", "displayName": "Kuni" } },
            "channel": { "unbanRequestsSettings": { "isEnabled": true, "cooldownMinutes": 60 }, "self": { "canRequestUnban": false } }
        });
        let s = standing_of(&d);
        let ban = s.ban.unwrap();
        assert_eq!((ban.until, ban.by.as_deref(), ban.reason.as_str()), (None, None, "spam"));
        assert_eq!(ban.since.as_deref(), Some("2026-11-04T10:00:00Z"));
        assert_eq!(s.unban, Some(UnbanRules { enabled: true, cooldown_minutes: 60 }));
        assert_eq!(s.can_request, Some(false));
    }

    #[test]
    fn reads_free_and_refused_parts() {
        let s = standing_of(&json!({ "ban": null, "channel": { "unbanRequestsSettings": null, "self": null } }));
        assert_eq!(s, Standing::default());
        let t = standing_of(&json!({ "ban": { "expiresAt": "2026-11-04T10:05:00Z", "isPermanent": false } }));
        assert_eq!(t.ban.unwrap().until.as_deref(), Some("2026-11-04T10:05:00Z"));
    }

    #[test]
    fn mutation_answers() {
        assert!(answered(&json!({ "unbanRequest": { "id": "u1" }, "error": null }), |p| p["unbanRequest"]["id"].is_string()).is_ok());
        let e = answered(&json!({ "unbanRequest": null, "error": { "code": "ALREADY_PENDING" } }), |_| true).unwrap_err();
        assert!(matches!(&e, Error::Refused(c) if c == "ALREADY_PENDING"));
        assert!(answered(&Value::Null, |_| true).is_err());
        assert!(answered(&json!({ "error": null }), |_| true).is_ok());
        assert_eq!(explain("ALREADY_PENDING"), "You already asked. Wait for an answer.");
        assert_eq!(explain("SOMETHING_NEW"), "Twitch said no. Try again later.");
    }

    #[test]
    fn explains_by_whole_words() {
        assert_eq!(explain("MODERATE_FAILED"), "Twitch said no. Try again later.");
        assert_eq!(explain("RATE_LIMITED"), "Not yet. Try again later.");
        assert_eq!(explain("REQUEST_TOO_SOON"), "Not yet. Try again later.");
        assert_eq!(explain("NOT_BANNED"), "Nothing to do here anymore.");
        assert_eq!(explain("MESSAGE_TOO_LONG"), "That message is too long.");
    }

    #[test]
    fn ban_null_plus_errors_is_uncertain() {
        let err = |path: &str| -> Vec<GqlError> {
            serde_json::from_value(json!([{ "message": "x", "path": [path] }])).unwrap()
        };
        let none: Vec<GqlError> = serde_json::from_value(json!([{ "message": "x" }])).unwrap();
        let d = json!({ "ban": null, "channel": { "unbanRequestsSettings": null } });
        assert!(ban_uncertain(&d, &err("ban")), "the ban read failed");
        assert!(ban_uncertain(&d, &none), "an error with no path could be anything");
        assert!(!ban_uncertain(&d, &err("channel")), "a viewer refused the unban settings");
        assert!(!ban_uncertain(&d, &[]));
        assert!(!ban_uncertain(&json!({ "ban": { "isPermanent": true } }), &err("ban")));
        let res: Response<Value> =
            serde_json::from_value(json!({ "data": d, "errors": [{ "message": "service timeout", "path": ["ban"] }] })).unwrap();
        assert!(answer(res).is_err());
        let res: Response<Value> = serde_json::from_value(
            json!({ "data": d, "errors": [{ "message": "no", "path": ["channel", "unbanRequestsSettings"] }] }),
        )
        .unwrap();
        assert!(answer(res).is_ok());
    }

    #[test]
    fn cooldown_saturates() {
        let s = standing_of(&json!({ "channel": { "unbanRequestsSettings": { "isEnabled": true, "cooldownMinutes": 9_999_999_999u64 } } }));
        assert_eq!(s.unban.unwrap().cooldown_minutes, u32::MAX);
    }

    #[test]
    fn topic_changes() {
        let ban = json!({ "type": "user_moderation_action", "data": { "action": "ban", "channel_id": "9", "target_id": "1" } });
        assert_eq!(change_of(&ban), Some(("9".into(), Change::Moved)));
        let warn = json!({ "type": "user_moderation_action",
            "data": { "action": "warn", "channel_id": "9", "reason": "be nice", "chat_rules_cited": ["No spam"] } });
        assert_eq!(
            change_of(&warn),
            Some(("9".into(), Change::Warned(Warning { reason: "be nice".into(), rules: vec!["No spam".into()] })))
        );
        let ack = json!({ "type": "warning_acknowledged", "data": { "channel_id": "9" } });
        assert_eq!(change_of(&ack), Some(("9".into(), Change::Moved)), "an acknowledgement isn't a new warning");
        assert_eq!(change_of(&json!({ "type": "x", "data": {} })), None);
        assert_eq!(topic("42"), "chatrooms-user-v1.42");
    }
}
