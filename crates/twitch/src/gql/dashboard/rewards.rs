//! V107-R37 Channel points rewards on your own channel, the way Twitch's dashboard runs
//! them (Viewer Rewards › Channel Points, the stream manager's reward queue). GQL manages
//! every reward; Helix only the ones its own client made, so it isn't used. Operations
//! from Twitch's public dashboard bundles (2026/10/21, docs/research/twitch-api.md):
//! `ChannelPointsCustomRewards`, `CreateCustomChannelPointsReward`,
//! `UpdateCustomChannelPointsReward`, `DeleteCustomChannelPointsReward`,
//! `RedemptionsByRewardID_Paginated`, `BulkUpdateRedemptions`. Sent as raw text.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use ts_rs::TS;

use super::Refusal;
use crate::gql::{Auth, Response, URL, request_at, unwrap};
use crate::http::json as send_json;
use crate::{Error, Http, Result};

const FIELDS: &str = "id title prompt cost pricingType backgroundColor isEnabled isPaused isInStock isSubOnly \
    isUserInputRequired shouldRedemptionsSkipRequestQueue cooldownExpiresAt redemptionsRedeemedCurrentStream \
    image { url url2x } defaultImage { url url2x } \
    maxPerStreamSetting { isEnabled maxPerStream } maxPerUserPerStreamSetting { isEnabled maxPerUserPerStream } \
    globalCooldownSetting { isEnabled globalCooldownSeconds }";

fn list_query() -> String {
    format!(
        "query($id: ID!) {{ user(id: $id) {{ channel {{ communityPointsSettings {{ isEnabled \
         customRewards {{ {FIELDS} }} summarizedRewards {{ count isCountAtMaximum node {{ id }} }} }} }} }} }}"
    )
}

fn create_query() -> String {
    format!(
        "mutation($input: CreateCommunityPointsCustomRewardInput!) {{ createCommunityPointsCustomReward(input: $input) \
         {{ error {{ code }} reward {{ {FIELDS} }} }} }}"
    )
}

fn update_query() -> String {
    format!(
        "mutation($input: UpdateCommunityPointsCustomRewardInput!) {{ updateCommunityPointsCustomReward(input: $input) \
         {{ error {{ code }} reward {{ {FIELDS} }} }} }}"
    )
}

const DELETE: &str = "mutation($input: DeleteCommunityPointsCustomRewardInput!) { \
    deleteCommunityPointsCustomReward(input: $input) { error { code } reward { id } } }";
const QUEUE: &str = "query($id: ID!, $reward: ID, $cursor: Cursor, $count: Int) { user(id: $id) { channel { \
    communityPointsRedemptionQueue(options: { rewardID: $reward, status: UNFULFILLED, order: NEWEST }, \
    first: $count, after: $cursor) { edges { cursor node { id input timestamp reward { id title cost } \
    user { id login displayName } } } pageInfo { hasNextPage } } } } }";
const SET_STATUS: &str = "mutation($input: UpdateCommunityPointsCustomRewardRedemptionStatusesByRedemptionsInput!) { \
    updateCommunityPointsCustomRewardRedemptionStatusesByRedemptions(input: $input) { error { code } } }";

/// Twitch takes 50 a page.
pub const QUEUE_PAGE: u32 = 50;
pub const TITLE_MAX: usize = 45;
pub const PROMPT_MAX: usize = 200;
/// Seven days.
pub const COOLDOWN_MAX: u32 = 604_800;

/// One custom reward as its owner sees it. Only Channel Points ones; Bits ones are
/// power-ups and stay on Twitch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmReward")]
pub struct Reward {
    pub id: String,
    pub title: String,
    pub prompt: String,
    #[ts(type = "number")]
    pub cost: u64,
    /// `#RRGGBB`.
    pub color: String,
    pub image: Option<String>,
    pub enabled: bool,
    pub paused: bool,
    pub in_stock: bool,
    pub sub_only: bool,
    pub input_required: bool,
    pub skip_queue: bool,
    /// Seconds; `None` = no cooldown.
    pub cooldown: Option<u32>,
    pub max_per_stream: Option<u32>,
    pub max_per_user: Option<u32>,
    /// RFC 3339, while it cools down.
    pub cooldown_until: Option<String>,
    pub redeemed_this_stream: Option<u32>,
    /// Redemptions waiting in the queue.
    pub waiting: u32,
    /// `waiting` is Twitch's cap, there are more.
    pub waiting_more: bool,
}

/// The rewards list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmRewards")]
pub struct Rewards {
    /// Channel points are on for the channel (affiliates and partners).
    pub enabled: bool,
    pub rewards: Vec<Reward>,
}

