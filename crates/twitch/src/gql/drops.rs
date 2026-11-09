//! Drops for the miner (1.30, 1.32, V110-U68): which channels have campaigns, the running
//! campaigns and their badges, the account's progress, and claiming finished drops. Raw
//! queries. The field names are the ones the cyex-miner reference reads from its persisted
//! Inventory, ViewerDropsDashboard and DropCampaignDetails answers; schema-check the new
//! ones (docs/research/miner.md) before relying on them live. Since 2026/10/07 Twitch
//! wants the game account linked before a drop can be claimed.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, BATCH_MAX, batch, batch_persisted, nullable, persisted, query};
use crate::clock::parse_rfc3339;
use crate::{Error, Http, Result};

/// Campaign details per request (the reference script's batch size).
pub const DETAILS_BATCH: usize = 20;
/// Channels checked per request.
pub const CHANNELS_BATCH: usize = 30;

/// The fields of one drop, shared by the inventory and the campaign details.
macro_rules! drop_fields {
    () => {
        "id name requiredMinutesWatched requiredSubs startAt endAt \
         benefitEdges { benefit { id name imageAssetURL distributionType } }"
    };
}
macro_rules! campaign_fields {
    () => {
        "id name status startAt endAt game { id slug displayName } \
         allow { isEnabled channels { id name } }"
    };
}

const CHANNEL: &str = "query($id: ID!) { channel(id: $id) { viewerDropCampaigns { id } } }";
const DASHBOARD: &str = "query { currentUser { dropCampaigns { id status } } }";
const DETAILS: &str = concat!(
    "query($id: ID!) { currentUser { dropCampaign(id: $id) { ",
    campaign_fields!(),
    " timeBasedDrops { ",
    drop_fields!(),
    " } } } }"
);
const INVENTORY: &str = concat!(
    "query { currentUser { inventory { dropCampaignsInProgress { ",
    campaign_fields!(),
    " timeBasedDrops { ",
    drop_fields!(),
    " self { hasPreconditionsMet currentMinutesWatched dropInstanceID isClaimed } } } \
     gameEventDrops { id } } } }"
);
/// The inventory query checked 2026/10/10, before badges: the last resort when the one
/// above and its persisted twin are both turned down.
const INVENTORY_BASIC: &str = "query { currentUser { inventory { dropCampaignsInProgress { id name \
    game { id displayName } timeBasedDrops { id name requiredMinutesWatched \
    benefitEdges { benefit { name imageAssetURL } } \
    self { hasPreconditionsMet currentMinutesWatched dropInstanceID isClaimed } } } } } }";
const CLAIM: &str = "mutation($input: ClaimDropRewardsInput!) { claimDropRewards(input: $input) { status } }";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub game: Option<String>,
    pub game_id: Option<String>,
    /// The directory's name for the game, e.g. `just-chatting`.
    pub game_slug: Option<String>,
    /// Only these channels count; empty means any channel playing the game.
    pub allowed: Vec<Allowed>,
    /// Unix seconds; 0 when Twitch didn't say.
    pub starts_at: i64,
    pub ends_at: i64,
    pub drops: Vec<Drop>,
}

/// A channel a campaign is limited to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    pub id: String,
    pub login: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drop {
    pub id: String,
    /// The reward's name, else the drop's.
    pub name: String,
    pub image: Option<String>,
    /// The reward's id (the badge's, if the drop gives one).
    pub benefit_id: Option<String>,
    /// The drop gives a chat badge (`distributionType` BADGE).
    pub badge: bool,
    /// Needs this many subs; badges for watching alone have none.
    pub required_subs: u32,
    /// Unix seconds; 0 when Twitch didn't say.
    pub starts_at: i64,
    pub ends_at: i64,
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

/// The inventory: campaigns with progress, and the rewards already earned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    pub campaigns: Vec<Campaign>,
    /// Benefit ids of earned rewards (`gameEventDrops`); a badge shows up here once earned.
    pub earned: Vec<String>,
}

