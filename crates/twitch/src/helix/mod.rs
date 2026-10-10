//! Helix (api.twitch.tv/helix). One file per endpoint group.

pub mod blocks;
mod chat;
mod clips;
pub mod eventsub;
mod follows;
pub mod moderation;
pub mod modes;
pub mod roles;
mod users;

pub use chat::{set_chat_color, user_emotes};
pub use clips::{CreatedClip, create_clip};
pub use follows::{followed_at, followed_ids, followed_live_ids};
pub use users::{User, users_by_id};

use reqwest::{Method, RequestBuilder};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::Http;
use crate::http::json;

const BASE: &str = "https://api.twitch.tv/helix";

/// A user token and the client ID it was issued to. Helix needs both to match.
#[derive(Clone, Copy)]
pub struct Auth<'a> {
    pub client_id: &'a str,
    pub token: &'a str,
}

#[derive(Deserialize)]
struct Data<T> {
    data: Vec<T>,
}

/// A list answer: one page and the cursor to the next.
#[derive(Deserialize)]
struct Page<T> {
    data: Vec<T>,
    #[serde(default)]
    pagination: Pagination,
    /// Some lists say how many there are in all.
    #[serde(default)]
    total: Option<u64>,
}

#[derive(Default, Deserialize)]
struct Pagination {
    cursor: Option<String>,
}

fn request(http: &Http, auth: Auth<'_>, method: Method, path: &str) -> RequestBuilder {
    http.0
        .request(method, format!("{BASE}{path}"))
        .header("Client-Id", auth.client_id)
        .bearer_auth(auth.token)
}

fn get(http: &Http, auth: Auth<'_>, path: &str) -> RequestBuilder {
    request(http, auth, Method::GET, path)
}

fn post(http: &Http, auth: Auth<'_>, path: &str) -> RequestBuilder {
    request(http, auth, Method::POST, path)
}

/// Every page of a list, up to `max_pages`. `make` builds the first request; the cursor
/// is added for the rest. Returns the items and the `total` Twitch gave, if any.
async fn pages<T: DeserializeOwned>(
    max_pages: usize,
    make: impl Fn() -> RequestBuilder,
) -> crate::Result<(Vec<T>, Option<u64>)> {
    let mut out = Vec::new();
    let mut total = None;
    let mut after: Option<String> = None;
    for _ in 0..max_pages {
        let mut req = make();
        if let Some(c) = &after {
            req = req.query(&[("after", c)]);
        }
        let page: Page<T> = json(req).await?;
        total = total.or(page.total);
        let done = page.data.is_empty();
        out.extend(page.data);
        after = page.pagination.cursor.filter(|c| !c.is_empty());
        if done || after.is_none() {
            break;
        }
    }
    Ok((out, total))
}

/// The ids a mod tool acts with: the channel, and the moderator acting.
#[derive(Clone, Copy)]
pub struct Room<'a> {
    pub broadcaster_id: &'a str,
    pub moderator_id: &'a str,
}

impl Room<'_> {
    fn query(&self) -> [(&'static str, &str); 2] {
        [("broadcaster_id", self.broadcaster_id), ("moderator_id", self.moderator_id)]
    }
}
