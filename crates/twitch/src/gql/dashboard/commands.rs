//! V111-50 GQL twins of three Helix chat commands (/announce, /shoutout, /shield), the way
//! twitch.tv's own chat runs them. Read 2026/11/17 from Twitch's public web bundle (chunk
//! `features.chat.components.chat-command-handlers.component-1d29641d292e88cd37ac.js`, enums
//! in `95120-9f602c40a37651b59528.js`); docs/research/twitch-api.md "V111-50". Never run
//! against Twitch: mock-tested only. Twitch's web client has no GQL /clear (it sends the text
//! over IRC), so /clear has no twin.

use serde_json::{Value, json};

use super::channel::write;
use crate::gql::Auth;
use crate::{Error, Http, Result};

/// Twitch's `SendAnnouncementMessage` (module 842246; handler 405963 sends
/// `{channelID, message, color}`). `error` is a bare enum: `UNAUTHORIZED`,
/// `FAILED_MESSAGE_ENFORCEMENTS`, `RATE_LIMITED`, `UNKNOWN`.
const ANNOUNCE: &str = "mutation SendAnnouncementMessage($input: SendAnnouncementMessageInput!) {
  sendAnnouncementMessage(input: $input) { error }
}";

/// Twitch's `createShoutout` (module 857373; handler 733770 sends logins, not ids:
/// `{channelLogin, callerLogin, targetLogin}`). `error` is a bare enum: `ALREADY_SHOUTING_OUT`,
/// `CANNOT_SHOUTOUT_BROADCASTER`, `CANNOT_SHOUTOUT_CHANNEL`, `INVALID_BROADCASTER`,
/// `INVALID_SOURCE_CHANNEL`, `INVALID_TARGET_CHANNEL`, `MAX_SHOUTOUTS_REACHED`,
/// `OFFLINE_BROADCASTER`, `PERMISSION_DENIED`.
const SHOUTOUT: &str = "mutation createShoutout($input: CreateShoutoutInput!) {
  createShoutout(input: $input) { error cooldownEndsAt }
}";

/// Twitch's `UpdateShieldModeActive` (module 817881) without its user fragment; input
/// `{channelID, shieldModeStatus: SHIELD|DEFAULT}` (handler 733770). `error` is a bare enum:
/// `ALREADY_SET`, `TOO_FAST`, `UNKNOWN`.
const SHIELD: &str = "mutation UpdateShieldModeActive($input: SetChannelShieldModeStatusInput!) {
  setChannelShieldModeStatus(input: $input) { isActive error }
}";

/// Helix's lowercase announcement colors as Twitch's GQL enum (`PRIMARY BLUE GREEN ORANGE
/// PURPLE`); anything else is the channel's color, as on Helix.
fn color(c: &str) -> &'static str {
    match c {
        "blue" => "BLUE",
        "green" => "GREEN",
        "orange" => "ORANGE",
        "purple" => "PURPLE",
        _ => "PRIMARY",
    }
}

pub async fn announce(http: &Http, auth: Auth<'_>, channel_id: &str, text: &str, c: &str) -> Result<()> {
    announce_at(http, auth, None, channel_id, text, c).await
}

async fn announce_at(http: &Http, auth: Auth<'_>, base: Option<&str>, channel_id: &str, text: &str, c: &str) -> Result<()> {
    let input = json!({ "channelID": channel_id, "message": text, "color": color(c) });
    write(http, auth, base, ANNOUNCE, "sendAnnouncementMessage", input).await.map(drop)
}

/// `channel` gives the shoutout (your own channel, so you are the caller too); `target` gets it.
pub async fn shoutout(http: &Http, auth: Auth<'_>, channel: &str, caller: &str, target: &str) -> Result<()> {
    shoutout_at(http, auth, None, channel, caller, target).await
}

async fn shoutout_at(http: &Http, auth: Auth<'_>, base: Option<&str>, channel: &str, caller: &str, target: &str) -> Result<()> {
    let input = json!({ "channelLogin": channel, "callerLogin": caller, "targetLogin": target });
    write(http, auth, base, SHOUTOUT, "createShoutout", input).await.map(drop)
}

pub async fn set_shield(http: &Http, auth: Auth<'_>, channel_id: &str, on: bool) -> Result<()> {
    shield_at(http, auth, None, channel_id, on).await
}

