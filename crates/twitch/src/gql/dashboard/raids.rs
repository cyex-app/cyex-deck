//! 1.35 Raids from your own channel: start the countdown or call it off. GQL has no read
//! of a pending raid; its state comes live from Hermes `raid.<channel id>` (D2, read here),
//! about once a second while it counts down. D2 also: "Raid now" and Twitch's suggestions.

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::mutate;
use crate::gql::{Auth, query};
use crate::{Http, Result};

const CREATE: &str = "mutation($input: CreateRaidInput!) { createRaid(input: $input) { error { code } } }";
const CANCEL: &str = "mutation($input: CancelRaidInput!) { cancelRaid(input: $input) { error { code } } }";
const GO: &str = "mutation($input: GoRaidInput!) { goRaid(input: $input) { error { code } } }";
const PICKS: &str = "query($id: ID!) { user(id: $id) { raidRecommendations(options: {sources: null}) {
  user { id login displayName profileImageURL(width: 50) broadcastSettings { title } }
  viewers category { name } } } }";
/// Twitch's countdown when the raid doesn't say.
const FORCE_SECS: i64 = 90;

pub async fn start(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str) -> Result<()> {
    let input = json!({ "sourceID": channel_id, "targetID": target_id });
    mutate(http, auth, CREATE, "createRaid", input).await
}

pub async fn cancel(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    mutate(
        http,
        auth,
        CANCEL,
        "cancelRaid",
        json!({ "sourceID": channel_id }),
    )
    .await
}

/// D2 Sends the raid now instead of waiting for the countdown.
pub async fn go(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    mutate(http, auth, GO, "goRaid", json!({ "sourceID": channel_id })).await
}

/// D2 A live channel Twitch suggests raiding.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmRaidPick")]
pub struct RaidPick {
    pub login: String,
    pub name: String,
    pub avatar: String,
    #[ts(type = "number")]
    pub viewers: u64,
    pub category: String,
    pub title: String,
}

/// D2 Twitch's mix of suggestions (followed, similar, same category). Needs the
/// broadcaster's own GQL sign-in.
pub async fn suggestions(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<RaidPick>> {
    let data: Value = query(http, auth, PICKS, json!({ "id": channel_id })).await?;
    Ok(picks(&data))
}

fn picks(data: &Value) -> Vec<RaidPick> {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    data["user"]["raidRecommendations"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|r| {
                    let u = &r["user"];
                    let login = u["login"].as_str()?.to_owned();
                    Some(RaidPick {
                        name: u["displayName"].as_str().map_or_else(|| login.clone(), str::to_owned),
                        login,
                        avatar: s(&u["profileImageURL"]),
                        viewers: r["viewers"].as_u64().unwrap_or(0),
                        category: s(&r["category"]["name"]),
                        title: s(&u["broadcastSettings"]["title"]),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// D2 A raid counting down from your channel.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmRaid")]
pub struct RaidStatus {
    pub id: String,
    pub target_login: String,
    pub target_name: String,
    pub target_avatar: String,
    /// Viewers going along.
    #[ts(type = "number")]
    pub viewers: u64,
    /// Unix seconds it started.
    #[ts(type = "number")]
    pub created_at: i64,
    /// Unix seconds it goes by itself.
    #[ts(type = "number")]
    pub goes_at: i64,
}

/// D2 What a `raid.<channel id>` message says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RaidEvent {
    Update(RaidStatus),
    /// It went or was called off.
    Done { id: String },
}

/// Only the source channel's own raids count.
pub fn parse_raid(msg: &Value, channel_id: &str) -> Option<RaidEvent> {
    let r = &msg["raid"];
    let id = r["id"].as_str()?.to_owned();
    if r["source_id"].as_str().is_some_and(|s| s != channel_id) {
        return None;
    }
    match msg["type"].as_str()? {
        "raid_update_v2" => {
            let created_at = r["raid_created_at"].as_i64()?;
            let s = |k: &str| r[k].as_str().unwrap_or_default().to_owned();
            let login = s("target_login");
            Some(RaidEvent::Update(RaidStatus {
                id,
                target_name: Some(s("target_display_name")).filter(|n| !n.is_empty()).unwrap_or_else(|| login.clone()),
                target_login: login,
                target_avatar: s("target_profile_image"),
                viewers: r["viewer_count"].as_u64().unwrap_or(0),
                created_at,
                goes_at: created_at + r["force_raid_now_seconds"].as_i64().unwrap_or(FORCE_SECS),
            }))
        }
        "raid_go_v2" | "raid_cancel_v2" => Some(RaidEvent::Done { id }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid_messages_read_like_twitchs() {
        let msg = json!({ "type": "raid_update_v2", "raid": { "id": "r1", "creator_id": "7", "source_id": "7",
            "target_id": "9", "target_login": "kuni", "target_display_name": "Kuni", "target_profile_image": "https://p",
            "target_viewer_count": "1234", "viewer_count": 682, "force_raid_now_seconds": 90, "raid_created_at": 1000 } });
        let Some(RaidEvent::Update(s)) = parse_raid(&msg, "7") else { panic!() };
        assert_eq!((s.target_name.as_str(), s.viewers, s.goes_at), ("Kuni", 682, 1090));
        assert_eq!(parse_raid(&msg, "8"), None, "someone else's raid");
        let go = json!({ "type": "raid_go_v2", "raid": { "id": "r1", "source_id": "7" } });
        assert_eq!(parse_raid(&go, "7"), Some(RaidEvent::Done { id: "r1".into() }));
    }

    #[test]
    fn suggestions_read() {
        let data = json!({ "user": { "raidRecommendations": [
            { "user": { "id": "1", "login": "a", "displayName": "A", "profileImageURL": "https://a",
                "broadcastSettings": { "title": "hi" } }, "viewers": 50, "category": { "name": "Chess" } },
            { "user": null } ] } });
        let p = picks(&data);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].name.as_str(), p[0].viewers, p[0].category.as_str()), ("A", 50, "Chess"));
    }
}
