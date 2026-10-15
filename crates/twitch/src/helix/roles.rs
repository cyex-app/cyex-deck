//! Who is who in a channel: channels an account moderates (1.57), mods and VIPs (1.50,
//! broadcaster only, `channel:manage:moderators` / `channel:manage:vips`), chatters and
//! followers (1.48, mods). Also the broadcaster's raids, ads and markers.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, Data, Room, get, pages, post, request};
use crate::http::{empty, json};
use crate::{Http, Result};

/// Someone in a list.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export)]
pub struct ListedUser {
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
}

/// Mods of the broadcaster's own channel (`moderation:read`).
pub async fn moderators(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<Vec<ListedUser>> {
    let make = || get(http, auth, "/moderation/moderators").query(&[("broadcaster_id", broadcaster_id), ("first", "100")]);
    Ok(pages(20, make).await?.0)
}

/// VIPs of the broadcaster's own channel (`channel:read:vips`).
pub async fn vips(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<Vec<ListedUser>> {
    let make = || get(http, auth, "/channels/vips").query(&[("broadcaster_id", broadcaster_id), ("first", "100")]);
    Ok(pages(20, make).await?.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Mod,
    Vip,
}

/// 1.50 Gives or takes a role. Twitch allows about 10 changes per 10 s.
pub async fn set_role(http: &Http, auth: Auth<'_>, broadcaster_id: &str, user_id: &str, role: Role, on: bool) -> Result<()> {
    let path = match role {
        Role::Mod => "/moderation/moderators",
        Role::Vip => "/channels/vips",
    };
    let method = if on { Method::POST } else { Method::DELETE };
    let req = request(http, auth, method, path).query(&[("broadcaster_id", broadcaster_id), ("user_id", user_id)]);
    empty(req).await
}

/// 1.48 Everyone in the chat right now (mods only), and how many Twitch counts.
pub async fn chatters(http: &Http, auth: Auth<'_>, room: Room<'_>) -> Result<(Vec<ListedUser>, u64)> {
    let make = || get(http, auth, "/chat/chatters").query(&room.query()).query(&[("first", "1000")]);
    let (list, total) = pages::<ListedUser>(10, make).await?;
    let n = total.unwrap_or(list.len() as u64);
    Ok((list, n))
}

/// A follower and when they followed.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export)]
pub struct Follower {
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    /// RFC 3339.
    pub followed_at: String,
}

/// 1.48 The newest followers, newest first (`moderator:read:followers`), up to `max`.
pub async fn followers(http: &Http, auth: Auth<'_>, broadcaster_id: &str, max: usize) -> Result<Vec<Follower>> {
    let make = || get(http, auth, "/channels/followers").query(&[("broadcaster_id", broadcaster_id), ("first", "100")]);
    let (mut list, _) = pages::<Follower>(max.div_ceil(100).max(1), make).await?;
    list.truncate(max);
    Ok(list)
}

/// Raids `to_id` from the broadcaster's channel (`channel:manage:raids`).
pub async fn raid(http: &Http, auth: Auth<'_>, from_id: &str, to_id: &str) -> Result<()> {
    let req = post(http, auth, "/raids").query(&[("from_broadcaster_id", from_id), ("to_broadcaster_id", to_id)]);
    empty(req).await
}

pub async fn unraid(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<()> {
    let req = request(http, auth, Method::DELETE, "/raids").query(&[("broadcaster_id", broadcaster_id)]);
    empty(req).await
}

#[derive(Deserialize)]
struct Commercial {
    length: u32,
}

/// Runs an ad of about `secs` (30 to 180); returns the length Twitch picked.
pub async fn commercial(http: &Http, auth: Auth<'_>, broadcaster_id: &str, secs: u32) -> Result<u32> {
    let req = post(http, auth, "/channels/commercial")
        .json(&json!({ "broadcaster_id": broadcaster_id, "length": secs.clamp(30, 180) }));
    let d: Data<Commercial> = json(req).await?;
    Ok(d.data.first().map_or(secs, |c| c.length))
}

/// A stream marker at the current spot (`channel:manage:broadcast`).
pub async fn marker(http: &Http, auth: Auth<'_>, broadcaster_id: &str, description: &str) -> Result<()> {
    let req = post(http, auth, "/streams/markers")
        .json(&json!({ "user_id": broadcaster_id, "description": description.chars().take(140).collect::<String>() }));
    empty(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::Page;

    #[test]
    fn lists_parse() {
        let p: Page<ListedUser> = serde_json::from_str(
            r#"{"data":[{"user_id":"128393656","user_login":"smittysmithers","user_name":"smittysmithers"}],
            "pagination":{},"total":8}"#,
        )
        .unwrap();
        assert_eq!((p.data[0].user_login.as_str(), p.total), ("smittysmithers", Some(8)));
        let f: Page<Follower> = serde_json::from_str(
            r#"{"total":8,"data":[{"user_id":"11","user_name":"UserDisplayName","user_login":"userloginname",
            "followed_at":"2022-05-24T22:22:08Z"}],"pagination":{"cursor":"x"}}"#,
        )
        .unwrap();
        assert_eq!(f.data[0].followed_at, "2022-05-24T22:22:08Z");
    }
}