async fn shield_at(http: &Http, auth: Auth<'_>, base: Option<&str>, channel_id: &str, on: bool) -> Result<()> {
    let input = json!({ "channelID": channel_id, "shieldModeStatus": if on { "SHIELD" } else { "DEFAULT" } });
    already_set_is_done(write(http, auth, base, SHIELD, "setChannelShieldModeStatus", input).await)
}

/// Twitch's own hook (module 108284) counts `ALREADY_SET` as done: shield mode is where it
/// was asked to be.
fn already_set_is_done(res: Result<Value>) -> Result<()> {
    match res {
        Err(Error::Refused(code)) if code == "ALREADY_SET" => Ok(()),
        r => r.map(drop),
    }
}

#[cfg(test)]
mod tests {
    use super::super::NOT_DONE;
    use super::super::mock::{AUTH, query, serve};
    use super::*;

    #[test]
    fn v111_50_colors_map_to_twitchs_enum() {
        assert_eq!(color("blue"), "BLUE");
        assert_eq!(color("purple"), "PURPLE");
        assert_eq!(color("primary"), "PRIMARY");
        assert_eq!(color("pink"), "PRIMARY");
    }

    #[tokio::test]
    async fn v111_50_announce_sends_twitchs_input_and_reads_the_bare_error() {
        let m = serve([
            json!({ "data": { "sendAnnouncementMessage": { "error": null } } }).to_string(),
            json!({ "data": { "sendAnnouncementMessage": { "error": "RATE_LIMITED" } } }).to_string(),
        ]);
        announce_at(&m.http, AUTH, Some(&m.base), "9", "hi all", "green").await.unwrap();
        let err = announce_at(&m.http, AUTH, Some(&m.base), "9", "again", "").await;
        assert!(matches!(err, Err(Error::Refused(c)) if c == "RATE_LIMITED"));
        let sent = m.sent();
        assert!(query(&sent[0]).contains("sendAnnouncementMessage(input: $input) { error }"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "channelID": "9", "message": "hi all", "color": "GREEN" } }));
        assert_eq!(sent[1]["variables"]["input"]["color"], "PRIMARY");
    }

    #[tokio::test]
    async fn v111_50_shoutout_goes_by_login_and_reads_the_cooldown_code() {
        let m = serve([
            json!({ "data": { "createShoutout": { "error": null, "cooldownEndsAt": null } } }).to_string(),
            json!({ "data": { "createShoutout": { "error": "ALREADY_SHOUTING_OUT", "cooldownEndsAt": "2026-11-17T10:00:00Z" } } })
                .to_string(),
            json!({ "data": { "createShoutout": null } }).to_string(),
        ]);
        shoutout_at(&m.http, AUTH, Some(&m.base), "me", "me", "friend").await.unwrap();
        let err = shoutout_at(&m.http, AUTH, Some(&m.base), "me", "me", "friend").await;
        assert!(matches!(err, Err(Error::Refused(c)) if c == "ALREADY_SHOUTING_OUT"));
        let gone = shoutout_at(&m.http, AUTH, Some(&m.base), "me", "me", "friend").await;
        assert!(matches!(gone, Err(Error::Refused(c)) if c == NOT_DONE));
        let sent = m.sent();
        assert!(query(&sent[0]).contains("createShoutout(input: $input)"));
        assert_eq!(
            sent[0]["variables"],
            json!({ "input": { "channelLogin": "me", "callerLogin": "me", "targetLogin": "friend" } })
        );
    }

    #[tokio::test]
    async fn v111_50_shield_on_off_and_already_set_counts_as_done() {
        let m = serve([
            json!({ "data": { "setChannelShieldModeStatus": { "isActive": true, "error": null } } }).to_string(),
            json!({ "data": { "setChannelShieldModeStatus": { "isActive": false, "error": "ALREADY_SET" } } }).to_string(),
            json!({ "data": { "setChannelShieldModeStatus": { "isActive": true, "error": "TOO_FAST" } } }).to_string(),
        ]);
        shield_at(&m.http, AUTH, Some(&m.base), "9", true).await.unwrap();
        shield_at(&m.http, AUTH, Some(&m.base), "9", false).await.unwrap();
        let err = shield_at(&m.http, AUTH, Some(&m.base), "9", false).await;
        assert!(matches!(err, Err(Error::Refused(c)) if c == "TOO_FAST"));
        let sent = m.sent();
        assert!(query(&sent[0]).contains("setChannelShieldModeStatus(input: $input) { isActive error }"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "channelID": "9", "shieldModeStatus": "SHIELD" } }));
        assert_eq!(sent[1]["variables"]["input"]["shieldModeStatus"], "DEFAULT");
    }
}