/// What a create or an edit sends.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmRewardEdit")]
pub struct RewardEdit {
    pub title: String,
    pub prompt: String,
    #[ts(type = "number")]
    pub cost: u64,
    pub color: String,
    pub enabled: bool,
    pub input_required: bool,
    pub skip_queue: bool,
    pub cooldown: Option<u32>,
    pub max_per_stream: Option<u32>,
    pub max_per_user: Option<u32>,
}

/// A switch on one reward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind", content = "on")]
#[ts(export, rename = "SmRewardSwitch")]
pub enum Switch {
    Enabled(bool),
    Paused(bool),
}

/// A redemption waiting in the queue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmRedemption")]
pub struct Redemption {
    pub id: String,
    pub reward_id: String,
    pub reward: String,
    #[ts(type = "number")]
    pub cost: u64,
    pub login: Option<String>,
    pub name: Option<String>,
    pub input: Option<String>,
    /// RFC 3339.
    pub at: String,
}

/// A page of the queue, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmQueue")]
pub struct Queue {
    pub items: Vec<Redemption>,
    /// Cursor of the last item when there are more.
    pub next: Option<String>,
}

/// Complete or reject (refund).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[ts(export, rename = "SmRedemptionStatus")]
pub enum Status {
    #[serde(rename = "FULFILLED")]
    Fulfilled,
    #[serde(rename = "CANCELED")]
    Canceled,
}

impl Status {
    fn gql(self) -> &'static str {
        match self {
            Status::Fulfilled => "FULFILLED",
            Status::Canceled => "CANCELED",
        }
    }
}

// ---- wire shapes -------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    id: String,
    #[serde(default)]
    title: String,
    prompt: Option<String>,
    #[serde(default)]
    cost: u64,
    pricing_type: Option<String>,
    background_color: Option<String>,
    #[serde(default)]
    is_enabled: bool,
    #[serde(default)]
    is_paused: bool,
    #[serde(default = "yes")]
    is_in_stock: bool,
    #[serde(default)]
    is_sub_only: bool,
    #[serde(default)]
    is_user_input_required: bool,
    #[serde(default)]
    should_redemptions_skip_request_queue: bool,
    cooldown_expires_at: Option<String>,
    redemptions_redeemed_current_stream: Option<u32>,
    image: Option<Img>,
    default_image: Option<Img>,
    max_per_stream_setting: Option<Limit>,
    max_per_user_per_stream_setting: Option<Limit>,
    global_cooldown_setting: Option<Limit>,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct Img {
    url: Option<String>,
    url2x: Option<String>,
}

/// One of the three limit settings; Twitch names the number differently in each.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Limit {
    #[serde(default)]
    is_enabled: bool,
    #[serde(alias = "maxPerUserPerStream", alias = "globalCooldownSeconds")]
    max_per_stream: Option<u32>,
}

impl Limit {
    fn value(&self) -> Option<u32> {
        self.max_per_stream.filter(|n| self.is_enabled && *n > 0)
    }
}

fn limit(l: Option<&Limit>) -> Option<u32> {
    l.and_then(Limit::value)
}

fn img(i: Option<&Img>) -> Option<String> {
    i.and_then(|i| i.url2x.clone().or_else(|| i.url.clone()))
        .filter(|u| !u.is_empty())
}

fn reward_from(r: Raw, waiting: u32, more: bool) -> Option<Reward> {
    if r.pricing_type.as_deref().is_some_and(|p| p != "POINTS") {
        return None;
    }
    Some(Reward {
        image: img(r.image.as_ref()).or_else(|| img(r.default_image.as_ref())),
        cooldown: limit(r.global_cooldown_setting.as_ref()),
        max_per_stream: limit(r.max_per_stream_setting.as_ref()),
        max_per_user: limit(r.max_per_user_per_stream_setting.as_ref()),
        id: r.id,
        title: r.title,
        prompt: r.prompt.unwrap_or_default(),
        cost: r.cost,
        color: r
            .background_color
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or_else(|| "#9147FF".into()),
        enabled: r.is_enabled,
        paused: r.is_paused,
        in_stock: r.is_in_stock,
        sub_only: r.is_sub_only,
        input_required: r.is_user_input_required,
        skip_queue: r.should_redemptions_skip_request_queue,
        cooldown_until: r.cooldown_expires_at,
        redeemed_this_stream: r.redemptions_redeemed_current_stream,
        waiting,
        waiting_more: more,
    })
}

