//! 1.11 Follow and unfollow. Helix can't do this anymore, so it needs the account's GQL
//! token. Raw mutations, schema-checked 2026/10/09 (moltorino7 and FFZ use the same).
//! 1.6 Who the account follows, for home: raw `currentUser` queries, schema-checked
//! 2026/10/10. 2.2 Live alerts read the same list with each channel's Twitch bell.

use serde::Deserialize;
use serde_json::json;

use super::cards::{BOX_ART, CARD, Category, ChannelCard, RawCard, RawCategory, RawConnection};
use super::{Auth, persisted, query};
use crate::{Error, Http, Result};

/// 50 pages of 100.
const MAX_PAGES: usize = 50;

const FOLLOWING: &str = "query($id: ID!) { user(id: $id) { self { follower { followedAt } } } }";
const FOLLOW: &str = "mutation($id: ID!) { followUser(input: {targetID: $id, disableNotifications: false}) { \
    follow { followedAt } error { code } } }";
const UNFOLLOW: &str = "mutation($id: ID!) { unfollowUser(input: {targetID: $id}) { follow { followedAt } } }";

#[derive(Deserialize)]
struct FollowingData {
    user: Option<FollowingUser>,
}

#[derive(Deserialize)]
struct FollowingUser {
    #[serde(rename = "self")]
    me: Option<SelfEdge>,
}

#[derive(Deserialize)]
struct SelfEdge {
    follower: Option<serde_json::Value>,
}

