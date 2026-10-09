//! Channel points as a viewer (1.9): rewards, balance, redeeming. Redeeming needs the
//! account's GQL token.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, persisted, query};
use crate::random::hex;
use crate::{Http, Result};

pub(crate) const SETTINGS_FIELDS: &str = "name isEnabled image { url url2x } \
    customRewards { id title prompt cost backgroundColor isEnabled isPaused isInStock isSubOnly \
      isUserInputRequired cooldownExpiresAt image { url url2x } defaultImage { url url2x } } \
    automaticRewards { id type cost defaultCost pricingType backgroundColor defaultBackgroundColor \
      isEnabled isHiddenForSubs cooldownExpiresAt image { url url2x } defaultImage { url url2x } }";

const BALANCE: &str =
    "query($login: String!) { channel(name: $login) { self { communityPoints { balance } } } }";
const REDEEM_CUSTOM: (&str, &str) = (
    "RedeemCustomReward",
    "d56249a7adb4978898ea3412e196688d4ac3cea1c0c2dfd65561d229ea5dcc42",
);
const HIGHLIGHT: (&str, &str) = (
    "SendHighlightedChatMessage",
    "bb187d763156dc5c25c6457e1b32da6c5033cb7504854e6d33a8b876d10444b6",
);
const RANDOM_EMOTE: (&str, &str) = (
    "UnlockRandomSubscriberEmote",
    "f548e89966b21d0094f3dc35233232eb6ec76d63e02594c8a494407712a85350",
);
const SUB_MODE: &str = "mutation($input: SendChatMessageThroughSubscriberModeInput!) { \
    sendChatMessageThroughSubscriberMode(input: $input) { balance error { code } } }";

/// A channel's points setup. `balance` is filled in when signed in.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Points {
    /// Custom name, e.g. "Poki Points". `None` = "Channel Points".
    pub name: Option<String>,
    /// Custom icon. `None` = the default one.
    pub icon: Option<String>,
    pub enabled: bool,
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
    pub custom: Vec<CustomReward>,
    pub automatic: Vec<AutoReward>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CustomReward {
    pub id: String,
    pub title: String,
    pub prompt: Option<String>,
    pub cost: u32,
    pub color: String,
    pub image: Option<String>,
    pub input_required: bool,
    pub sub_only: bool,
    /// Enabled, not paused, in stock.
    pub available: bool,
    pub cooldown_until: Option<String>,
}

/// Twitch's built-in rewards: `SEND_HIGHLIGHTED_MESSAGE`, `SINGLE_MESSAGE_BYPASS_SUB_MODE`,
/// `RANDOM_SUB_EMOTE_UNLOCK`, `CHOSEN_SUB_EMOTE_UNLOCK`, `CHOSEN_MODIFIED_SUB_EMOTE_UNLOCK`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AutoReward {
    pub id: String,
    pub kind: String,
    pub cost: u32,
    pub color: String,
    pub image: Option<String>,
    pub available: bool,
    pub hidden_for_subs: bool,
    pub cooldown_until: Option<String>,
}

/// What Twitch said after a redeem. `error` is Twitch's code, e.g. `NOT_ENOUGH_POINTS`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Redeemed {
    pub error: Option<String>,
    pub balance: Option<u64>,
    /// Unlocked emote's name (random emote reward).
    pub emote: Option<String>,
}

