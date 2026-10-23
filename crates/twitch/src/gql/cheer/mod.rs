//! V107-Bn Cheering: a channel's Bits rules with your balance, and `sendCheer` (research
//! "Cheering", 2026/10/22). Spends real Bits. Raw text only, no persisted hash. `base` is a
//! local mock's address in tests and mocks builds, else Twitch.

pub mod check;
pub mod codes;

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, run_at};
use crate::{Http, Result};

/// Global and channel cheermotes with their type, the channel's rules, your balance.
const RULES: &str = "query($login: String!) { currentUser { id bitsBalance } \
    cheerConfig { groups { nodes { prefix type } } } \
    channel: user(login: $login) { id cheer { id isBitsEnabled settings { id cheerMinimumBits emoteMinimumBits } \
    cheerGroups { nodes { prefix type } } } } }";

/// The web chat input's own document.
const SEND: &str = "mutation ChatInput_SendCheer($input: SendCheerInput!) { sendCheer(input: $input) { id \
    currentUser { id bitsBalance } validationError { code message messageContent { text fragments { text content { \
    ... on AutoMod { topics { type weight } } ... on CheermoteToken { bitsAmount prefix tier } \
    ... on Emote { emoteID: id setID token } ... on User { id login displayName } } } } } } }";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheerPrefix {
    /// Lowercase.
    pub prefix: String,
    /// DEFAULT, CUSTOM, DISPLAY_ONLY, ANONYMOUS…
    pub kind: String,
}

/// What a cheer in one channel must meet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheerRules {
    pub channel_id: String,
    pub enabled: bool,
    pub cheer_min: u32,
    pub emote_min: u32,
    /// The channel's own first, then global ones; each prefix once.
    pub prefixes: Vec<CheerPrefix>,
    /// With a sign-in.
    pub balance: Option<u64>,
}

impl CheerRules {
    /// Prefixes a cheer may use (lowercase): not display-only, not anonymous.
    pub fn usable(&self) -> Vec<String> {
        self.prefixes
            .iter()
            .filter(|p| p.prefix != "anon" && !matches!(p.kind.as_str(), "DISPLAY_ONLY" | "ANONYMOUS"))
            .map(|p| p.prefix.clone())
            .collect()
    }
}

/// One cheer to send. `id` is the idempotency id: the same for a retry of the same cheer.
pub struct SendCheer<'a> {
    pub id: &'a str,
    pub channel_id: &'a str,
    pub bits: u32,
    pub content: &'a str,
}

/// Twitch's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheerReply {
    pub balance: Option<u64>,
    /// `validationError.code`, when Twitch said no.
    pub refused: Option<String>,
}

/// A fresh idempotency id.
pub fn new_id() -> String {
    crate::random::uuid()
}

/// `None` when the channel doesn't exist or has no Bits settings.
pub async fn rules(http: &Http, auth: Auth<'_>, base: Option<&str>, login: &str) -> Result<Option<CheerRules>> {
    let v = run_at(http, auth, base, RULES, json!({ "login": login })).await?;
    Ok(parse_rules(v))
}

fn parse_rules(v: Value) -> Option<CheerRules> {
    let d: RawRules = serde_json::from_value(v).ok()?;
    let channel = d.channel?;
    let cheer = channel.cheer?;
    let settings = cheer.settings.unwrap_or_default();
    let mut prefixes: Vec<CheerPrefix> = Vec::new();
    let groups = cheer.cheer_groups.into_iter().chain(d.cheer_config.map(|c| c.groups).unwrap_or_default());
    for n in groups.flat_map(|g| g.nodes) {
        let prefix = n.prefix.to_ascii_lowercase();
        if !prefixes.iter().any(|p| p.prefix == prefix) {
            prefixes.push(CheerPrefix { prefix, kind: n.kind });
        }
    }
    Some(CheerRules {
        channel_id: channel.id,
        enabled: cheer.is_bits_enabled,
        cheer_min: settings.cheer_minimum_bits,
        emote_min: settings.emote_minimum_bits,
        prefixes,
        balance: d.current_user.and_then(|u| u.bits_balance),
    })
}

/// Sends one cheer. Spends Bits. `Err` when the outcome is unknown (a retry reuses `c.id`).
pub async fn send(http: &Http, auth: Auth<'_>, base: Option<&str>, c: &SendCheer<'_>) -> Result<CheerReply> {
    let input = json!({
        "id": c.id,
        "targetID": c.channel_id,
        "bits": c.bits,
        "content": c.content,
        "isAutoModEnabled": true,
        "shouldCheerAnyway": false,
    });
    let v = run_at(http, auth, base, SEND, json!({ "input": input })).await?;
    Ok(parse_reply(&v))
}