/// Campaign ids running on a channel for this account.
pub async fn channel_campaigns(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<String>> {
    let data: ChannelData = query(http, auth, CHANNEL, json!({ "id": channel_id })).await?;
    Ok(data.campaign_ids())
}

/// For each channel id, the campaign ids running on it for this account, 30 channels per
/// request. A channel Twitch didn't answer for is left out; if it answered for none, the
/// first error comes back. V110-U68 Refused as raw text, the persisted twin is asked.
pub async fn channels_campaigns(
    http: &Http,
    auth: Auth<'_>,
    ids: &[String],
) -> Result<Vec<(String, Vec<String>)>> {
    let mut out = Vec::new();
    for chunk in ids.chunks(CHANNELS_BATCH.min(BATCH_MAX)) {
        let raw = async {
            let ops: Vec<(&str, Value)> = chunk.iter().map(|id| (CHANNEL, json!({ "id": id }))).collect();
            channel_answers(chunk, batch(http, auth, &ops).await?)
        };
        let twin = || async {
            let ops: Vec<(&str, &str, Value)> = chunk
                .iter()
                .map(|id| (P_CHANNEL.0, P_CHANNEL.1, json!({ "channelID": id })))
                .collect();
            channel_answers(chunk, batch_persisted(http, auth, &ops).await?)
        };
        out.extend(raw_or_persisted(raw.await, twin).await?);
    }
    Ok(out)
}

fn channel_answers(ids: &[String], answers: Vec<Result<Value>>) -> Result<Vec<(String, Vec<String>)>> {
    let mut out = Vec::new();
    let mut first_err = None;
    for (id, answer) in ids.iter().zip(answers) {
        match answer.and_then(parse_channel) {
            Ok(c) => out.push((id.clone(), c)),
            Err(e) => {
                first_err.get_or_insert(e);
            }
        }
    }
    all_failed(out, first_err)
}

/// V110-U68 Campaign ids in one channel answer (`data`), raw or persisted.
pub fn parse_channel(data: Value) -> Result<Vec<String>> {
    Ok(serde_json::from_value::<ChannelData>(data).map_err(Error::Decode)?.campaign_ids())
}

/// Ids of the campaigns running now (the dashboard's ACTIVE ones).
pub async fn dashboard(http: &Http, auth: Auth<'_>) -> Result<Vec<String>> {
    let raw = async { parse_dashboard(query(http, auth, DASHBOARD, json!({})).await?) };
    let twin = || async {
        let vars = json!({ "fetchRewardCampaigns": false });
        parse_dashboard(persisted(http, auth, P_DASHBOARD.0, P_DASHBOARD.1, vars).await?)
    };
    raw_or_persisted(raw.await, twin).await
}

/// V110-U68 The ACTIVE ids of a dashboard answer (`data`); no list is an error.
pub fn parse_dashboard(data: Value) -> Result<Vec<String>> {
    serde_json::from_value::<DashboardData>(data).map_err(Error::Decode)?.active()
}

/// Details of campaigns by id, 20 per request. A campaign Twitch didn't answer for is left
/// out; if it answered for none, the first error comes back. `user_id` is the account's,
/// which the persisted twin wants.
pub async fn details(http: &Http, auth: Auth<'_>, user_id: &str, ids: &[String]) -> Result<Vec<Campaign>> {
    let mut out = Vec::new();
    for chunk in ids.chunks(DETAILS_BATCH.min(BATCH_MAX)) {
        let raw = async {
            let ops: Vec<(&str, Value)> = chunk.iter().map(|id| (DETAILS, json!({ "id": id }))).collect();
            details_answers(batch(http, auth, &ops).await?)
        };
        let twin = || async {
            let ops: Vec<(&str, &str, Value)> = chunk
                .iter()
                .map(|id| (P_DETAILS.0, P_DETAILS.1, json!({ "dropID": id, "channelLogin": user_id })))
                .collect();
            details_answers(batch_persisted(http, auth, &ops).await?)
        };
        out.extend(raw_or_persisted(raw.await, twin).await?);
    }
    Ok(out)
}

/// Answers that all came back without one campaign in them are a refusal too: the shape
/// didn't match.
fn details_answers(answers: Vec<Result<Value>>) -> Result<Vec<Campaign>> {
    let asked = answers.len();
    let mut out = Vec::new();
    let mut first_err = None;
    for answer in answers {
        match answer {
            Ok(v) => out.extend(campaign_from_details(v)),
            Err(e) => {
                first_err.get_or_insert(e);
            }
        }
    }
    if out.is_empty() && first_err.is_none() && asked > 0 {
        return Err(no_shape("no campaign in the details"));
    }
    all_failed(out, first_err)
}

fn all_failed<T>(out: Vec<T>, first_err: Option<Error>) -> Result<Vec<T>> {
    match (out.is_empty(), first_err) {
        (true, Some(e)) => Err(e),
        _ => Ok(out),
    }
}

/// V110-U68 One campaign of a details answer (`data`, under `currentUser` or `user`).
pub fn campaign_from_details(data: Value) -> Option<Campaign> {
    let d: DetailsData = serde_json::from_value(data).ok()?;
    d.current_user?.drop_campaign.map(Campaign::from)
}

/// Campaigns the account has progress in.
pub async fn inventory(http: &Http, auth: Auth<'_>) -> Result<Vec<Campaign>> {
    Ok(inventory_full(http, auth).await?.campaigns)
}

/// The inventory with the earned rewards too. V110-U68 Refused as raw text, the persisted
/// twin is asked; refused both ways, the older checked query (no earned list, no badges)
/// keeps drop claims going.
pub async fn inventory_full(http: &Http, auth: Auth<'_>) -> Result<Inventory> {
    let raw = async { parse_inventory(query(http, auth, INVENTORY, json!({})).await?) };
    let twin = || async {
        let vars = json!({ "fetchRewardCampaigns": false });
        parse_inventory(persisted(http, auth, P_INVENTORY.0, P_INVENTORY.1, vars).await?)
    };
    match raw_or_persisted(raw.await, twin).await {
        Err(e) if both_refused(&e) => match query::<Value>(http, auth, INVENTORY_BASIC, json!({})).await {
            Ok(v) => parse_inventory(v).map_err(|_| e),
            Err(_) => Err(e),
        },
        other => other,
    }
}

/// V110-U68 An inventory answer (`data`), raw or persisted. A missing inventory reads as
/// empty (a new account has none).
pub fn parse_inventory(data: Value) -> Result<Inventory> {
    Ok(serde_json::from_value::<InventoryData>(data).map_err(Error::Decode)?.into())
}

/// V110-U68 The persisted twins (name, sha256) of the raw queries, from the cyex-miner
/// reference (src/constants.rs). Used only when Twitch turns the raw text down.
const P_DASHBOARD: (&str, &str) = (
    "ViewerDropsDashboard",
    "c16bb890cc8ce7647a96ee69cd313d423a378a3dedadf630a1017cde18975feb",
);
const P_DETAILS: (&str, &str) = (
    "DropCampaignDetails",
    "039277bf98f3130929262cc7c6efd9c141ca3749cb6dca442fc8ead9a53f77c1",
);
const P_INVENTORY: (&str, &str) = (
    "Inventory",
    "8337eb8541b314040b0edde0c09c5c7a2783ba1960aa9edfbf3bac16d0fec404",
);
const P_CHANNEL: (&str, &str) = (
    "DropsHighlightService_AvailableDrops",
    "782dad0f032942260171d2d80a654f88bdd0c5a9dddc392e9bc92218a0f42d20",
);

/// The start of the message when both ways were turned down.
const BOTH: &str = "raw and persisted queries both refused";

fn no_shape(what: &str) -> Error {
    Error::Api {
        status: 200,
        message: what.into(),
    }
}

/// Twitch answered but the query didn't work as sent (an unknown field, a missing list,
/// an answer of another shape): worth asking the other way. Network trouble, 401, 429 and
/// 5xx are not.
fn refused(e: &Error) -> bool {
    match e {
        Error::Decode(_) => true,
        Error::Api { status, .. } => matches!(status, 200 | 400 | 422),
        _ => false,
    }
}

/// V110-U68 True when the raw query and its persisted twin were both turned down, so a
/// read that came back empty-handed isn't mistaken for "nothing to earn".
pub fn both_refused(e: &Error) -> bool {
    e.api_message().is_some_and(|m| m.starts_with(BOTH))
}

async fn raw_or_persisted<T, F>(raw: Result<T>, twin: impl FnOnce() -> F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match raw {
        Err(e) if refused(&e) => {
            twin().await.map_err(|p| Error::Api {
                status: 200,
                message: format!("{BOTH} (raw: {e}; persisted: {p})"),
            })
        }
        other => other,
    }
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

impl ChannelData {
    fn campaign_ids(self) -> Vec<String> {
        self.channel
            .and_then(|c| c.viewer_drop_campaigns)
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.id)
            .collect()
    }
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
struct DashboardData {
    current_user: Option<RawDashboardUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDashboardUser {
    drop_campaigns: Option<Vec<RawDashboardCampaign>>,
}

#[derive(Deserialize)]
struct RawDashboardCampaign {
    id: String,
    status: Option<String>,
}

impl DashboardData {
    /// No user is a failed read, not "nothing is running". A user whose list is `null`
    /// (checked 2026/11/09 against real Twitch, raw and persisted alike, no `errors`) has
    /// none to show: cyexbot gets this, while its inventory still lists a badge.
    fn active(self) -> std::result::Result<Vec<String>, Error> {
        let user = self.current_user.ok_or_else(|| Error::Api {
            status: 200,
            message: "no campaign list".into(),
        })?;
        Ok(user
            .drop_campaigns
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.status.as_deref() == Some("ACTIVE"))
            .map(|c| c.id)
            .collect())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailsData {
    #[serde(alias = "user")]
    current_user: Option<RawDetailsUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDetailsUser {
    drop_campaign: Option<RawCampaign>,
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
    game_event_drops: Option<Vec<IdOnly>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCampaign {
    id: String,
    // V110-F80 A JSON null isn't covered by `default`: it would fail the whole decode.
    #[serde(default, deserialize_with = "nullable")]
    name: String,
    start_at: Option<String>,
    end_at: Option<String>,
    game: Option<RawGame>,
    allow: Option<RawAllow>,
    #[serde(default, deserialize_with = "nullable")]
    time_based_drops: Vec<RawDrop>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAllow {
    is_enabled: Option<bool>,
    channels: Option<Vec<RawAllowed>>,
}

#[derive(Deserialize)]
struct RawAllowed {
    id: String,
    #[serde(default, deserialize_with = "nullable")]
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    id: Option<String>,
    slug: Option<String>,
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDrop {
    id: String,
    #[serde(default, deserialize_with = "nullable")]
    name: String,
    #[serde(default, deserialize_with = "nullable")]
    required_minutes_watched: u32,
    #[serde(default, deserialize_with = "nullable")]
    required_subs: u32,
    start_at: Option<String>,
    end_at: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
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
    id: Option<String>,
    name: Option<String>,
    distribution_type: Option<String>,
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

fn secs(t: Option<&str>) -> i64 {
    t.and_then(parse_rfc3339).unwrap_or(0)
}

impl From<InventoryData> for Inventory {
    fn from(d: InventoryData) -> Self {
        let Some(inv) = d.current_user.and_then(|u| u.inventory) else {
            return Self::default();
        };
        Self {
            campaigns: inv
                .drop_campaigns_in_progress
                .unwrap_or_default()
                .into_iter()
                .map(Campaign::from)
                .collect(),
            earned: inv
                .game_event_drops
                .unwrap_or_default()
                .into_iter()
                .map(|d| d.id)
                .collect(),
        }
    }
}

impl From<RawCampaign> for Campaign {
    fn from(c: RawCampaign) -> Self {
        let game = c.game;
        let allowed = match c.allow {
            // Switched off: any channel playing the game counts.
            Some(RawAllow {
                is_enabled: Some(false),
                ..
            })
            | None => Vec::new(),
            Some(a) => a
                .channels
                .unwrap_or_default()
                .into_iter()
                .map(|c| Allowed {
                    id: c.id,
                    login: c.name.to_lowercase(),
                })
                .collect(),
        };
        Self {
            id: c.id,
            name: c.name,
            game: game.as_ref().and_then(|g| g.display_name.clone()),
            game_id: game.as_ref().and_then(|g| g.id.clone()),
            game_slug: game.and_then(|g| g.slug),
            allowed,
            starts_at: secs(c.start_at.as_deref()),
            ends_at: secs(c.end_at.as_deref()),
            drops: c.time_based_drops.into_iter().map(Drop::from).collect(),
        }
    }
}

impl From<RawDrop> for Drop {
    fn from(d: RawDrop) -> Self {
        // A badge outranks the drop's other rewards.
        let mut benefits: Vec<RawBenefit> = d.benefit_edges.into_iter().filter_map(|e| e.benefit).collect();
        let badge = benefits
            .iter()
            .position(|b| b.distribution_type.as_deref() == Some("BADGE"));
        let benefit = match badge {
            Some(i) => Some(benefits.swap_remove(i)),
            None => benefits.into_iter().next(),
        };
        let me = d.me;
        Self {
            name: benefit
                .as_ref()
                .and_then(|b| b.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or(d.name),
            image: benefit.as_ref().and_then(|b| b.image_asset_url.clone()),
            benefit_id: benefit.and_then(|b| b.id),
            badge: badge.is_some(),
            required_subs: d.required_subs,
            starts_at: secs(d.start_at.as_deref()),
            ends_at: secs(d.end_at.as_deref()),
            id: d.id,
            required_minutes: d.required_minutes_watched,
            watched_minutes: me.as_ref().map_or(0, |m| m.current_minutes_watched),
            instance_id: me.as_ref().and_then(|m| m.drop_instance_id.clone()),
            claimed: me.is_some_and(|m| m.is_claimed),
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

    fn inventory_of(raw: &str) -> Inventory {
        serde_json::from_str::<InventoryData>(raw).unwrap().into()
    }

    #[test]
    fn inventory_parses_progress_and_claims() {
        let inv = inventory_of(
            r#"{"currentUser":{"inventory":{"dropCampaignsInProgress":[{"id":"c","name":"Fall drops",
            "game":{"id":"1","slug":"rust","displayName":"Rust"},"timeBasedDrops":[
            {"id":"d1","name":"Drop 1","requiredMinutesWatched":60,"benefitEdges":[{"benefit":{"id":"b1","name":"Hat","imageAssetURL":"x.png","distributionType":"DIRECT_ENTITLEMENT"}}],
             "self":{"hasPreconditionsMet":true,"currentMinutesWatched":60,"dropInstanceID":"i1","isClaimed":false}},
            {"id":"d2","name":"Drop 2","requiredMinutesWatched":120,"benefitEdges":[],
             "self":{"hasPreconditionsMet":true,"currentMinutesWatched":61,"dropInstanceID":null,"isClaimed":false}}]}],
             "gameEventDrops":[{"id":"b9"},{"id":"b1"}]}}}"#,
        );
        let c = &inv.campaigns;
        assert_eq!(c[0].game.as_deref(), Some("Rust"));
        assert_eq!(c[0].game_slug.as_deref(), Some("rust"));
        assert_eq!(c[0].drops[0].name, "Hat");
        assert_eq!(c[0].drops[0].benefit_id.as_deref(), Some("b1"));
        assert!(!c[0].drops[0].badge);
        assert!(c[0].drops[0].claimable());
        assert_eq!(c[0].drops[1].name, "Drop 2");
        assert!(!c[0].drops[1].claimable());
        assert_eq!(c[0].drops[1].watched_minutes, 61);
        assert_eq!(inv.earned, ["b9", "b1"]);
    }

    #[test]
    fn v110_f80_explicit_nulls_keep_the_other_rows() {
        let inv = inventory_of(
            r#"{"currentUser":{"inventory":{"dropCampaignsInProgress":[
            {"id":"a","name":null,"timeBasedDrops":null},
            {"id":"b","name":"Fall","allow":{"isEnabled":true,"channels":[{"id":"1","name":null}]},"timeBasedDrops":[
              {"id":"d1","name":null,"requiredMinutesWatched":null,"requiredSubs":null,"benefitEdges":null},
              {"id":"d2","name":"Two","requiredMinutesWatched":30,"benefitEdges":[]}]}]}}}"#,
        );
        assert_eq!(inv.campaigns.len(), 2);
        assert!(inv.campaigns[0].drops.is_empty());
        assert_eq!(inv.campaigns[1].name, "Fall");
        assert_eq!(inv.campaigns[1].drops.len(), 2);
        assert_eq!(inv.campaigns[1].drops[1].name, "Two");
    }

    #[test]
    fn an_empty_inventory_is_empty() {
        assert_eq!(inventory_of(r#"{"currentUser":{"inventory":null}}"#), Inventory::default());
        assert_eq!(inventory_of(r#"{"currentUser":null}"#), Inventory::default());
    }

    #[test]
    fn dashboard_keeps_the_active_campaigns() {
        let d: DashboardData = serde_json::from_str(
            r#"{"currentUser":{"dropCampaigns":[{"id":"a","status":"ACTIVE"},{"id":"b","status":"EXPIRED"},
            {"id":"c","status":"UPCOMING"},{"id":"d","status":"ACTIVE"}]}}"#,
        )
        .unwrap();
        assert_eq!(d.active().unwrap(), ["a", "d"]);
        // No list at all is an error, not an empty day.
        let none: DashboardData = serde_json::from_str(r#"{"currentUser":null}"#).unwrap();
        assert!(none.active().is_err());
        let empty: DashboardData = serde_json::from_str(r#"{"currentUser":{"dropCampaigns":[]}}"#).unwrap();
        assert!(empty.active().unwrap().is_empty());
        // V110-U68 What real Twitch sent for cyexbot, raw and persisted: a user, a null list.
        let null: DashboardData =
            serde_json::from_str(r#"{"currentUser":{"__typename":"User","dropCampaigns":null,"id":"1","login":"x"}}"#).unwrap();
        assert!(null.active().unwrap().is_empty());
    }

    const DETAILS_FIXTURE: &str = r#"{"currentUser":{"dropCampaign":{"id":"c1","name":"Event","status":"ACTIVE",
        "startAt":"2026-11-01T10:00:00Z","endAt":"2026-11-30T10:00:00.000Z",
        "game":{"id":"7","slug":"chess","displayName":"Chess"},
        "allow":{"isEnabled":true,"channels":[{"id":"11","name":"Alpha"},{"id":"12","name":"Beta"}]},
        "timeBasedDrops":[
         {"id":"d1","name":"30 min","requiredMinutesWatched":30,"requiredSubs":0,
          "startAt":"2026-11-01T10:00:00Z","endAt":"2026-11-30T10:00:00Z",
          "benefitEdges":[{"benefit":{"id":"bb1","name":"Event badge","imageAssetURL":"b.png","distributionType":"BADGE"}}]},
         {"id":"d2","name":"Sub drop","requiredMinutesWatched":0,"requiredSubs":1,
          "benefitEdges":[{"benefit":{"id":"bb2","name":"Sub badge","imageAssetURL":"s.png","distributionType":"BADGE"}}]},
         {"id":"d3","name":"Item","requiredMinutesWatched":60,"requiredSubs":0,
          "benefitEdges":[{"benefit":{"id":"bb3","name":"Hat","imageAssetURL":"h.png","distributionType":"DIRECT_ENTITLEMENT"}},
                          {"benefit":{"id":"bb4","name":"Mixed badge","imageAssetURL":"m.png","distributionType":"BADGE"}}]}]}}}"#;

    #[test]
    fn details_parse_badges_allow_list_and_times() {
        let c = campaign_from_details(serde_json::from_str(DETAILS_FIXTURE).unwrap()).unwrap();
        assert_eq!((c.id.as_str(), c.game_id.as_deref(), c.game_slug.as_deref()), ("c1", Some("7"), Some("chess")));
        assert_eq!(
            c.allowed,
            [
                Allowed { id: "11".into(), login: "alpha".into() },
                Allowed { id: "12".into(), login: "beta".into() }
            ]
        );
        assert_eq!((c.starts_at, c.ends_at), (1_793_527_200, 1_796_032_800));
        assert_eq!(c.drops.len(), 3);
        let d = &c.drops[0];
        assert!(d.badge && d.required_subs == 0);
        assert_eq!((d.name.as_str(), d.benefit_id.as_deref(), d.required_minutes), ("Event badge", Some("bb1"), 30));
        assert_eq!(c.drops[1].required_subs, 1);
        // A drop with an item and a badge shows the badge.
        let mixed = &c.drops[2];
        assert!(mixed.badge);
        assert_eq!((mixed.name.as_str(), mixed.benefit_id.as_deref()), ("Mixed badge", Some("bb4")));
    }

    #[test]
    fn details_also_read_the_user_path_and_survive_nulls() {
        let v: Value = serde_json::from_str(&DETAILS_FIXTURE.replace("currentUser", "user")).unwrap();
        assert!(campaign_from_details(v).is_some());
        assert!(campaign_from_details(json!({ "currentUser": { "dropCampaign": null } })).is_none());
        assert!(campaign_from_details(json!({ "currentUser": null })).is_none());
    }

    #[test]
    fn a_disabled_allow_list_means_any_channel() {
        let raw = DETAILS_FIXTURE.replace(r#""isEnabled":true"#, r#""isEnabled":false"#);
        let c = campaign_from_details(serde_json::from_str(&raw).unwrap()).unwrap();
        assert!(c.allowed.is_empty());
        let no_allow = DETAILS_FIXTURE.replace(
            r#""allow":{"isEnabled":true,"channels":[{"id":"11","name":"Alpha"},{"id":"12","name":"Beta"}]},"#,
            "",
        );
        let c = campaign_from_details(serde_json::from_str(&no_allow).unwrap()).unwrap();
        assert!(c.allowed.is_empty());
    }

    #[test]
    fn channel_campaign_ids() {
        let d: ChannelData =
            serde_json::from_str(r#"{"channel":{"viewerDropCampaigns":[{"id":"x"},{"id":"y"}]}}"#).unwrap();
        assert_eq!(d.campaign_ids(), ["x", "y"]);
        let none: ChannelData = serde_json::from_str(r#"{"channel":{"viewerDropCampaigns":null}}"#).unwrap();
        assert!(none.campaign_ids().is_empty());
    }

    #[tokio::test]
    async fn v110_u68_a_refused_raw_query_asks_the_persisted_twin() {
        let refused = || Err::<u32, _>(no_shape("Cannot query field \"dropCampaigns\""));
        assert_eq!(raw_or_persisted(refused(), || async { Ok(7) }).await.unwrap(), 7);
        let both = raw_or_persisted(refused(), || async { Err::<u32, _>(no_shape("PersistedQueryNotFound")) })
            .await
            .unwrap_err();
        assert!(both_refused(&both), "{both}");
        // Rate limits and outages are not refusals: the twin isn't asked.
        let slow = Err::<u32, _>(Error::Api { status: 429, message: "slow".into() });
        let e = raw_or_persisted(slow, || async { Ok(1) }).await.unwrap_err();
        assert!(e.is_rate_limited() && !both_refused(&e));
        assert_eq!(raw_or_persisted(Ok(3), || async { Ok(1) }).await.unwrap(), 3);
    }

    #[test]
    fn v110_u68_answers_without_a_campaign_are_a_refusal() {
        assert!(details_answers(vec![Ok(json!({ "currentUser": null }))]).is_err());
        assert!(details_answers(Vec::new()).unwrap().is_empty());
        let one = details_answers(vec![Ok(serde_json::from_str(DETAILS_FIXTURE).unwrap()), Ok(json!({}))]).unwrap();
        assert_eq!(one.len(), 1);
    }

    #[test]
    fn v110_u68_persisted_shapes_parse_too() {
        let dash = json!({ "currentUser": { "dropCampaigns": [{ "id": "a", "status": "ACTIVE" }] } });
        assert_eq!(parse_dashboard(dash).unwrap(), ["a"]);
        assert!(parse_dashboard(json!({ "currentUser": null })).is_err());
        let ch = json!({ "channel": { "viewerDropCampaigns": [{ "id": "x" }] } });
        assert_eq!(parse_channel(ch).unwrap(), ["x"]);
        let inv = parse_inventory(json!({ "currentUser": { "inventory": { "gameEventDrops": [{ "id": "b" }] } } })).unwrap();
        assert_eq!(inv.earned, ["b"]);
        let user: Value = serde_json::from_str(&DETAILS_FIXTURE.replace("currentUser", "user")).unwrap();
        assert!(details_answers(vec![Ok(user)]).is_ok());
    }

    #[test]
    fn the_query_texts_are_whole() {
        assert_eq!(INVENTORY_BASIC.matches('{').count(), INVENTORY_BASIC.matches('}').count());
        for q in [DETAILS, INVENTORY] {
            assert_eq!(q.matches('{').count(), q.matches('}').count(), "{q}");
            assert!(q.contains("distributionType") && q.contains("requiredSubs"));
        }
        assert!(INVENTORY.contains("gameEventDrops { id }"));
        assert!(!DETAILS.contains("  "), "no stray spaces from line continuations");
    }
}