/// The list answer.
pub fn read_rewards(data: &Value) -> Result<Rewards> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Settings {
        #[serde(default)]
        is_enabled: bool,
        #[serde(default)]
        custom_rewards: Vec<Raw>,
        #[serde(default)]
        summarized_rewards: Vec<Summary>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Summary {
        #[serde(default)]
        count: u32,
        #[serde(default)]
        is_count_at_maximum: bool,
        node: Option<Node>,
    }
    #[derive(Deserialize)]
    struct Node {
        id: String,
    }
    let v = data
        .pointer("/user/channel/communityPointsSettings")
        .cloned()
        .unwrap_or(Value::Null);
    if v.is_null() {
        return Ok(Rewards::default());
    }
    let s: Settings = serde_json::from_value(v).map_err(Error::Decode)?;
    let count = |id: &str| {
        s.summarized_rewards
            .iter()
            .find(|x| x.node.as_ref().is_some_and(|n| n.id == id))
            .map_or((0, false), |x| (x.count, x.is_count_at_maximum))
    };
    let mut rewards = Vec::new();
    for r in s.custom_rewards {
        let (n, more) = count(&r.id);
        rewards.extend(reward_from(r, n, more));
    }
    Ok(Rewards {
        enabled: s.is_enabled,
        rewards,
    })
}

/// A create or update payload: the reward, or the refusal.
pub fn read_saved(data: &Value, field: &str) -> Result<Reward> {
    #[derive(Deserialize)]
    struct Payload {
        error: Option<Refusal>,
        reward: Option<Raw>,
    }
    let p: Payload =
        serde_json::from_value(data.get(field).cloned().unwrap_or(Value::Null)).map_err(Error::Decode)?;
    super::check(p.error)?;
    p.reward
        .and_then(|r| reward_from(r, 0, false))
        .ok_or_else(|| Error::Refused("NO_REWARD".into()))
}

