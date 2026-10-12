//! 1.35 Chat modes on your own channel. Sub-only chat isn't a chat setting on GQL: it's a
//! switch on the channel's tier 1 sub product, so non-affiliates can't turn it on.

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::mutate;
use crate::gql::{Auth, query};
use crate::helix::modes::{ChatModes, MAX_FOLLOW_MINUTES, ModeChange, SLOW_RANGE};
use crate::{Error, Http, Result};

const READ: &str = "query($id: ID!) { user(id: $id) { chatSettings { isEmoteOnlyModeEnabled \
    isSubscribersOnlyModeEnabled followersOnlyDurationMinutes slowModeDurationSeconds isUniqueChatModeEnabled \
    chatDelayMs } } }";
const SET: &str =
    "mutation($input: UpdateChatSettingsInput!) { updateChatSettings(input: $input) { error { code } } }";
const PRODUCTS: &str = "query($id: ID!) { user(id: $id) { subscriptionProducts { id tier } } }";
const SUB_ONLY: &str = "mutation($input: UpdateSubscriptionProductInput!) { \
    updateSubscriptionProduct(input: $input) { product { id } } }";

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    is_emote_only_mode_enabled: bool,
    #[serde(default)]
    is_subscribers_only_mode_enabled: bool,
    /// Null or -1 = off; 0 = any follower.
    followers_only_duration_minutes: Option<i64>,
    /// Null or 0 = off.
    slow_mode_duration_seconds: Option<i64>,
    #[serde(default)]
    is_unique_chat_mode_enabled: bool,
    chat_delay_ms: Option<i64>,
}

fn modes_from(s: Settings) -> ChatModes {
    let follow = s.followers_only_duration_minutes.filter(|m| *m >= 0);
    let slow = s.slow_mode_duration_seconds.filter(|s| *s > 0);
    let delay = s.chat_delay_ms.filter(|ms| *ms > 0);
    ChatModes {
        emote_mode: s.is_emote_only_mode_enabled,
        follower_mode: follow.is_some(),
        follower_mode_duration: follow.map(|m| m as u32),
        slow_mode: slow.is_some(),
        slow_mode_wait_time: slow.map(|s| s as u32),
        subscriber_mode: s.is_subscribers_only_mode_enabled,
        unique_chat_mode: s.is_unique_chat_mode_enabled,
        non_moderator_chat_delay: Some(delay.is_some()),
        non_moderator_chat_delay_duration: delay.map(|ms| (ms / 1000) as u32),
    }
}

pub async fn modes(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<ChatModes> {
    #[derive(Deserialize)]
    struct Data {
        user: Option<User>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct User {
        chat_settings: Option<Settings>,
    }
    let d: Data = query(http, auth, READ, json!({ "id": channel_id })).await?;
    Ok(modes_from(
        d.user.and_then(|u| u.chat_settings).unwrap_or_default(),
    ))
}

/// The `UpdateChatSettingsInput` fields for a change; `None` for sub-only, which isn't one.
fn settings_input(channel_id: &str, change: ModeChange) -> Option<Value> {
    let mut m = Map::new();
    m.insert("channelID".into(), channel_id.into());
    let (k, v): (&str, Value) = match change {
        ModeChange::Emote(on) => ("isEmoteOnlyModeEnabled", on.into()),
        ModeChange::Unique(on) => ("isUniqueChatModeEnabled", on.into()),
        ModeChange::Followers(min) => (
            "followersOnlyDurationMinutes",
            min.map_or(-1, |m| i64::from(m.min(MAX_FOLLOW_MINUTES))).into(),
        ),
        ModeChange::Slow(secs) => (
            "slowModeDurationSeconds",
            secs.map_or(0, |s| s.clamp(SLOW_RANGE.0, SLOW_RANGE.1)).into(),
        ),
        ModeChange::Delay(secs) => (
            "chatDelayInMs",
            secs.map_or(0, |s| {
                [2, 4, 6]
                    .into_iter()
                    .min_by_key(|v: &u32| v.abs_diff(s))
                    .unwrap_or(2)
                    * 1000
            })
            .into(),
        ),
        ModeChange::Subs(_) => return None,
    };
    m.insert(k.into(), v);
    Some(Value::Object(m))
}

/// The tier 1 product, which carries the sub-only switch.
async fn tier1_product(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Data {
        user: Option<User>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct User {
        #[serde(default)]
        subscription_products: Option<Vec<Product>>,
    }
    #[derive(Deserialize)]
    struct Product {
        id: String,
        tier: Option<String>,
    }
    let d: Data = query(http, auth, PRODUCTS, json!({ "id": channel_id })).await?;
    let products = d.user.and_then(|u| u.subscription_products).unwrap_or_default();
    products
        .iter()
        .find(|p| p.tier.as_deref() == Some("1000"))
        .or(products.first())
        .map(|p| p.id.clone())
        .ok_or_else(|| Error::Refused("NO_SUB_PRODUCT".into()))
}

/// Applies one change and reads the modes back.
pub async fn set_mode(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    change: ModeChange,
) -> Result<ChatModes> {
    match (change, settings_input(channel_id, change)) {
        (_, Some(input)) => mutate(http, auth, SET, "updateChatSettings", input).await?,
        (ModeChange::Subs(on), None) => {
            let product = tier1_product(http, auth, channel_id).await?;
            let input = json!({ "id": product, "targetUserID": channel_id, "hasSubOnlyChat": on });
            let _: Value = query(http, auth, SUB_ONLY, json!({ "input": input })).await?;
        }
        _ => {}
    }
    modes(http, auth, channel_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_values_read_as_off() {
        let s: Settings = serde_json::from_str(
            r#"{"isEmoteOnlyModeEnabled":true,"followersOnlyDurationMinutes":-1,"slowModeDurationSeconds":null,
               "chatDelayMs":0}"#,
        )
        .unwrap();
        let m = modes_from(s);
        assert!(m.emote_mode && !m.follower_mode && !m.slow_mode);
        assert_eq!(m.non_moderator_chat_delay, Some(false));
        let s: Settings =
            serde_json::from_str(r#"{"followersOnlyDurationMinutes":0,"slowModeDurationSeconds":30}"#)
                .unwrap();
        let m = modes_from(s);
        assert_eq!((m.follower_mode, m.follower_mode_duration), (true, Some(0)));
        assert_eq!(m.slow_mode_wait_time, Some(30));
    }

    #[test]
    fn changes_send_twitchs_off_values() {
        let input = |c| settings_input("7", c).unwrap();
        assert_eq!(
            input(ModeChange::Followers(None))["followersOnlyDurationMinutes"],
            -1
        );
        assert_eq!(
            input(ModeChange::Followers(Some(10)))["followersOnlyDurationMinutes"],
            10
        );
        assert_eq!(input(ModeChange::Slow(None))["slowModeDurationSeconds"], 0);
        assert_eq!(input(ModeChange::Slow(Some(500)))["slowModeDurationSeconds"], 120);
        assert_eq!(input(ModeChange::Emote(true))["channelID"], "7");
        assert!(settings_input("7", ModeChange::Subs(true)).is_none());
    }
}
