//! V108-C40 The account's own standing in a chat: a ban or timeout on it, whether it may ask
//! for an unban (and its last request), and Twitch's chat warnings. Checked by wrong-typed
//! validation probes 2026/11/04, signed out; neither mutation was ever run.
//! - Read: `chatRoomBanStatus(channelID, userID) { createdAt expiresAt isPermanent reason
//!   moderator }`, `channel(id) { unbanRequestsSettings { isEnabled cooldownMinutes } self {
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
use super::{Auth, query, query_partial};
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
    createdAt expiresAt isPermanent reason moderator { login displayName } }
  channel(id: $c) { unbanRequestsSettings { isEnabled cooldownMinutes } self { canRequestUnban } } }";

/// The standing of `user_id` (the signed-in account) in `channel_id`.
pub async fn read(http: &Http, auth: Auth<'_>, channel_id: &str, user_id: &str) -> Result<Standing> {
    let vars = json!({ "c": channel_id, "u": user_id });
    // Any part may be refused to a viewer; what came back still counts.
    let data: Value = query_partial(http, auth, READ, vars, |_| true).await?;
    let mut s = standing_of(&data);
    if s.ban.as_ref().is_some_and(|b| b.until.is_none()) {
        s.request = modlogs::unban_past(http, auth, channel_id, user_id)
            .await
            .ok()
            .and_then(|l| l.into_iter().next());
    }
    Ok(s)
}

fn standing_of(data: &Value) -> Standing {
    let rules = &data["channel"]["unbanRequestsSettings"];
    Standing {
        ban: modlogs::ban_of(data),
        unban: rules.is_object().then(|| UnbanRules {
            enabled: rules["isEnabled"].as_bool().unwrap_or(false),
            cooldown_minutes: rules["cooldownMinutes"].as_u64().unwrap_or(0) as u32,
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
    if c.contains("ALREADY") || c.contains("PENDING") || c.contains("EXISTS") {
        "You already asked. Wait for an answer."
    } else if c.contains("COOLDOWN") || c.contains("TOO_SOON") || c.contains("RATE") || c.contains("THROTTLE") {
        "Not yet. Try again later."
    } else if c.contains("DISABLED") || c.contains("NOT_ENABLED") {
        "This channel doesn't take unban requests."
    } else if c.contains("LONG") || c.contains("LENGTH") {
        "That message is too long."
    } else if c.contains("NOT_BANNED") || c.contains("NO_WARNING") || c.contains("NOT_FOUND") {
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
        assert_eq!((ban.until, ban.by.as_deref(), ban.reason.as_str()), (None, Some("Kuni"), "spam"));
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
