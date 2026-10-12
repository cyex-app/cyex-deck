//! What the points miner (08) asks GQL: a channel's points context, stream info for
//! watch time, and the claims (bonus chests C22, moments, raids) and community goals
//! (W2). Raw queries and mutations, schema-checked without running them (2026/10/10);
//! twitch-channel-points-miner-v2 sends the same as persisted operations, whose hashes
//! keep rotating.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::random::hex;
use crate::{Http, Result};

const CONTEXT: &str = "query($login: String!) { channel(name: $login) { id \
    self { communityPoints { balance availableClaim { id } activeMultipliers { factor } \
    goalContributions { goal { id } userPointsContributedThisStream } } } \
    communityPointsSettings { goals { id title status isInStock pointsContributed amountNeeded \
    perStreamUserMaximumContribution } } } }";
const STREAMS: &str = "query($ids: [ID!]) { users(ids: $ids) { ...U } } \
    fragment U on User { id login displayName profileImageURL(width: 70) stream { id createdAt game { id name } } }";
const STREAMS_BY_LOGIN: &str = "query($logins: [String!]) { users(logins: $logins) { ...U } } \
    fragment U on User { id login displayName profileImageURL(width: 70) stream { id createdAt game { id name } } }";
const CLAIM: &str = "mutation($input: ClaimCommunityPointsInput!) { claimCommunityPoints(input: $input) { \
    currentPoints error { code } } }";
const MOMENT: &str = "mutation($input: ClaimCommunityMomentInput!) { claimCommunityMoment(input: $input) { moment { id } error } }";
const RAID: &str = "mutation($input: JoinRaidInput!) { joinRaid(input: $input) { raidID } }";
const CONTRIBUTE: &str = "mutation($input: ContributeCommunityPointsCommunityGoalInput!) { \
    contributeCommunityPointsCommunityGoal(input: $input) { error { code } } }";

/// A channel's points for the signed-in account.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointsContext {
    pub channel_id: String,
    pub balance: u64,
    /// A bonus chest waiting to be claimed.
    pub claim: Option<String>,
    /// Sum of the active multipliers' factors (subs, gifts), e.g. 0.2.
    pub multiplier: f64,
    pub goals: Vec<Goal>,
}

/// W2 A community goal and what this account put in during this stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Goal {
    pub id: String,
    pub title: String,
    /// STARTED, ENDED, …
    pub status: String,
    pub in_stock: bool,
    pub contributed: u64,
    pub needed: u64,
    pub per_stream_max: u64,
    pub mine_this_stream: u64,
}

/// What watch time (spade) needs about a live stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinerStream {
    /// The broadcast id.
    pub id: String,
    /// RFC 3339.
    pub started_at: String,
    pub game: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinerChannel {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    pub stream: Option<MinerStream>,
}

