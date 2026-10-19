//! Mod actions over GQL, for accounts without a Helix sign-in (e.g. a GQL-only account).
//! Raw mutations, checked against Twitch's schema (research, phase 10). Each returns the
//! GQL error code, if any, so the app can word it. The ban mutation is also batched for
//! the GQL massban (1.55).

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, BATCH_MAX, batch, query};
use crate::helix::moderation::PIN_SECS;
use crate::{Error, Http, Result};

/// A ban, or a timeout when `expires_in` is set (`"10m"`, `"1h"`, `"1d"`, up to `"14d"`).
pub const BAN: &str = "mutation Ban($input: BanUserFromChatRoomInput!) {
  banUserFromChatRoom(input: $input) { error { code } }
}";

fn ban_vars(channel_id: &str, login: &str, expires_in: Option<&str>, reason: &str) -> Value {
    json!({ "input": {
        "channelID": channel_id,
        "bannedUserLogin": login,
        "expiresIn": expires_in.unwrap_or(""),
        "reason": reason,
    }})
}

#[derive(Deserialize)]
struct ErrorField {
    error: Option<Code>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

/// The error code from a mutation whose payload is `{ <field>: { error { code } } }`.
fn code(data: &Value, field: &str) -> Option<String> {
    serde_json::from_value::<ErrorField>(data.get(field)?.clone())
        .ok()
        .and_then(|f| f.error)
        .map(|c| c.code)
}

/// Bans or times out one user. `Ok(None)` on success, `Ok(Some(code))` when Twitch
/// refused (e.g. `TARGET_IS_MOD`, `TARGET_ALREADY_BANNED`).
pub async fn ban(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    login: &str,
    expires_in: Option<&str>,
    reason: &str,
) -> Result<Option<String>> {
    let data: Value = query(http, auth, BAN, ban_vars(channel_id, login, expires_in, reason)).await?;
    Ok(code(&data, "banUserFromChatRoom"))
}

/// 1.55 One ban per login, batched `BATCH_MAX` at a time. Returns each login with the
/// error code, if any; `None` means it went through.
pub async fn ban_many(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    logins: &[String],
    expires_in: Option<&str>,
    reason: &str,
) -> Result<Vec<(String, Option<String>)>> {
    let mut out = Vec::with_capacity(logins.len());
    for chunk in logins.chunks(BATCH_MAX) {
        let ops: Vec<(&str, Value)> = chunk
            .iter()
            .map(|l| (BAN, ban_vars(channel_id, l, expires_in, reason)))
            .collect();
        let results = batch(http, auth, &ops).await?;
        for (login, res) in chunk.iter().zip(results) {
            let code = match res {
                Ok(data) => code(&data, "banUserFromChatRoom"),
                Err(e) => Some(e.api_message().unwrap_or("error").to_owned()),
            };
            out.push((login.clone(), code));
        }
    }
    Ok(out)
}

pub async fn unban(http: &Http, auth: Auth<'_>, channel_id: &str, login: &str) -> Result<Option<String>> {
    const Q: &str = "mutation Unban($input: UnbanUserFromChatRoomInput!) {
      unbanUserFromChatRoom(input: $input) { error { code } }
    }";
    let vars = json!({ "input": { "channelID": channel_id, "bannedUserLogin": login } });
    let data: Value = query(http, auth, Q, vars).await?;
    Ok(code(&data, "unbanUserFromChatRoom"))
}

pub async fn delete_message(http: &Http, auth: Auth<'_>, channel_id: &str, message_id: &str) -> Result<Option<String>> {
    const Q: &str = "mutation Del($input: DeleteChatMessageInput!) {
      deleteChatMessage(input: $input) { responseCode }
    }";
    let vars = json!({ "input": { "channelID": channel_id, "messageID": message_id } });
    let data: Value = query(http, auth, Q, vars).await?;
    let ok = data
        .get("deleteChatMessage")
        .and_then(|d| d.get("responseCode"))
        .and_then(Value::as_str)
        .is_some_and(|c| c.starts_with("SUCCESS"));
    Ok((!ok).then(|| {
        data.get("deleteChatMessage")
            .and_then(|d| d.get("responseCode"))
            .and_then(Value::as_str)
            .unwrap_or("FORBIDDEN")
            .to_owned()
    }))
}

pub async fn warn(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    login: &str,
    reason: &str,
    rules: &[String],
) -> Result<Option<String>> {
    const Q: &str = "mutation Warn($input: WarnUserInChatRoomInput!) {
      warnUserInChatRoom(input: $input) { error { code } }
    }";
    let vars = json!({ "input": {
        "channelID": channel_id,
        "targetUserLogin": login,
        "reason": reason,
        "chatRulesCited": rules,
    }});
    let data: Value = query(http, auth, Q, vars).await?;
    Ok(code(&data, "warnUserInChatRoom"))
}

/// 1.56 Lets a held message through, or drops it.
pub async fn resolve_automod(http: &Http, auth: Auth<'_>, id: &str, allow: bool) -> Result<()> {
    let q = if allow {
        "mutation A($input: AllowRejectedChatMessageInput!) { allowRejectedChatMessage(input: $input) { message { id } } }"
    } else {
        "mutation D($input: DenyRejectedChatMessageInput!) { denyRejectedChatMessage(input: $input) { message { id } } }"
    };
    let _: Value = query(http, auth, q, json!({ "input": { "id": id } })).await?;
    Ok(())
}

/// M3 `ACTIVE_MONITORING`, `RESTRICTED`, or `NONE` to end it.
pub async fn treat_low_trust(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str, treatment: &str) -> Result<()> {
    const Q: &str = "mutation T($input: UpdateLowTrustUserTreatmentInput!) {
      updateLowTrustUserTreatment(input: $input) { error }
    }";
    let vars = json!({ "input": { "channelID": channel_id, "targetID": target_id, "treatment": treatment } });
    let _: Value = query(http, auth, Q, vars).await?;
    Ok(())
}

