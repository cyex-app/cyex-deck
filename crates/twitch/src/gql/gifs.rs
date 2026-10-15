//! 1.43 Sending chat GIFs (Tier 2 and 3 subscribers where the channel allows them).
//! Twitch hands out its GIPHY key with the picker settings; it stays in Rust.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, persisted_or_raw, query};
use crate::{Http, Result};

const ACCESS: &str = "query($id: ID!) { gifPickerConfig(channelID: $id) { isEnabled isAllowlisted apiKey contentRating } \
    user(id: $id) { subscriptionProducts { tier hasGifs } self { subscriptionBenefit { tier } } } }";

const SEND: (&str, &str, &str) = (
    "sendGifMessage",
    "1b7a15ea1afa85371aa360c47b4c26d49c995ef6fb02902f6fb9f1ed8a5f9e45",
    "mutation sendGifMessage($input: SendGifMessageInput!) { sendGifMessage(input: $input) { error secondsUntilCanSend message { id } } }",
);

/// Whether the account can send GIFs in a channel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GifAccess {
    pub can_send: bool,
    /// Why not, ready to show.
    pub reason: Option<&'static str>,
    pub api_key: Option<String>,
    /// GIPHY rating: `g`, `pg` or `pg-13`.
    pub rating: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifSent {
    /// Twitch's code when it didn't go through.
    pub error: Option<String>,
    /// Wait this long before the next one.
    pub wait_secs: Option<u32>,
}

pub async fn gif_access(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<GifAccess> {
    let d: RawData = query(http, auth, ACCESS, json!({ "id": channel_id })).await?;
    Ok(decide(d))
}

fn decide(d: RawData) -> GifAccess {
    let channel_has = d
        .user
        .as_ref()
        .is_some_and(|u| u.subscription_products.iter().any(|p| p.has_gifs));
    let tier = d
        .user
        .as_ref()
        .and_then(|u| u.me.as_ref())
        .and_then(|s| s.subscription_benefit.as_ref())
        .and_then(|b| b.tier.parse::<u32>().ok())
        .unwrap_or(0);
    let cfg = d.gif_picker_config.unwrap_or_default();
    let reason = if !channel_has {
        Some("This channel doesn’t allow GIFs.")
    } else if !(cfg.is_enabled && cfg.is_allowlisted) {
        Some("GIFs are unavailable right now.")
    } else if tier < 2000 {
        Some("Subscribe at Tier 2 or 3 to send GIFs here.")
    } else {
        None
    };
    GifAccess {
        can_send: reason.is_none() && cfg.api_key.is_some(),
        reason,
        api_key: cfg.api_key,
        rating: match cfg.content_rating.as_deref() {
            Some("PG_13") => "pg-13",
            Some("G_PG") => "pg",
            _ => "g",
        },
    }
}

pub async fn send_gif(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    gif_id: &str,
    gif_url: &str,
    search: &str,
) -> Result<GifSent> {
    let mut input = json!({ "channelID": channel_id, "gifID": gif_id, "gifURL": gif_url });
    if !search.is_empty() {
        input["searchTerm"] = json!(search);
    }
    let v = persisted_or_raw(http, auth, SEND, json!({ "input": input })).await?;
    let r = &v["sendGifMessage"];
    Ok(GifSent {
        error: r["error"].as_str().map(str::to_owned),
        wait_secs: r["secondsUntilCanSend"]
            .as_u64()
            .map(|s| s as u32)
            .filter(|s| *s > 0),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawData {
    gif_picker_config: Option<RawConfig>,
    user: Option<RawUser>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConfig {
    is_enabled: bool,
    is_allowlisted: bool,
    api_key: Option<String>,
    content_rating: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    #[serde(default)]
    subscription_products: Vec<RawProduct>,
    #[serde(rename = "self")]
    me: Option<RawSelf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProduct {
    has_gifs: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSelf {
    subscription_benefit: Option<RawBenefit>,
}

#[derive(Deserialize)]
struct RawBenefit {
    tier: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(json: &str) -> RawData {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn who_can_send() {
        let ok = decide(data(
            r#"{"gifPickerConfig":{"isEnabled":true,"isAllowlisted":true,"apiKey":"k","contentRating":"G_PG"},
            "user":{"subscriptionProducts":[{"tier":"1000","hasGifs":false},{"tier":"2000","hasGifs":true}],"self":{"subscriptionBenefit":{"tier":"2000"}}}}"#,
        ));
        assert!(ok.can_send);
        assert_eq!(ok.rating, "pg");
        let tier1 = decide(data(
            r#"{"gifPickerConfig":{"isEnabled":true,"isAllowlisted":true,"apiKey":"k","contentRating":null},
            "user":{"subscriptionProducts":[{"tier":"2000","hasGifs":true}],"self":{"subscriptionBenefit":{"tier":"1000"}}}}"#,
        ));
        assert_eq!(tier1.reason, Some("Subscribe at Tier 2 or 3 to send GIFs here."));
        let off = decide(data(
            r#"{"gifPickerConfig":null,"user":{"subscriptionProducts":[{"tier":"2000","hasGifs":false}],"self":null}}"#,
        ));
        assert_eq!(off.reason, Some("This channel doesn’t allow GIFs."));
    }
}