/// Whether the token's account follows the channel. `None` if the channel is gone.
pub async fn is_following(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Option<bool>> {
    let data: FollowingData = query(http, auth, FOLLOWING, json!({ "id": channel_id })).await?;
    Ok(data
        .user
        .map(|u| u.me.and_then(|s| s.follower).is_some_and(|f| !f.is_null())))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FollowData {
    follow_user: Option<FollowPayload>,
}

#[derive(Deserialize)]
struct FollowPayload {
    error: Option<FollowError>,
}

#[derive(Deserialize)]
struct FollowError {
    code: String,
}

pub async fn follow(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    let data: FollowData = query(http, auth, FOLLOW, json!({ "id": channel_id })).await?;
    match data.follow_user.and_then(|p| p.error) {
        Some(e) => Err(Error::Api {
            status: 200,
            message: e.code,
        }),
        None => Ok(()),
    }
}

pub async fn unfollow(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<()> {
    let _: serde_json::Value = query(http, auth, UNFOLLOW, json!({ "id": channel_id })).await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Me<T> {
    current_user: Option<T>,
}

/// twitch.tv's own query for the following page. The raw `follows` field answers
/// "service error" (checked 2026/10/10); this one has been the same since 2023.
const CHANNEL_FOLLOWS: (&str, &str) = ("ChannelFollows", "eecf815273d3d949e5cf0085cc5084cd8a1b5b7b6f7990cf43cb0beadf546907");

#[derive(Deserialize)]
struct FollowsData {
    user: Option<FollowsUser>,
}

#[derive(Deserialize)]
struct FollowsUser {
    follows: Option<RawConnection<IdNode>>,
}

#[derive(Deserialize)]
struct IdNode {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveFollows {
    followed_live_users: Option<RawConnection<RawCard>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FollowedGames {
    followed_games: Option<Nodes>,
}

#[derive(Deserialize)]
struct Nodes {
    #[serde(default)]
    nodes: Vec<Option<RawCategory>>,
}

/// 1.6 Ids of every channel the account follows, newest follow first. `None` if the
/// token doesn't say who it is.
pub async fn followed_ids(http: &Http, auth: Auth<'_>) -> Result<Option<Vec<String>>> {
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let vars = json!({ "limit": 100, "order": "DESC", "cursor": cursor });
        let data: FollowsData = persisted(http, auth, CHANNEL_FOLLOWS.0, CHANNEL_FOLLOWS.1, vars).await?;
        let Some(user) = data.user else { return Ok(None) };
        let Some(page) = user.follows.map(|c| c.into_page(|n| Some(n.id))) else { break };
        out.extend(page.items);
        cursor = page.cursor;
        if cursor.is_none() {
            break;
        }
    }
    Ok(Some(out))
}

/// 1.6 The followed channels live right now. Only `sort: RELEVANCE` answers; the
/// others say "service error" (checked 2026/10/10).
pub async fn followed_live(http: &Http, auth: Auth<'_>) -> Result<Option<Vec<ChannelCard>>> {
    let q = format!(
        "query($after: Cursor) {{ currentUser {{ followedLiveUsers(first: 100, after: $after, sort: RELEVANCE) {{          edges {{ cursor node {{ ...Card }} }} pageInfo {{ hasNextPage }} }} }} }} {CARD}"
    );
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let data: Me<LiveFollows> = query(http, auth, &q, json!({ "after": after })).await?;
        let Some(me) = data.current_user else { return Ok(None) };
        let Some(page) = me.followed_live_users.map(|c| c.into_page(|n| Some(ChannelCard::from(n)))) else {
            break;
        };
        out.extend(page.items);
        after = page.cursor;
        if after.is_none() {
            break;
        }
    }
    Ok(Some(out))
}

/// 2.2 A followed channel live now, and whether its Twitch bell (live notifications) is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveAlert {
    pub card: ChannelCard,
    pub bell: bool,
}

#[derive(Deserialize)]
struct BellNode {
    #[serde(flatten)]
    card: RawCard,
    #[serde(rename = "self")]
    me: Option<BellSelf>,
}

#[derive(Deserialize)]
struct BellSelf {
    follower: Option<BellFollower>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BellFollower {
    notification_settings: Option<BellSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BellSettings {
    #[serde(default)]
    is_enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveBells {
    followed_live_users: Option<RawConnection<BellNode>>,
}

/// 2.2 Followed channels live now with their bell state, for live alerts. One call
/// answers both, the way Xtra decides its alerts.
pub async fn followed_live_alerts(http: &Http, auth: Auth<'_>) -> Result<Option<Vec<LiveAlert>>> {
    let q = format!(
        "query($after: Cursor) {{ currentUser {{ followedLiveUsers(first: 100, after: $after, sort: RELEVANCE) {{          edges {{ cursor node {{ ...Card self {{ follower {{ notificationSettings {{ isEnabled }} }} }} }} }}          pageInfo {{ hasNextPage }} }} }} }} {CARD}"
    );
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let data: Me<LiveBells> = query(http, auth, &q, json!({ "after": after })).await?;
        let Some(me) = data.current_user else { return Ok(None) };
        let Some(page) = me.followed_live_users.map(|c| {
            c.into_page(|n| {
                let bell = n
                    .me
                    .and_then(|s| s.follower)
                    .and_then(|f| f.notification_settings)
                    .is_some_and(|s| s.is_enabled);
                Some(LiveAlert {
                    card: ChannelCard::from(n.card),
                    bell,
                })
            })
        }) else {
            break;
        };
        out.extend(page.items);
        after = page.cursor;
        if after.is_none() {
            break;
        }
    }
    Ok(Some(out))
}

/// H4 Categories the account follows.
pub async fn followed_categories(http: &Http, auth: Auth<'_>) -> Result<Vec<Category>> {
    let q = format!(
        "{{ currentUser {{ followedGames(first: 100, type: ALL) {{ nodes {{ id slug displayName viewersCount followersCount {BOX_ART} }} }} }} }}"
    );
    let data: Me<FollowedGames> = query(http, auth, &q, json!({})).await?;
    Ok(data
        .current_user
        .and_then(|u| u.followed_games)
        .map(|n| n.nodes.into_iter().flatten().map(Category::from).collect())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_follows_parse() {
        let d: FollowsData = serde_json::from_str(
            r#"{"user":{"id":"9","follows":{"edges":[{"cursor":"c1","node":{"id":"1","login":"a","displayName":"A",
              "profileImageURL":"p","self":{"follower":{"followedAt":"2026-01-01T00:00:00Z"}}}}],
              "pageInfo":{"hasNextPage":true}}}}"#,
        )
        .unwrap();
        let page = d.user.unwrap().follows.unwrap().into_page(|n| Some(n.id));
        assert_eq!((page.items, page.cursor.as_deref()), (vec!["1".to_owned()], Some("c1")));
    }

    #[test]
    fn following_reads_the_edge() {
        let yes: FollowingData =
            serde_json::from_str(r#"{"user":{"self":{"follower":{"followedAt":"2020-01-01T00:00:00Z"}}}}"#).unwrap();
        assert!(yes.user.unwrap().me.unwrap().follower.is_some());
        let no: FollowingData = serde_json::from_str(r#"{"user":{"self":{"follower":null}}}"#).unwrap();
        assert!(no.user.unwrap().me.unwrap().follower.is_none());
        let err: FollowData =
            serde_json::from_str(r#"{"followUser":{"follow":null,"error":{"code":"TOO_MANY_FOLLOWS"}}}"#).unwrap();
        assert_eq!(err.follow_user.unwrap().error.unwrap().code, "TOO_MANY_FOLLOWS");
    }
}