pub async fn balance(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<u64>> {
    let v: Value = query(http, auth, BALANCE, json!({ "login": login })).await?;
    Ok(v.pointer("/channel/self/communityPoints/balance").and_then(Value::as_u64))
}

pub async fn redeem_custom(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    reward: &CustomReward,
    text: Option<&str>,
) -> Result<Redeemed> {
    let mut input = json!({
        "channelID": channel_id,
        "cost": reward.cost,
        "pricingType": "POINTS",
        "prompt": reward.prompt,
        "rewardID": reward.id,
        "title": reward.title,
        "transactionID": hex(16),
    });
    if let Some(t) = text.filter(|t| !t.is_empty()) {
        input["textInput"] = json!(t);
    }
    let v: Value = persisted(http, auth, REDEEM_CUSTOM.0, REDEEM_CUSTOM.1, json!({ "input": input })).await?;
    Ok(outcome(&v, "redeemCommunityPointsCustomReward"))
}

/// "Highlight my message": Twitch posts the message itself.
pub async fn send_highlighted(http: &Http, auth: Auth<'_>, channel_id: &str, cost: u32, message: &str) -> Result<Redeemed> {
    let input = message_input(channel_id, cost, message);
    let v: Value = persisted(http, auth, HIGHLIGHT.0, HIGHLIGHT.1, json!({ "input": input })).await?;
    Ok(outcome(&v, "sendHighlightedChatMessage"))
}

/// "Send a message in sub-only mode": Twitch posts the message itself.
pub async fn send_in_sub_mode(http: &Http, auth: Auth<'_>, channel_id: &str, cost: u32, message: &str) -> Result<Redeemed> {
    let input = message_input(channel_id, cost, message);
    let v: Value = query(http, auth, SUB_MODE, json!({ "input": input })).await?;
    Ok(outcome(&v, "sendChatMessageThroughSubscriberMode"))
}

pub async fn unlock_random_emote(http: &Http, auth: Auth<'_>, channel_id: &str, cost: u32) -> Result<Redeemed> {
    let input = json!({ "channelID": channel_id, "cost": cost, "transactionID": hex(16) });
    let v: Value = persisted(http, auth, RANDOM_EMOTE.0, RANDOM_EMOTE.1, json!({ "input": input })).await?;
    Ok(outcome(&v, "unlockRandomSubscriberEmote"))
}

fn message_input(channel_id: &str, cost: u32, message: &str) -> Value {
    json!({ "channelID": channel_id, "cost": cost, "message": message, "transactionID": hex(16) })
}

fn outcome(v: &Value, field: &str) -> Redeemed {
    let r = &v[field];
    Redeemed {
        error: r
            .pointer("/error/code")
            .and_then(Value::as_str)
            .map(str::to_owned),
        balance: r["balance"].as_u64(),
        emote: r.pointer("/emote/token").and_then(Value::as_str).map(str::to_owned),
    }
}

// Raw shapes.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawPointsSettings {
    name: Option<String>,
    #[serde(default)]
    is_enabled: bool,
    image: Option<RawImage>,
    #[serde(default)]
    custom_rewards: Vec<RawCustom>,
    #[serde(default)]
    automatic_rewards: Vec<RawAuto>,
}

#[derive(Deserialize)]
struct RawImage {
    url: Option<String>,
    url2x: Option<String>,
}

