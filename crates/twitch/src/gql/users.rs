use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

const USER_BY_ID: &str =
    "query($id: ID!) { user(id: $id) { id login displayName profileImageURL(width: 70) } }";

const PROFILE: &str = "id login displayName description createdAt profileImageURL(width: 150) bannerImageURL \
    chatColor roles { isAffiliate isPartner isStaff } followers { totalCount }";

/// V102-25 Why Twitch hides the account, asked next to the profile. `lookupType: ALL`
/// still finds suspended and closed accounts (checked 2026/10/11); this says which.
const WHY: &str = "{ __typename ... on UserDoesNotExist { reason } }";

/// By login or by id; Twitch's `userResult` fields want the one that's set.
fn profile_query(by_id: bool) -> String {
    let (var, arg, why) = if by_id {
        ("$id: ID!", "id: $id", "userResultByID(id: $id)")
    } else {
        (
            "$login: String!",
            "login: $login",
            "userResultByLogin(login: $login)",
        )
    };
    format!("query({var}) {{ user({arg}, lookupType: ALL) {{ {PROFILE} }} why: {why} {WHY} }}")
}

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

#[derive(Deserialize)]
struct Ids {
    #[serde(default)]
    users: Vec<Option<IdLogin>>,
}

#[derive(Deserialize)]
struct IdLogin {
    id: String,
    login: String,
}

/// V107-R21 `(login, id)` for each login Twitch knows, 100 per call, public. Suspended and
/// unknown logins come back `null` and are left out.
pub async fn ids_by_login(http: &Http, logins: &[String]) -> Result<Vec<(String, String)>> {
    const Q: &str = "query($logins: [String!]) { users(logins: $logins) { id login } }";
    let mut out = Vec::with_capacity(logins.len());
    for chunk in logins.chunks(100) {
        let data: Ids = query(http, Auth::PUBLIC, Q, json!({ "logins": chunk })).await?;
        out.extend(data.users.into_iter().flatten().map(|u| (u.login, u.id)));
    }
    Ok(out)
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
    /// V102-25 Hidden on Twitch: suspended, taken down or closed. `None` for everyone else.
    pub suspended: Option<Suspension>,
}

/// V102-25 Why Twitch hides an account (`UserDoesNotExist.reason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Suspension {
    /// `TOS_TEMPORARY`
    Temporary,
    /// `TOS_INDEFINITE`
    Indefinite,
    /// `DMCA`
    Dmca,
    /// `DEACTIVATED`: closed by its owner.
    Deactivated,
    /// A reason Twitch added since.
    Other,
}

impl Suspension {
    /// `UNKNOWN` is Twitch's answer for a name nobody has, so it's no reason.
    pub fn from_reason(reason: &str) -> Option<Self> {
        Some(match reason {
            "TOS_TEMPORARY" => Self::Temporary,
            "TOS_INDEFINITE" => Self::Indefinite,
            "DMCA" => Self::Dmca,
            "DEACTIVATED" => Self::Deactivated,
            "" | "UNKNOWN" => return None,
            _ => Self::Other,
        })
    }
}

/// By login or by id. `None` if there's no such user.
pub async fn profile(http: &Http, login: Option<&str>, id: Option<&str>) -> Result<Option<Profile>> {
    let (q, vars) = match (login, id) {
        (Some(login), _) => (profile_query(false), json!({ "login": login })),
        (None, Some(id)) => (profile_query(true), json!({ "id": id })),
        (None, None) => return Ok(None),
    };
    let data: RawProfileData = query(http, Auth::PUBLIC, &q, vars).await?;
    Ok(read_profile(data))
}

fn read_profile(data: RawProfileData) -> Option<Profile> {
    let suspended = data.why.and_then(|w| w.suspension());
    data.user.map(|u| {
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
            suspended,
        }
    })
}

#[derive(Deserialize)]
struct RawProfileData {
    user: Option<RawProfile>,
    #[serde(default)]
    why: Option<RawWhy>,
}

/// `User` when the account is fine, `UserDoesNotExist` with a reason when it's hidden.
#[derive(Deserialize)]
struct RawWhy {
    #[serde(rename = "__typename")]
    kind: String,
    reason: Option<String>,
}

impl RawWhy {
    fn suspension(&self) -> Option<Suspension> {
        if self.kind != "UserDoesNotExist" {
            return None;
        }
        Suspension::from_reason(self.reason.as_deref().unwrap_or_default())
    }
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

    #[test]
    fn suspension_reasons() {
        assert_eq!(
            Suspension::from_reason("TOS_TEMPORARY"),
            Some(Suspension::Temporary)
        );
        assert_eq!(
            Suspension::from_reason("TOS_INDEFINITE"),
            Some(Suspension::Indefinite)
        );
        assert_eq!(Suspension::from_reason("DMCA"), Some(Suspension::Dmca));
        assert_eq!(
            Suspension::from_reason("DEACTIVATED"),
            Some(Suspension::Deactivated)
        );
        assert_eq!(
            Suspension::from_reason("TOS_SOMETHING_NEW"),
            Some(Suspension::Other)
        );
        assert_eq!(Suspension::from_reason("UNKNOWN"), None, "nobody has the name");
        assert_eq!(Suspension::from_reason(""), None);
    }

    /// V102-25 Shapes from live anonymous reads, 2026/10/11.
    #[test]
    fn suspended_profiles_still_read() {
        let raw = |user: &str, why: &str| -> RawProfileData {
            serde_json::from_str(&format!(r#"{{"user":{user},"why":{why}}}"#)).unwrap()
        };
        let user = r#"{"id":"15950","login":"diddy","displayName":"diddy","description":null,
            "createdAt":"2007-05-22T10:40:26.603601Z","profileImageURL":"a","bannerImageURL":null,"chatColor":null,
            "roles":{"isAffiliate":false,"isPartner":false,"isStaff":null},"followers":{"totalCount":0}}"#;
        let p = read_profile(raw(
            user,
            r#"{"__typename":"UserDoesNotExist","reason":"TOS_INDEFINITE"}"#,
        ))
        .unwrap();
        assert_eq!(
            (p.id.as_str(), p.suspended),
            ("15950", Some(Suspension::Indefinite))
        );
        let p = read_profile(raw(
            user,
            r#"{"__typename":"UserDoesNotExist","reason":"DEACTIVATED"}"#,
        ))
        .unwrap();
        assert_eq!(p.suspended, Some(Suspension::Deactivated));
        let p = read_profile(raw(user, r#"{"__typename":"User"}"#)).unwrap();
        assert_eq!(p.suspended, None);
        // Nobody by that name.
        let none = raw("null", r#"{"__typename":"UserDoesNotExist","reason":"UNKNOWN"}"#);
        assert!(read_profile(none).is_none());
        // An answer without `why` still reads.
        let old: RawProfileData = serde_json::from_str(&format!(r#"{{"user":{user}}}"#)).unwrap();
        assert_eq!(read_profile(old).unwrap().suspended, None);
    }

    #[test]
    fn queries_ask_by_what_is_set() {
        let by_login = profile_query(false);
        assert!(by_login.contains("user(login: $login, lookupType: ALL)"));
        assert!(by_login.contains("why: userResultByLogin(login: $login)"));
        let by_id = profile_query(true);
        assert!(by_id.starts_with("query($id: ID!)"));
        assert!(by_id.contains("why: userResultByID(id: $id)"));
    }
}