/// 1.50 Grants or revokes mod or VIP. A lead mod may use these where Helix refuses.
pub async fn set_role(http: &Http, auth: Auth<'_>, channel_id: &str, login: &str, mod_role: bool, on: bool) -> Result<Option<String>> {
    let (op, field, grantee) = match (mod_role, on) {
        (true, true) => ("mutation M($i: ModUserInput!){ modUser(input:$i){ error{code} } }", "modUser", "targetLogin"),
        (true, false) => ("mutation M($i: UnmodUserInput!){ unmodUser(input:$i){ error{code} } }", "unmodUser", "targetLogin"),
        (false, true) => ("mutation V($i: GrantVIPInput!){ grantVIP(input:$i){ error{code} } }", "grantVIP", "granteeLogin"),
        (false, false) => ("mutation V($i: RevokeVIPInput!){ revokeVIP(input:$i){ error{code} } }", "revokeVIP", "revokeeLogin"),
    };
    let vars = json!({ "i": { "channelID": channel_id, grantee: login } });
    let data: Value = query(http, auth, op, vars).await?;
    Ok(code(&data, field))
}

/// V101-34 Pins a chat message for `secs` (clamped to [`PIN_SECS`]), or until the stream ends.
/// Schema-checked 2026/10/11; the web client and the 7TV extension send `type: MOD`.
pub async fn pin(http: &Http, auth: Auth<'_>, channel_id: &str, message_id: &str, secs: Option<u32>) -> Result<Option<String>> {
    const Q: &str = "mutation P($input: PinChatMessageInput!) {
      pinChatMessage(input: $input) { error { code } }
    }";
    let data: Value = query(http, auth, Q, pin_vars(channel_id, message_id, secs)).await?;
    Ok(code(&data, "pinChatMessage"))
}

fn pin_vars(channel_id: &str, message_id: &str, secs: Option<u32>) -> Value {
    let mut input = json!({ "channelID": channel_id, "messageID": message_id, "type": "MOD" });
    if let Some(s) = secs {
        input["durationSeconds"] = json!(s.clamp(PIN_SECS.0, PIN_SECS.1));
    }
    json!({ "input": input })
}

/// M11 Sends a message and pins it, as Twitch's own `/pin <text>` does (schema-checked
/// 2026/10/11). The pin's id; [`Error::Refused`] with Twitch's code when it says no.
pub async fn send_pinned(http: &Http, auth: Auth<'_>, channel_id: &str, text: &str) -> Result<String> {
    const Q: &str = "mutation S($input: SendPinnedChatMessageInput!) {
      sendPinnedChatMessage(input: $input) { pinnedChatMessage { id } error { code } }
    }";
    let vars = json!({ "input": { "channelID": channel_id, "messageText": text } });
    let data: Value = query(http, auth, Q, vars).await?;
    if let Some(c) = code(&data, "sendPinnedChatMessage") {
        return Err(Error::Refused(c));
    }
    data["sendPinnedChatMessage"]["pinnedChatMessage"]["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Refused("NO_PIN".into()))
}

/// M11 How long a pin stays (clamped to [`PIN_SECS`]). `pin_id` is the pin's own id.
pub async fn update_pin(http: &Http, auth: Auth<'_>, pin_id: &str, secs: u32) -> Result<Option<String>> {
    const Q: &str = "mutation U($input: UpdatePinnedChatMessageInput!) {
      updatePinnedChatMessage(input: $input) { error { code } }
    }";
    let vars = json!({ "input": { "id": pin_id, "durationSeconds": secs.clamp(PIN_SECS.0, PIN_SECS.1) } });
    let data: Value = query(http, auth, Q, vars).await?;
    Ok(code(&data, "updatePinnedChatMessage"))
}

/// V101-34 Takes a pin down. `pin_id` is the pin's own id, not the message's.
pub async fn unpin(http: &Http, auth: Auth<'_>, pin_id: &str) -> Result<Option<String>> {
    const Q: &str = "mutation U($input: UnpinChatMessageInput!) {
      unpinChatMessage(input: $input) { error { code } }
    }";
    let vars = json!({ "input": { "id": pin_id, "reason": "UNPIN" } });
    let data: Value = query(http, auth, Q, vars).await?;
    Ok(code(&data, "unpinChatMessage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ban_vars_shape() {
        let v = ban_vars("1", "spammer", Some("10m"), "bot");
        assert_eq!(v["input"]["channelID"], "1");
        assert_eq!(v["input"]["bannedUserLogin"], "spammer");
        assert_eq!(v["input"]["expiresIn"], "10m");
        // A permanent ban sends an empty string, as the reference does.
        assert_eq!(ban_vars("1", "x", None, "")["input"]["expiresIn"], "");
    }

    #[test]
    fn pin_vars_shape() {
        let v = pin_vars("1", "m", Some(5));
        assert_eq!(v["input"]["type"], "MOD");
        assert_eq!(v["input"]["messageID"], "m");
        assert_eq!(v["input"]["durationSeconds"], 30);
        // Until the stream ends: no duration at all.
        assert!(pin_vars("1", "m", None)["input"].get("durationSeconds").is_none());
    }

    #[test]
    fn reads_the_error_code() {
        let data = json!({ "banUserFromChatRoom": { "error": { "code": "TARGET_IS_MOD" } } });
        assert_eq!(code(&data, "banUserFromChatRoom"), Some("TARGET_IS_MOD".to_owned()));
        let ok = json!({ "banUserFromChatRoom": { "error": null } });
        assert_eq!(code(&ok, "banUserFromChatRoom"), None);
    }
}