/// The queue answer.
pub fn read_queue(data: &Value) -> Result<Queue> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Conn {
        #[serde(default)]
        edges: Vec<Edge>,
        page_info: Option<PageInfo>,
    }
    #[derive(Deserialize)]
    struct Edge {
        cursor: Option<String>,
        node: Option<Node>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PageInfo {
        #[serde(default)]
        has_next_page: bool,
    }
    #[derive(Deserialize)]
    struct Node {
        id: String,
        input: Option<String>,
        timestamp: Option<String>,
        reward: Option<RewardRef>,
        user: Option<UserRef>,
    }
    #[derive(Deserialize)]
    struct RewardRef {
        id: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        cost: u64,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UserRef {
        login: Option<String>,
        display_name: Option<String>,
    }
    let v = data
        .pointer("/user/channel/communityPointsRedemptionQueue")
        .cloned()
        .unwrap_or(Value::Null);
    if v.is_null() {
        return Ok(Queue::default());
    }
    let c: Conn = serde_json::from_value(v).map_err(Error::Decode)?;
    let more = c.page_info.is_some_and(|p| p.has_next_page);
    let next = more
        .then(|| c.edges.last().and_then(|e| e.cursor.clone()))
        .flatten();
    let items = c
        .edges
        .into_iter()
        .filter_map(|e| e.node)
        .map(|n| {
            let (reward_id, reward, cost) = n
                .reward
                .map_or((String::new(), String::new(), 0), |r| (r.id, r.title, r.cost));
            let (login, name) = n.user.map_or((None, None), |u| (u.login, u.display_name));
            Redemption {
                id: n.id,
                reward_id,
                reward,
                cost,
                login,
                name,
                input: n.input.filter(|s| !s.trim().is_empty()),
                at: n.timestamp.unwrap_or_default(),
            }
        })
        .collect();
    Ok(Queue { items, next })
}

// ---- inputs ------------------------------------------------------------------------

fn setting(on: Option<u32>, key: &str) -> Value {
    let mut m = Map::new();
    m.insert("isEnabled".into(), on.is_some().into());
    m.insert(key.into(), on.unwrap_or(0).into());
    Value::Object(m)
}

/// `CreateCommunityPointsCustomRewardInput` / `UpdateCommunityPointsCustomRewardInput`.
/// `reward` set = an update. A create can't set `isEnabled` (live schema, 2026/10/21): new
/// rewards start on, `enabled: false` is a switch after.
pub fn edit_input(channel_id: &str, reward: Option<&str>, e: &RewardEdit) -> Value {
    let prompt = e.prompt.trim();
    let mut v = json!({
        "channelID": channel_id,
        "title": e.title.trim(),
        "prompt": if prompt.is_empty() { Value::Null } else { prompt.into() },
        "cost": e.cost,
        "backgroundColor": e.color.to_ascii_uppercase(),
        "isUserInputRequired": e.input_required,
        "shouldRedemptionsSkipRequestQueue": e.skip_queue,
        "globalCooldownSetting": setting(e.cooldown, "globalCooldownSeconds"),
        "maxPerStreamSetting": setting(e.max_per_stream, "maxPerStream"),
        "maxPerUserPerStreamSetting": setting(e.max_per_user, "maxPerUserPerStream"),
    });
    match reward {
        Some(id) => {
            v["rewardID"] = id.into();
            v["isEnabled"] = e.enabled.into();
        }
        None => {
            v["isSubOnly"] = false.into();
            v["pricingType"] = "POINTS".into();
        }
    }
    v
}

pub fn switch_input(channel_id: &str, reward: &str, s: Switch) -> Value {
    let (k, on) = match s {
        Switch::Enabled(on) => ("isEnabled", on),
        Switch::Paused(on) => ("isPaused", on),
    };
    json!({ "channelID": channel_id, "rewardID": reward, k: on })
}

/// `waiting`: what happens to its unfulfilled redemptions; Twitch's own default otherwise.
pub fn delete_input(channel_id: &str, reward: &str, waiting: Option<Status>) -> Value {
    let mut v = json!({ "channelID": channel_id, "rewardID": reward });
    if let Some(s) = waiting {
        v["unfulfilledRedemptionsNewStatus"] = s.gql().into();
    }
    v
}

pub fn status_input(channel_id: &str, ids: &[String], s: Status) -> Value {
    json!({ "channelID": channel_id, "redemptionIDs": ids, "newStatus": s.gql(), "oldStatus": "UNFULFILLED" })
}

// ---- calls -------------------------------------------------------------------------

/// Twitch's GQL, or a local mock's `{base}/gql` (tests).
async fn run(http: &Http, auth: Auth<'_>, base: Option<&str>, text: &str, variables: Value) -> Result<Value> {
    let url = base.map_or_else(|| URL.to_owned(), |b| format!("{b}/gql"));
    let req = request_at(http, &url, auth).json(&json!({ "query": text, "variables": variables }));
    let res: Response<Value> = send_json(req).await?;
    unwrap(res)
}

pub async fn list(http: &Http, auth: Auth<'_>, base: Option<&str>, channel_id: &str) -> Result<Rewards> {
    read_rewards(&run(http, auth, base, &list_query(), json!({ "id": channel_id })).await?)
}

pub async fn create(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    e: &RewardEdit,
) -> Result<Reward> {
    let d = run(
        http,
        auth,
        base,
        &create_query(),
        json!({ "input": edit_input(channel_id, None, e) }),
    )
    .await?;
    read_saved(&d, "createCommunityPointsCustomReward")
}

pub async fn update(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    reward: &str,
    e: &RewardEdit,
) -> Result<Reward> {
    let d = run(
        http,
        auth,
        base,
        &update_query(),
        json!({ "input": edit_input(channel_id, Some(reward), e) }),
    )
    .await?;
    read_saved(&d, "updateCommunityPointsCustomReward")
}

pub async fn switch(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    reward: &str,
    s: Switch,
) -> Result<Reward> {
    let d = run(
        http,
        auth,
        base,
        &update_query(),
        json!({ "input": switch_input(channel_id, reward, s) }),
    )
    .await?;
    read_saved(&d, "updateCommunityPointsCustomReward")
}

pub async fn delete(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    reward: &str,
    waiting: Option<Status>,
) -> Result<()> {
    let d = run(
        http,
        auth,
        base,
        DELETE,
        json!({ "input": delete_input(channel_id, reward, waiting) }),
    )
    .await?;
    refusal(&d, "deleteCommunityPointsCustomReward")
}

/// Newest first; `reward` narrows it to one reward.
pub async fn queue(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    reward: Option<&str>,
    cursor: Option<&str>,
) -> Result<Queue> {
    let vars = json!({ "id": channel_id, "reward": reward, "cursor": cursor, "count": QUEUE_PAGE });
    read_queue(&run(http, auth, base, QUEUE, vars).await?)
}

/// Completes or rejects (refunds) redemptions, at most 50 per call, as Twitch's queue does.
pub async fn set_status(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    ids: &[String],
    s: Status,
) -> Result<()> {
    let d = run(
        http,
        auth,
        base,
        SET_STATUS,
        json!({ "input": status_input(channel_id, ids, s) }),
    )
    .await?;
    refusal(
        &d,
        "updateCommunityPointsCustomRewardRedemptionStatusesByRedemptions",
    )
}

fn refusal(data: &Value, field: &str) -> Result<()> {
    #[derive(Deserialize)]
    struct Payload {
        error: Option<Refusal>,
    }
    match data.get(field) {
        None | Some(Value::Null) => Ok(()),
        Some(p) => super::check(
            serde_json::from_value::<Payload>(p.clone())
                .map_err(Error::Decode)?
                .error,
        ),
    }
}

#[cfg(test)]
mod tests;
