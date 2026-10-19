//! What channels know about one user, read signed out: whether they moderate each one
//! (V102-21, the role check's outside source) and every Twitch badge they own in one
//! (V102-13, the usercard, like the 7TV extension's `channelViewer.earnedBadges`).

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::chat::{BADGE_FIELDS, BadgeInfo, RawBadge};
use super::{Auth, query};
use crate::{Http, Result};

/// Channels asked about per request (checked: 40 in one went through).
pub const MODERATES_MAX: usize = 50;

const EARNED: &str = "query($user: String!, $channel: String!) { channelViewer(userLogin: $user, channelLogin: $channel) { \
    earnedBadges { BADGE } } }";

/// Whether `user_id` moderates each channel, in order. The broadcaster of a channel is
/// not its moderator here. Ids that aren't numbers come back `false`.
pub async fn moderates(http: &Http, user_id: &str, channel_ids: &[String]) -> Result<Vec<bool>> {
    if channel_ids.is_empty() {
        return Ok(Vec::new());
    }
    let data: ModData = query(
        http,
        Auth::PUBLIC,
        &moderates_query(channel_ids),
        json!({ "id": user_id }),
    )
    .await?;
    Ok(read_moderates(data, channel_ids.len()))
}

fn moderates_query(channel_ids: &[String]) -> String {
    let fields: String = channel_ids
        .iter()
        .enumerate()
        .filter(|(_, id)| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
        .map(|(i, id)| format!("c{i}: isModerator(channelID: \"{id}\") "))
        .collect();
    format!("query($id: ID!) {{ user(id: $id) {{ id {fields}}} }}")
}

#[derive(Deserialize)]
struct ModData {
    user: Option<HashMap<String, serde_json::Value>>,
}

fn read_moderates(data: ModData, n: usize) -> Vec<bool> {
    let user = data.user.unwrap_or_default();
    (0..n)
        .map(|i| {
            user.get(&format!("c{i}"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .collect()
}

/// Every Twitch badge `user` owns in `channel`'s chat: the channel's own (sub, bits, mod)
/// and global ones, in Twitch's order.
pub async fn earned_badges(http: &Http, user: &str, channel: &str) -> Result<Vec<BadgeInfo>> {
    let q = EARNED.replace("BADGE", BADGE_FIELDS);
    let data: EarnedData = query(
        http,
        Auth::PUBLIC,
        &q,
        json!({ "user": user, "channel": channel }),
    )
    .await?;
    Ok(data
        .channel_viewer
        .map(|v| {
            v.earned_badges
                .into_iter()
                .flatten()
                .flatten()
                .map(BadgeInfo::from)
                .collect()
        })
        .unwrap_or_default())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EarnedData {
    channel_viewer: Option<RawViewer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawViewer {
    /// V107-M8 `null` for someone with no badges at all.
    #[serde(default)]
    earned_badges: Option<Vec<Option<RawBadge>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_m8_no_badges_is_null() {
        let d: EarnedData = serde_json::from_str(r#"{"channelViewer":{"earnedBadges":null}}"#).unwrap();
        assert!(d.channel_viewer.and_then(|v| v.earned_badges).is_none());
    }

    #[test]
    fn moderates_asks_once_per_channel_and_reads_in_order() {
        let ids = vec!["22484632".to_owned(), "x\"}".to_owned(), "11148817".to_owned()];
        let q = moderates_query(&ids);
        assert!(q.contains("c0: isModerator(channelID: \"22484632\")"));
        assert!(q.contains("c2: isModerator(channelID: \"11148817\")"));
        assert!(!q.contains("c1:"), "odd ids aren't sent: {q}");
        let data: ModData = serde_json::from_str(r#"{"user":{"id":"1","c0":true,"c2":false}}"#).unwrap();
        assert_eq!(read_moderates(data, 3), [true, false, false]);
        let gone: ModData = serde_json::from_str(r#"{"user":null}"#).unwrap();
        assert_eq!(read_moderates(gone, 2), [false, false]);
    }

    #[test]
    fn earned_badges_parse() {
        let d: EarnedData = serde_json::from_str(
            r#"{"channelViewer":{"id":"1:2","earnedBadges":[
              {"setID":"subscriber","version":"132","title":"11-Year Subscriber","description":"11-Year Subscriber","clickURL":null,
               "image1x":"a1","image2x":"a2","image4x":"a4"},
              null]}}"#,
        )
        .unwrap();
        let list: Vec<BadgeInfo> = d
            .channel_viewer
            .unwrap()
            .earned_badges
            .into_iter()
            .flatten()
            .flatten()
            .map(BadgeInfo::from)
            .collect();
        assert_eq!(
            (list.len(), list[0].set.as_str(), list[0].version.as_str()),
            (1, "subscriber", "132")
        );
        let none: EarnedData = serde_json::from_str(r#"{"channelViewer":null}"#).unwrap();
        assert!(none.channel_viewer.is_none());
    }
}