fn parse_reply(v: &Value) -> CheerReply {
    let s = v.get("sendCheer");
    let refused = s
        .and_then(|s| s.pointer("/validationError/code"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    // No payload at all: Twitch didn't say it worked.
    let refused = refused.or_else(|| s.is_none_or(Value::is_null).then(|| codes::UNKNOWN.to_owned()));
    CheerReply {
        balance: s.and_then(|s| s.pointer("/currentUser/bitsBalance")).and_then(Value::as_u64),
        refused,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRules {
    current_user: Option<RawMe>,
    cheer_config: Option<RawConfig>,
    channel: Option<RawChannel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMe {
    bits_balance: Option<u64>,
}

#[derive(Deserialize)]
struct RawConfig {
    #[serde(default)]
    groups: Vec<RawGroup>,
}

#[derive(Deserialize)]
struct RawGroup {
    #[serde(default)]
    nodes: Vec<RawNode>,
}

#[derive(Deserialize)]
struct RawNode {
    prefix: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Deserialize)]
struct RawChannel {
    id: String,
    cheer: Option<RawCheer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheer {
    #[serde(default)]
    is_bits_enabled: bool,
    settings: Option<RawSettings>,
    #[serde(default)]
    cheer_groups: Vec<RawGroup>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSettings {
    #[serde(default)]
    cheer_minimum_bits: u32,
    #[serde(default)]
    emote_minimum_bits: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_bn_rules_from_a_real_shape() {
        // Shape as read signed out for xqc on 2026/10/22 (trimmed).
        let v: Value = serde_json::from_str(
            r#"{"currentUser":{"id":"9","bitsBalance":1234},"cheerConfig":{"groups":[{"nodes":[
            {"prefix":"Cheer","type":"DEFAULT"},{"prefix":"Scoops","type":"DISPLAY_ONLY"},{"prefix":"4Head","type":"FIRST_PARTY"},
            {"prefix":"Anon","type":"DISPLAY_ONLY"},{"prefix":"Charity","type":"FIRST_PARTY"}]}]},
            "channel":{"id":"71092938","cheer":{"id":"71092938","isBitsEnabled":true,
            "settings":{"id":"71092938","cheerMinimumBits":200,"emoteMinimumBits":1},
            "cheerGroups":[{"nodes":[{"prefix":"Charity","type":"DISPLAY_ONLY"}]}]}}}"#,
        )
        .unwrap();
        let r = parse_rules(v).unwrap();
        assert_eq!(r.channel_id, "71092938");
        assert!(r.enabled);
        assert_eq!((r.cheer_min, r.emote_min, r.balance), (200, 1, Some(1234)));
        // The channel's entry wins.
        assert_eq!(r.prefixes[0], CheerPrefix { prefix: "charity".into(), kind: "DISPLAY_ONLY".into() });
        assert_eq!(r.prefixes.iter().filter(|p| p.prefix == "charity").count(), 1);
        assert_eq!(r.usable(), ["cheer", "4head"]);
    }

    #[test]
    fn v107_bn_rules_signed_out_or_no_bits() {
        let v = json!({ "currentUser": null, "cheerConfig": null, "channel": { "id": "1", "cheer": null } });
        assert_eq!(parse_rules(v), None);
        let v = json!({ "currentUser": null, "channel": { "id": "1", "cheer": { "isBitsEnabled": false } } });
        let r = parse_rules(v).unwrap();
        assert!(!r.enabled && r.balance.is_none() && r.prefixes.is_empty());
    }

    #[test]
    fn v107_bn_reply_balance_and_code() {
        let ok = json!({ "sendCheer": { "id": "x", "currentUser": { "id": "9", "bitsBalance": 900 }, "validationError": null } });
        assert_eq!(parse_reply(&ok), CheerReply { balance: Some(900), refused: None });
        let no = json!({ "sendCheer": { "id": null, "currentUser": { "id": "9", "bitsBalance": 50 },
            "validationError": { "code": "INSUFFICIENT_BALANCE", "message": "", "messageContent": null } } });
        assert_eq!(parse_reply(&no).refused.as_deref(), Some("INSUFFICIENT_BALANCE"));
        assert_eq!(parse_reply(&json!({ "sendCheer": null })).refused.as_deref(), Some("UNKNOWN"));
    }

    #[tokio::test]
    async fn v107_bn_send_wire_and_failures() {
        use crate::gql::dashboard::mock;
        let m = mock::serve([
            r#"{"data":{"sendCheer":{"id":"x","currentUser":{"id":"9","bitsBalance":900},"validationError":null}}}"#,
            r#"{"data":{"sendCheer":null}}"#,
            r#"{"errors":[{"message":"service error"}],"data":null}"#,
            r#"{"data":{"currentUser":null,"channel":{"id":"1","cheer":{"isBitsEnabled":false}}}}"#,
        ]);
        let base = Some(m.base.as_str());
        let (h, a) = (&m.http, mock::AUTH);
        let c = SendCheer { id: "abc", channel_id: "71", bits: 100, content: "Cheer100 gg" };
        let ok = send(h, a, base, &c).await.unwrap();
        assert_eq!(ok, CheerReply { balance: Some(900), refused: None });
        let none = send(h, a, base, &c).await.unwrap();
        assert_eq!(none.refused.as_deref(), Some("UNKNOWN"), "no payload: not a success");
        assert!(send(h, a, base, &c).await.is_err(), "a GQL error is an unknown outcome");
        assert!(rules(h, a, base, "someone").await.unwrap().is_some());
        let sent = m.sent();
        assert!(mock::query(&sent[0]).contains("sendCheer(input: $input)"));
        assert_eq!(
            sent[0]["variables"],
            json!({ "input": { "id": "abc", "targetID": "71", "bits": 100, "content": "Cheer100 gg",
                "isAutoModEnabled": true, "shouldCheerAnyway": false } })
        );
        assert_eq!(sent[3]["variables"], json!({ "login": "someone" }));
        // Nothing listening: the outcome is unknown, never a refusal.
        let off = mock::off();
        assert!(send(off.http, off.auth, off.base, &c).await.is_err());
    }
}