/// `None` if the channel doesn't exist; the balance is 0 signed out.
pub async fn points_context(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<PointsContext>> {
    let data: ContextData = query(http, auth, CONTEXT, json!({ "login": login })).await?;
    Ok(data.channel.map(PointsContext::from))
}

/// Up to 100 channels by id, with their streams. Public.
pub async fn streams(http: &Http, ids: &[String]) -> Result<Vec<MinerChannel>> {
    let mut out = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(100) {
        let data: StreamsData = query(http, Auth::PUBLIC, STREAMS, json!({ "ids": chunk })).await?;
        out.extend(data.users.into_iter().flatten().map(MinerChannel::from));
    }
    Ok(out)
}

/// W1 The same by login, for streams playing in the app.
pub async fn streams_by_logins(http: &Http, logins: &[String]) -> Result<Vec<MinerChannel>> {
    let mut out = Vec::with_capacity(logins.len());
    for chunk in logins.chunks(100) {
        let data: StreamsData =
            query(http, Auth::PUBLIC, STREAMS_BY_LOGIN, json!({ "logins": chunk })).await?;
        out.extend(data.users.into_iter().flatten().map(MinerChannel::from));
    }
    Ok(out)
}

/// C22 Claims a bonus chest. The new balance, or Twitch's error code.
pub async fn claim_bonus(http: &Http, auth: Auth<'_>, channel_id: &str, claim_id: &str) -> Result<Claimed> {
    let vars = json!({ "input": { "channelID": channel_id, "claimID": claim_id } });
    let data: ClaimData = query(http, auth, CLAIM, vars).await?;
    let p = data.claim_community_points.unwrap_or_default();
    Ok(match p.error {
        Some(e) => Claimed::Refused(e.code),
        None => Claimed::Balance(p.current_points),
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Claimed {
    Balance(Option<u64>),
    Refused(String),
}

/// Claims a moment. Twitch's error code, if any.
pub async fn claim_moment(http: &Http, auth: Auth<'_>, moment_id: &str) -> Result<Option<String>> {
    let data: MomentData = query(http, auth, MOMENT, json!({ "input": { "momentID": moment_id } })).await?;
    Ok(data.claim_community_moment.and_then(|m| m.error))
}

/// Joins a raid, so the account goes along and gets the raid's points.
pub async fn join_raid(http: &Http, auth: Auth<'_>, raid_id: &str) -> Result<()> {
    let _: serde_json::Value = query(http, auth, RAID, json!({ "input": { "raidID": raid_id } })).await?;
    Ok(())
}

/// W2 Puts points toward a goal. Twitch's error code, if any.
pub async fn contribute(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    goal_id: &str,
    amount: u64,
) -> Result<Option<String>> {
    let vars = json!({ "input": {
        "amount": amount, "channelID": channel_id, "goalID": goal_id, "transactionID": hex(16),
    }});
    let data: ContributeData = query(http, auth, CONTRIBUTE, vars).await?;
    Ok(data
        .contribute_community_points_community_goal
        .and_then(|c| c.error)
        .map(|e| e.code))
}

#[derive(Deserialize)]
struct ContextData {
    channel: Option<RawChannel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    id: String,
    #[serde(rename = "self")]
    me: Option<RawSelf>,
    community_points_settings: Option<RawSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSelf {
    community_points: Option<RawPoints>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPoints {
    #[serde(default)]
    balance: u64,
    available_claim: Option<Id>,
    #[serde(default)]
    active_multipliers: Vec<Factor>,
    #[serde(default)]
    goal_contributions: Vec<RawContribution>,
}

#[derive(Deserialize)]
struct Id {
    id: String,
}

#[derive(Deserialize)]
struct Factor {
    #[serde(default)]
    factor: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawContribution {
    goal: Option<Id>,
    #[serde(default)]
    user_points_contributed_this_stream: u64,
}

#[derive(Deserialize)]
struct RawSettings {
    #[serde(default)]
    goals: Vec<RawGoal>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGoal {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    is_in_stock: bool,
    #[serde(default)]
    points_contributed: u64,
    #[serde(default)]
    amount_needed: u64,
    #[serde(default)]
    per_stream_user_maximum_contribution: u64,
}

impl From<RawChannel> for PointsContext {
    fn from(c: RawChannel) -> Self {
        let points = c.me.and_then(|m| m.community_points);
        let mine: HashMap<String, u64> = points
            .as_ref()
            .map(|p| {
                p.goal_contributions
                    .iter()
                    .filter_map(|g| {
                        Some((g.goal.as_ref()?.id.clone(), g.user_points_contributed_this_stream))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let goals = c
            .community_points_settings
            .map(|s| s.goals)
            .unwrap_or_default()
            .into_iter()
            .map(|g| Goal {
                mine_this_stream: mine.get(&g.id).copied().unwrap_or(0),
                id: g.id,
                title: g.title,
                status: g.status,
                in_stock: g.is_in_stock,
                contributed: g.points_contributed,
                needed: g.amount_needed,
                per_stream_max: g.per_stream_user_maximum_contribution,
            })
            .collect();
        Self {
            channel_id: c.id,
            balance: points.as_ref().map_or(0, |p| p.balance),
            claim: points
                .as_ref()
                .and_then(|p| p.available_claim.as_ref())
                .map(|c| c.id.clone()),
            multiplier: points.map_or(0.0, |p| p.active_multipliers.iter().map(|m| m.factor).sum()),
            goals,
        }
    }
}

#[derive(Deserialize)]
struct StreamsData {
    #[serde(default)]
    users: Vec<Option<RawUser>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    login: String,
    display_name: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    stream: Option<RawStream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStream {
    id: String,
    #[serde(default)]
    created_at: String,
    game: Option<RawGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    id: String,
    name: String,
}

impl From<RawUser> for MinerChannel {
    fn from(u: RawUser) -> Self {
        Self {
            name: u.display_name.unwrap_or_else(|| u.login.clone()),
            id: u.id,
            login: u.login,
            avatar: u.profile_image_url,
            stream: u.stream.map(|s| MinerStream {
                id: s.id,
                started_at: s.created_at,
                game: s.game.map(|g| (g.id, g.name)),
            }),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimData {
    claim_community_points: Option<ClaimPayload>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaimPayload {
    current_points: Option<u64>,
    error: Option<Code>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MomentData {
    claim_community_moment: Option<MomentPayload>,
}

#[derive(Deserialize)]
struct MomentPayload {
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributeData {
    contribute_community_points_community_goal: Option<ContributePayload>,
}

#[derive(Deserialize)]
struct ContributePayload {
    error: Option<Code>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_reads_claims_multipliers_and_goals() {
        let raw = r#"{"channel":{"id":"11148817","self":{"communityPoints":{"balance":1234,
            "availableClaim":{"id":"c1"},"activeMultipliers":[{"factor":0.2},{"factor":0.1}],
            "goalContributions":[{"goal":{"id":"g1"},"userPointsContributedThisStream":300}]}},
            "communityPointsSettings":{"goals":[{"id":"g1","title":"Goal","status":"STARTED","isInStock":true,
            "pointsContributed":5000,"amountNeeded":9000,"perStreamUserMaximumContribution":2000}]}}}"#;
        let data: ContextData = serde_json::from_str(raw).unwrap();
        let c = PointsContext::from(data.channel.unwrap());
        assert_eq!((c.balance, c.claim.as_deref()), (1234, Some("c1")));
        assert!((c.multiplier - 0.3).abs() < 1e-9);
        assert_eq!(c.goals[0].mine_this_stream, 300);
        assert_eq!((c.goals[0].needed, c.goals[0].per_stream_max), (9000, 2000));
    }

    #[test]
    fn signed_out_context_is_empty() {
        let raw = r#"{"channel":{"id":"1","self":{"communityPoints":null},"communityPointsSettings":{"goals":[]}}}"#;
        let data: ContextData = serde_json::from_str(raw).unwrap();
        let c = PointsContext::from(data.channel.unwrap());
        assert_eq!((c.balance, c.claim, c.multiplier), (0, None, 0.0));
    }

    #[test]
    fn streams_parse() {
        let raw = r#"{"users":[{"id":"1","login":"a","displayName":"A","profileImageURL":null,"stream":null},
            null,{"id":"2","login":"b","displayName":"B","stream":{"id":"99","createdAt":"2026-10-10T09:09:39Z",
            "game":{"id":"7","name":"Chess","displayName":"Chess"}}}]}"#;
        let data: StreamsData = serde_json::from_str(raw).unwrap();
        let list: Vec<MinerChannel> = data.users.into_iter().flatten().map(MinerChannel::from).collect();
        assert_eq!(list.len(), 2);
        assert!(list[0].stream.is_none());
        let s = list[1].stream.as_ref().unwrap();
        assert_eq!(
            (s.id.as_str(), s.game.clone()),
            ("99", Some(("7".into(), "Chess".into())))
        );
    }

    #[test]
    fn claim_outcomes() {
        let ok: ClaimData =
            serde_json::from_str(r#"{"claimCommunityPoints":{"currentPoints":50,"error":null}}"#).unwrap();
        assert_eq!(ok.claim_community_points.unwrap().current_points, Some(50));
        let no: ClaimData = serde_json::from_str(
            r#"{"claimCommunityPoints":{"currentPoints":null,"error":{"code":"NOT_FOUND"}}}"#,
        )
        .unwrap();
        assert_eq!(
            no.claim_community_points.unwrap().error.unwrap().code,
            "NOT_FOUND"
        );
    }
}