impl RawImage {
    fn best(self) -> Option<String> {
        self.url2x.or(self.url)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCustom {
    id: String,
    title: String,
    prompt: Option<String>,
    cost: u32,
    background_color: Option<String>,
    is_enabled: bool,
    is_paused: bool,
    is_in_stock: bool,
    #[serde(default)]
    is_sub_only: bool,
    is_user_input_required: bool,
    cooldown_expires_at: Option<String>,
    image: Option<RawImage>,
    default_image: Option<RawImage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAuto {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    cost: Option<u32>,
    default_cost: Option<u32>,
    pricing_type: Option<String>,
    background_color: Option<String>,
    default_background_color: Option<String>,
    is_enabled: bool,
    #[serde(default)]
    is_hidden_for_subs: bool,
    cooldown_expires_at: Option<String>,
    image: Option<RawImage>,
    default_image: Option<RawImage>,
}

impl From<RawPointsSettings> for Points {
    fn from(s: RawPointsSettings) -> Self {
        Self {
            name: s.name.filter(|n| !n.is_empty()),
            icon: s.image.and_then(RawImage::best),
            enabled: s.is_enabled,
            balance: None,
            custom: s
                .custom_rewards
                .into_iter()
                .map(|r| CustomReward {
                    available: r.is_enabled && !r.is_paused && r.is_in_stock,
                    id: r.id,
                    title: r.title,
                    prompt: r.prompt.filter(|p| !p.is_empty()),
                    cost: r.cost,
                    color: r.background_color.unwrap_or_else(|| "#9147ff".into()),
                    image: r.image.and_then(RawImage::best).or_else(|| r.default_image.and_then(RawImage::best)),
                    input_required: r.is_user_input_required,
                    sub_only: r.is_sub_only,
                    cooldown_until: r.cooldown_expires_at,
                })
                .collect(),
            automatic: s
                .automatic_rewards
                .into_iter()
                // Bits-priced ones are power-ups (1.42), not points rewards.
                .filter(|r| r.pricing_type.as_deref().is_none_or(|p| p == "POINTS"))
                .map(|r| AutoReward {
                    id: r.id,
                    kind: r.kind,
                    cost: r.cost.or(r.default_cost).unwrap_or(0),
                    color: r
                        .background_color
                        .or(r.default_background_color)
                        .unwrap_or_else(|| "#9147ff".into()),
                    image: r.image.and_then(RawImage::best).or_else(|| r.default_image.and_then(RawImage::best)),
                    available: r.is_enabled,
                    hidden_for_subs: r.is_hidden_for_subs,
                    cooldown_until: r.cooldown_expires_at,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse_with_defaults() {
        let raw: RawPointsSettings = serde_json::from_str(
            r##"{"name":null,"isEnabled":true,"image":null,
              "customRewards":[{"id":"r1","title":"Hydrate","prompt":"","cost":500,"backgroundColor":"#00C7AC",
                "isEnabled":true,"isPaused":false,"isInStock":true,"isSubOnly":false,"isUserInputRequired":false,
                "cooldownExpiresAt":null,"image":null,"defaultImage":{"url":"d1","url2x":"d2"}}],
              "automaticRewards":[
                {"id":"1:SEND_HIGHLIGHTED_MESSAGE","type":"SEND_HIGHLIGHTED_MESSAGE","cost":null,"defaultCost":100,
                 "pricingType":"POINTS","backgroundColor":null,"defaultBackgroundColor":"#FF6905","isEnabled":true,
                 "isHiddenForSubs":false,"cooldownExpiresAt":null,"image":null,"defaultImage":null},
                {"id":"1:SEND_GIGANTIFIED_EMOTE","type":"SEND_GIGANTIFIED_EMOTE","cost":50,"defaultCost":50,
                 "pricingType":"BITS","backgroundColor":null,"defaultBackgroundColor":"#000","isEnabled":true,
                 "isHiddenForSubs":false,"cooldownExpiresAt":null,"image":null,"defaultImage":null}]}"##,
        )
        .unwrap();
        let p = Points::from(raw);
        assert_eq!(p.name, None);
        assert_eq!(p.custom[0].prompt, None);
        assert_eq!(p.custom[0].image.as_deref(), Some("d2"));
        assert_eq!(p.automatic.len(), 1);
        assert_eq!((p.automatic[0].cost, p.automatic[0].color.as_str()), (100, "#FF6905"));
    }

    #[test]
    fn redeem_outcomes() {
        let v = json!({ "redeemCommunityPointsCustomReward": { "error": { "code": "NOT_ENOUGH_POINTS" } } });
        assert_eq!(outcome(&v, "redeemCommunityPointsCustomReward").error.as_deref(), Some("NOT_ENOUGH_POINTS"));
        let v = json!({ "unlockRandomSubscriberEmote": { "error": null, "balance": 900, "emote": { "id": "1", "token": "abcHi" } } });
        let o = outcome(&v, "unlockRandomSubscriberEmote");
        assert_eq!((o.error, o.balance, o.emote.as_deref()), (None, Some(900), Some("abcHi")));
    }
}
