//! Drops for the miner (1.30, 1.32): which channels have campaigns, the account's
//! progress, and claiming finished drops. Raw queries, schema-checked 2026/10/10. Since
//! 2026/10/07 Twitch wants the game account linked before a drop can be claimed.

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::{Http, Result};

const CHANNEL: &str = "query($id: ID!) { channel(id: $id) { viewerDropCampaigns { id } } }";
const INVENTORY: &str = "query { currentUser { inventory { dropCampaignsInProgress { id name \
    game { id displayName } timeBasedDrops { id name requiredMinutesWatched \
    benefitEdges { benefit { name imageAssetURL } } \
    self { hasPreconditionsMet currentMinutesWatched dropInstanceID isClaimed } } } } } }";
const CLAIM: &str = "mutation($input: ClaimDropRewardsInput!) { claimDropRewards(input: $input) { status } }";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub game: Option<String>,
    pub drops: Vec<Drop>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drop {
    pub id: String,
    /// The reward's name, else the drop's.
    pub name: String,
    pub image: Option<String>,
    pub required_minutes: u32,
    pub watched_minutes: u32,
    /// Set once the drop is earned and can be claimed.
    pub instance_id: Option<String>,
    pub claimed: bool,
}

impl Drop {
    pub fn claimable(&self) -> bool {
        !self.claimed && self.instance_id.is_some()
    }
}

/// Campaign ids running on a channel for this account.
pub async fn channel_campaigns(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<String>> {
    let data: ChannelData = query(http, auth, CHANNEL, json!({ "id": channel_id })).await?;
    Ok(data
        .channel
        .and_then(|c| c.viewer_drop_campaigns)
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.id)
        .collect())
}

/// Campaigns the account has progress in.
pub async fn inventory(http: &Http, auth: Auth<'_>) -> Result<Vec<Campaign>> {
    let data: InventoryData = query(http, auth, INVENTORY, json!({})).await?;
    Ok(data
        .current_user
        .and_then(|u| u.inventory)
        .and_then(|i| i.drop_campaigns_in_progress)
        .unwrap_or_default()
        .into_iter()
        .map(Campaign::from)
        .collect())
}

/// Claims an earned drop. True when it's (now) claimed.
pub async fn claim(http: &Http, auth: Auth<'_>, instance_id: &str) -> Result<bool> {
    let data: ClaimData = query(
        http,
        auth,
        CLAIM,
        json!({ "input": { "dropInstanceID": instance_id } }),
    )
    .await?;
    let status = data.claim_drop_rewards.map(|c| c.status).unwrap_or_default();
    Ok(matches!(
        status.as_str(),
        "ELIGIBLE_FOR_ALL" | "DROP_INSTANCE_ALREADY_CLAIMED"
    ))
}

#[derive(Deserialize)]
struct ChannelData {
    channel: Option<RawChannel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    viewer_drop_campaigns: Option<Vec<IdOnly>>,
}

#[derive(Deserialize)]
struct IdOnly {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InventoryData {
    current_user: Option<RawUser>,
}

#[derive(Deserialize)]
struct RawUser {
    inventory: Option<RawInventory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInventory {
    drop_campaigns_in_progress: Option<Vec<RawCampaign>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCampaign {
    id: String,
    #[serde(default)]
    name: String,
    game: Option<RawGame>,
    #[serde(default)]
    time_based_drops: Vec<RawDrop>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDrop {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    required_minutes_watched: u32,
    #[serde(default)]
    benefit_edges: Vec<RawEdge>,
    #[serde(rename = "self")]
    me: Option<RawDropSelf>,
}

#[derive(Deserialize)]
struct RawEdge {
    benefit: Option<RawBenefit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBenefit {
    name: Option<String>,
    #[serde(rename = "imageAssetURL")]
    image_asset_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDropSelf {
    #[serde(default)]
    current_minutes_watched: u32,
    #[serde(rename = "dropInstanceID")]
    drop_instance_id: Option<String>,
    #[serde(default)]
    is_claimed: bool,
}

impl From<RawCampaign> for Campaign {
    fn from(c: RawCampaign) -> Self {
        Self {
            id: c.id,
            name: c.name,
            game: c.game.and_then(|g| g.display_name),
            drops: c
                .time_based_drops
                .into_iter()
                .map(|d| {
                    let benefit = d.benefit_edges.into_iter().find_map(|e| e.benefit);
                    let me = d.me;
                    Drop {
                        name: benefit
                            .as_ref()
                            .and_then(|b| b.name.clone())
                            .filter(|n| !n.is_empty())
                            .unwrap_or(d.name),
                        image: benefit.and_then(|b| b.image_asset_url),
                        id: d.id,
                        required_minutes: d.required_minutes_watched,
                        watched_minutes: me.as_ref().map_or(0, |m| m.current_minutes_watched),
                        instance_id: me.as_ref().and_then(|m| m.drop_instance_id.clone()),
                        claimed: me.is_some_and(|m| m.is_claimed),
                    }
                })
                .collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimData {
    claim_drop_rewards: Option<ClaimStatus>,
}

#[derive(Deserialize)]
struct ClaimStatus {
    #[serde(default)]
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_parses_progress_and_claims() {
        let raw = r#"{"currentUser":{"inventory":{"dropCampaignsInProgress":[{"id":"c","name":"Fall drops",
            "game":{"id":"1","displayName":"Rust"},"timeBasedDrops":[
            {"id":"d1","name":"Drop 1","requiredMinutesWatched":60,"benefitEdges":[{"benefit":{"name":"Hat","imageAssetURL":"x.png"}}],
             "self":{"hasPreconditionsMet":true,"currentMinutesWatched":60,"dropInstanceID":"i1","isClaimed":false}},
            {"id":"d2","name":"Drop 2","requiredMinutesWatched":120,"benefitEdges":[],
             "self":{"hasPreconditionsMet":true,"currentMinutesWatched":61,"dropInstanceID":null,"isClaimed":false}}]}]}}}"#;
        let data: InventoryData = serde_json::from_str(raw).unwrap();
        let c: Vec<Campaign> = data
            .current_user
            .unwrap()
            .inventory
            .unwrap()
            .drop_campaigns_in_progress
            .unwrap()
            .into_iter()
            .map(Campaign::from)
            .collect();
        assert_eq!(c[0].game.as_deref(), Some("Rust"));
        assert_eq!(c[0].drops[0].name, "Hat");
        assert!(c[0].drops[0].claimable());
        assert_eq!(c[0].drops[1].name, "Drop 2");
        assert!(!c[0].drops[1].claimable());
        assert_eq!(c[0].drops[1].watched_minutes, 61);
    }
}
