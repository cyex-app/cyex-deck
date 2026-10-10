//! 1.42 Power-ups, paid with Bits: message effects, gigantified emotes, on-screen
//! celebrations and a channel's own Bits rewards. Prices and the balance are public or
//! the account's; sending needs the GQL sign-in. Mutation inputs checked against the
//! schema without running them (2026/10/10).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, persisted_or_raw, query};
use crate::random::hex;
use crate::{Http, Result};

const READ: &str = "query($login: String!) { channel(name: $login) { id communityPointsSettings { \
    automaticRewards { id type pricingType bitsCost defaultBitsCost isEnabled isInStock cooldownExpiresAt } \
    customRewards { id title prompt pricingType cost bitsCost isEnabled isPaused isInStock isUserInputRequired cooldownExpiresAt } } } \
    currentUser { bitsBalance } }";

const EFFECT: (&str, &str, &str) = (
    "SendAnimatedChatMessage",
    "8d3f422b38452c2d1c2c51700c042ab9c69abf49ffa633d0afab264f2f08fea2",
    "mutation SendAnimatedChatMessage($input: SendAnimatedChatMessageInput!) { sendAnimatedChatMessage(input: $input) { error { code } } }",
);
const GIGANTIFY: (&str, &str, &str) = (
    "SendGigantifiedChatEmote",
    "d2ded7f1509a13ba0ae84b6e5be694a0ebe9fe8011d606afd1417a4cc0752458",
    "mutation SendGigantifiedChatEmote($input: SendGigantifiedChatEmoteInput!) { sendGigantifiedChatEmote(input: $input) { error { code } } }",
);
const CELEBRATION: (&str, &str, &str) = (
    "SendCelebration",
    "fd0cd882274313c360c30052eb90ebb3bda9a32679e4811c967dbcfe5c4aaa8a",
    "mutation SendCelebration($input: SendCelebrationInput!) { sendCelebration(input: $input) { error { code } } }",
);
const CUSTOM: (&str, &str, &str) = (
    "RedeemCustomReward",
    "d56249a7adb4978898ea3412e196688d4ac3cea1c0c2dfd65561d229ea5dcc42",
    "mutation RedeemCustomReward($input: RedeemCommunityPointsCustomRewardInput!) { redeemCommunityPointsCustomReward(input: $input) { error { code } } }",
);

