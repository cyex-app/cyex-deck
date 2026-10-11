use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

const USER_BY_ID: &str =
    "query($id: ID!) { user(id: $id) { id login displayName profileImageURL(width: 70) } }";

const PROFILE: &str = "query($login: String, $id: ID) { user(login: $login, id: $id, lookupType: ALL) { \
    id login displayName description createdAt profileImageURL(width: 150) bannerImageURL chatColor \
    roles { isAffiliate isPartner isStaff } followers { totalCount } } }";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GqlUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "profileImageURL", default)]
    pub profile_image_url: Option<String>,
}

#[derive(Deserialize)]
struct Data {
    user: Option<GqlUser>,
}

/// `None` if the user doesn't exist or is banned.
pub async fn user_by_id(http: &Http, auth: Auth<'_>, id: &str) -> Result<Option<GqlUser>> {
    let data: Data = query(http, auth, USER_BY_ID, json!({ "id": id })).await?;
    Ok(data.user)
}

/// Public profile for the usercard (1.36).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Profile {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub bio: String,
    /// RFC 3339.
    pub created_at: String,
    pub avatar: Option<String>,
    /// V101-05 The profile banner, behind the usercard's header.
    pub banner: Option<String>,
    pub color: Option<String>,
    pub partner: bool,
    pub affiliate: bool,
    pub staff: bool,
    #[ts(type = "number | null")]
    pub followers: Option<u64>,
}

/// By login or by id. `None` if there's no such user.
pub async fn profile(http: &Http, login: Option<&str>, id: Option<&str>) -> Result<Option<Profile>> {
    let data: RawProfileData = query(http, Auth::PUBLIC, PROFILE, json!({ "login": login, "id": id })).await?;
    Ok(data.user.map(|u| {
        let roles = u.roles.unwrap_or_default();
        Profile {
            id: u.id,
            login: u.login,
            display_name: u.display_name,
            bio: u.description.unwrap_or_default(),
            created_at: u.created_at.unwrap_or_default(),
            avatar: u.profile_image_url,
            banner: u.banner_image_url.filter(|b| !b.is_empty()),
            color: u.chat_color.filter(|c| !c.is_empty()),
            partner: roles.is_partner.unwrap_or(false),
            affiliate: roles.is_affiliate.unwrap_or(false),
            staff: roles.is_staff.unwrap_or(false),
            followers: u.followers.map(|f| f.total_count),
        }
    }))
}

#[derive(Deserialize)]
struct RawProfileData {
    user: Option<RawProfile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProfile {
    id: String,
    login: String,
    display_name: String,
    description: Option<String>,
    created_at: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    #[serde(rename = "bannerImageURL", default)]
    banner_image_url: Option<String>,
    chat_color: Option<String>,
    roles: Option<RawRoles>,
    followers: Option<RawFollowers>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRoles {
    is_affiliate: Option<bool>,
    is_partner: Option<bool>,
    is_staff: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFollowers {
    total_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_parses() {
        let d: Data = serde_json::from_str(
            r#"{"user":{"id":"12826","login":"twitch","displayName":"Twitch","profileImageURL":"https://x/p-70x70.png"}}"#,
        )
        .unwrap();
        let u = d.user.unwrap();
        assert_eq!(u.display_name, "Twitch");
        assert_eq!(u.profile_image_url.as_deref(), Some("https://x/p-70x70.png"));
    }

    #[test]
    fn profile_parses_nullable_roles() {
        let d: RawProfileData = serde_json::from_str(
            r##"{"user":{"id":"12826","login":"twitch","displayName":"Twitch","description":"hi","createdAt":"2007-05-22T10:39:54Z",
              "profileImageURL":"a","chatColor":"#8A2BE2","roles":{"isAffiliate":false,"isPartner":true,"isStaff":null},
              "followers":{"totalCount":5}}}"##,
        )
        .unwrap();
        let u = d.user.unwrap();
        assert!(u.roles.as_ref().unwrap().is_staff.is_none());
        assert_eq!(u.followers.unwrap().total_count, 5);
    }
}
