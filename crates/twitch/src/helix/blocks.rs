//! F2, 1.45 The account's Twitch block list, and `/block` and `/unblock`
//! (`user:read:blocked_users`, `user:manage:blocked_users`).

use serde::Deserialize;

use super::{Auth, get};
use crate::http::{empty, json};
use crate::{Http, Result};

/// Twitch lets you block about 1.000 people; 20 pages is plenty.
const MAX_PAGES: usize = 20;

#[derive(Deserialize)]
struct Page {
    data: Vec<Blocked>,
    #[serde(default)]
    pagination: Pagination,
}

#[derive(Default, Deserialize)]
struct Pagination {
    cursor: Option<String>,
}

#[derive(Deserialize)]
struct Blocked {
    user_id: String,
}

/// User ids the account blocked.
pub async fn blocked_ids(http: &Http, auth: Auth<'_>, user_id: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut req =
            get(http, auth, "/users/blocks").query(&[("broadcaster_id", user_id), ("first", "100")]);
        if let Some(c) = &after {
            req = req.query(&[("after", c)]);
        }
        let page: Page = json(req).await?;
        let done = page.data.is_empty();
        out.extend(page.data.into_iter().map(|b| b.user_id));
        after = page.pagination.cursor.filter(|c| !c.is_empty());
        if done || after.is_none() {
            break;
        }
    }
    Ok(out)
}

pub async fn block(http: &Http, auth: Auth<'_>, target_id: &str) -> Result<()> {
    let req = http
        .0
        .put(format!("{}/users/blocks", super::BASE))
        .header("Client-Id", auth.client_id)
        .bearer_auth(auth.token)
        .query(&[("target_user_id", target_id), ("source_context", "chat")]);
    empty(req).await
}

pub async fn unblock(http: &Http, auth: Auth<'_>, target_id: &str) -> Result<()> {
    let req = http
        .0
        .delete(format!("{}/users/blocks", super::BASE))
        .header("Client-Id", auth.client_id)
        .bearer_auth(auth.token)
        .query(&[("target_user_id", target_id)]);
    empty(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_parses() {
        let p: Page = serde_json::from_str(
            r#"{"data":[{"user_id":"135093069","user_login":"bluelava","display_name":"BlueLava"}],"pagination":{"cursor":"x"}}"#,
        )
        .unwrap();
        assert_eq!(p.data[0].user_id, "135093069");
        assert_eq!(p.pagination.cursor.as_deref(), Some("x"));
    }
}
