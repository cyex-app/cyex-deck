//! 1.11 Follow and unfollow. Helix can't do this anymore, so it needs the account's GQL
//! token. Raw mutations, schema-checked 2026/10/09 (moltorino7 and FFZ use the same).

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::{Error, Http, Result};

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

#[cfg(test)]
mod tests {
    use super::*;

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
