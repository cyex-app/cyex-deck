//! 1.6 Who an account follows, for accounts with only a Helix sign-in. Needs
//! `user:read:follows`. Only ids come from here; the rest is public GQL. Also when
//! someone followed a channel you moderate (1.36).

use serde::Deserialize;

use super::{Auth, get};
use crate::http::json;
use crate::{Http, Result};

/// 50 pages of 100. Twitch caps follows far lower for most accounts.
const MAX_PAGES: usize = 50;

#[derive(Deserialize)]
struct Page<T> {
    data: Vec<T>,
    #[serde(default)]
    pagination: Pagination,
}

#[derive(Default, Deserialize)]
struct Pagination {
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct Followed {
    broadcaster_id: String,
}

#[derive(Deserialize)]
struct Stream {
    user_id: String,
}

/// Ids of every channel the user follows, newest follow first.
pub async fn followed_ids(http: &Http, auth: Auth<'_>, user_id: &str) -> Result<Vec<String>> {
    let pages = all::<Followed>(http, auth, "/channels/followed", user_id).await?;
    Ok(pages.into_iter().map(|f| f.broadcaster_id).collect())
}

/// Ids of the followed channels live right now.
pub async fn followed_live_ids(http: &Http, auth: Auth<'_>, user_id: &str) -> Result<Vec<String>> {
    let pages = all::<Stream>(http, auth, "/streams/followed", user_id).await?;
    Ok(pages.into_iter().map(|s| s.user_id).collect())
}

#[derive(Deserialize)]
struct Follower {
    followed_at: String,
}

/// 1.36 When `user_id` followed the channel. Only answered for its broadcaster and mods
/// (`moderator:read:followers`); `None` if they don't follow.
pub async fn followed_at(http: &Http, auth: Auth<'_>, broadcaster_id: &str, user_id: &str) -> Result<Option<String>> {
    let req = get(http, auth, "/channels/followers").query(&[("broadcaster_id", broadcaster_id), ("user_id", user_id)]);
    let page: Page<Follower> = json(req).await?;
    Ok(page.data.into_iter().next().map(|f| f.followed_at))
}

async fn all<T: serde::de::DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    path: &str,
    user_id: &str,
) -> Result<Vec<T>> {
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut req = get(http, auth, path).query(&[("user_id", user_id), ("first", "100")]);
        if let Some(c) = &after {
            req = req.query(&[("after", c)]);
        }
        let page: Page<T> = json(req).await?;
        let done = page.data.is_empty();
        out.extend(page.data);
        after = page.pagination.cursor.filter(|c| !c.is_empty());
        if done || after.is_none() {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_parse() {
        let p: Page<Followed> = serde_json::from_str(
            r#"{"total":2,"data":[{"broadcaster_id":"11","broadcaster_login":"a","broadcaster_name":"A",
            "followed_at":"2022-05-24T22:22:08Z"}],"pagination":{"cursor":"eyJiIjpudWxsfQ"}}"#,
        )
        .unwrap();
        assert_eq!(p.data[0].broadcaster_id, "11");
        assert_eq!(p.pagination.cursor.as_deref(), Some("eyJiIjpudWxsfQ"));
        let last: Page<Stream> =
            serde_json::from_str(r#"{"data":[{"user_id":"5","type":"live"}],"pagination":{}}"#).unwrap();
        assert!(last.pagination.cursor.is_none());
    }
}