/// The three message effects Twitch has (2026/10/10).
pub const EFFECTS: [&str; 3] = ["simmer", "rainbow-eclipse", "cosmic-abyss"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PowerUps {
    pub channel_id: String,
    pub effect: Option<PowerUp>,
    pub gigantify: Option<PowerUp>,
    pub celebration: Option<PowerUp>,
    pub custom: Vec<BitsReward>,
    /// Your Bits, with a GQL sign-in.
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PowerUp {
    pub bits: u32,
    /// Enabled and in stock.
    pub available: bool,
    /// RFC 3339, while on cooldown.
    pub cooldown_until: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BitsReward {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub bits: u32,
    /// The points-side cost Twitch wants echoed back.
    pub cost: u32,
    pub needs_text: bool,
    pub available: bool,
    pub cooldown_until: Option<String>,
}

pub async fn power_ups(http: &Http, auth: Auth<'_>, login: &str) -> Result<PowerUps> {
    let d: RawData = query(http, auth, READ, json!({ "login": login })).await?;
    let Some(c) = d.channel else {
        return Ok(PowerUps::default());
    };
    let mut out = PowerUps {
        channel_id: c.id,
        balance: d.current_user.and_then(|u| u.bits_balance),
        ..PowerUps::default()
    };
    let settings = c.community_points_settings.unwrap_or_default();
    for r in settings
        .automatic_rewards
        .into_iter()
        .filter(|r| r.pricing_type == "BITS")
    {
        let p = PowerUp {
            bits: r.bits_cost.filter(|b| *b > 0).unwrap_or(r.default_bits_cost),
            available: r.is_enabled && r.is_in_stock,
            cooldown_until: r.cooldown_expires_at,
        };
        match r.kind.as_str() {
            "SEND_ANIMATED_MESSAGE" => out.effect = Some(p),
            "SEND_GIGANTIFIED_EMOTE" => out.gigantify = Some(p),
            "CELEBRATION" => out.celebration = Some(p),
            _ => {}
        }
    }
    out.custom = settings
        .custom_rewards
        .into_iter()
        .filter(|r| r.pricing_type == "BITS" && r.is_enabled)
        .map(|r| BitsReward {
            id: r.id,
            title: r.title,
            prompt: r.prompt.unwrap_or_default(),
            bits: r.bits_cost.unwrap_or(0),
            cost: r.cost,
            needs_text: r.is_user_input_required,
            available: !r.is_paused && r.is_in_stock,
            cooldown_until: r.cooldown_expires_at,
        })
        .collect();
    Ok(out)
}

fn code(v: &Value, field: &str) -> Option<String> {
    v.pointer(&format!("/{field}/error/code"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// A message with an effect. `Some(code)` when Twitch said no.
pub async fn send_effect(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    message: &str,
    effect: &str,
    bits: u32,
) -> Result<Option<String>> {
    let input = json!({ "channelID": channel_id, "message": message, "animationID": effect, "bitsCost": bits, "transactionID": hex(16) });
    let v = persisted_or_raw(http, auth, EFFECT, json!({ "input": input })).await?;
    Ok(code(&v, "sendAnimatedChatMessage"))
}

/// Twitch puts the emote, big, at the end of `message`.
pub async fn send_gigantified(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    emote_id: &str,
    message: &str,
    bits: u32,
) -> Result<Option<String>> {
    let input = json!({ "channelID": channel_id, "emoteID": emote_id, "message": message, "bitsCost": bits, "transactionID": hex(16) });
    let v = persisted_or_raw(http, auth, GIGANTIFY, json!({ "input": input })).await?;
    Ok(code(&v, "sendGigantifiedChatEmote"))
}

pub async fn send_celebration(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    emote_id: &str,
    bits: u32,
) -> Result<Option<String>> {
    let input =
        json!({ "channelID": channel_id, "emoteID": emote_id, "bitsCost": bits, "transactionID": hex(16) });
    let v = persisted_or_raw(http, auth, CELEBRATION, json!({ "input": input })).await?;
    Ok(code(&v, "sendCelebration"))
}

pub async fn redeem_bits_reward(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    r: &BitsReward,
    text: Option<&str>,
) -> Result<Option<String>> {
    let mut input = json!({
        "channelID": channel_id,
        "cost": r.cost,
        "bitsCost": r.bits,
        "pricingType": "BITS",
        "prompt": r.prompt,
        "rewardID": r.id,
        "title": r.title,
        "transactionID": hex(16),
    });
    if let Some(t) = text.filter(|t| !t.is_empty()) {
        input["textInput"] = json!(t);
    }
    let v = persisted_or_raw(http, auth, CUSTOM, json!({ "input": input })).await?;
    Ok(code(&v, "redeemCommunityPointsCustomReward"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawData {
    channel: Option<RawChannel>,
    current_user: Option<RawMe>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMe {
    bits_balance: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    id: String,
    community_points_settings: Option<RawSettings>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSettings {
    #[serde(default)]
    automatic_rewards: Vec<RawAutomatic>,
    #[serde(default)]
    custom_rewards: Vec<RawCustom>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAutomatic {
    #[serde(rename = "type")]
    kind: String,
    pricing_type: String,
    bits_cost: Option<u32>,
    #[serde(default)]
    default_bits_cost: u32,
    is_enabled: bool,
    is_in_stock: bool,
    cooldown_expires_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCustom {
    id: String,
    title: String,
    prompt: Option<String>,
    pricing_type: String,
    #[serde(default)]
    cost: u32,
    bits_cost: Option<u32>,
    is_enabled: bool,
    is_paused: bool,
    is_in_stock: bool,
    is_user_input_required: bool,
    cooldown_expires_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices() {
        let d: RawData = serde_json::from_str(
            r#"{"channel":{"id":"641972806","communityPointsSettings":{"automaticRewards":[
            {"id":"a","type":"CELEBRATION","pricingType":"BITS","bitsCost":10000,"defaultBitsCost":40,"isEnabled":true,"isInStock":true,"cooldownExpiresAt":null},
            {"id":"b","type":"SEND_ANIMATED_MESSAGE","pricingType":"BITS","bitsCost":null,"defaultBitsCost":20,"isEnabled":true,"isInStock":true,"cooldownExpiresAt":null},
            {"id":"c","type":"SEND_HIGHLIGHTED_MESSAGE","pricingType":"POINTS","bitsCost":null,"defaultBitsCost":0,"isEnabled":true,"isInStock":true,"cooldownExpiresAt":null}],
            "customRewards":[{"id":"r","title":"Hydrate","prompt":null,"pricingType":"BITS","cost":0,"bitsCost":100,"isEnabled":true,"isPaused":false,"isInStock":true,"isUserInputRequired":false,"cooldownExpiresAt":null}]}},
            "currentUser":null}"#,
        )
        .unwrap();
        let c = d.channel.unwrap();
        let s = c.community_points_settings.unwrap();
        let effect = s
            .automatic_rewards
            .iter()
            .find(|r| r.kind == "SEND_ANIMATED_MESSAGE")
            .unwrap();
        assert_eq!(
            effect
                .bits_cost
                .filter(|b| *b > 0)
                .unwrap_or(effect.default_bits_cost),
            20
        );
        assert_eq!(s.custom_rewards[0].bits_cost, Some(100));
    }
}
