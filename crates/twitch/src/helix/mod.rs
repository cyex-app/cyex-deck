//! Helix (api.twitch.tv/helix). One file per endpoint group.

pub mod ads;
pub mod blocks;
pub mod channel;
mod chat;
mod clips;
pub mod eventsub;
mod follows;
pub mod gueststar;
pub mod moderation;
pub mod modes;
pub mod roles;
pub mod stats;
mod users;
pub mod votes;
pub mod whispers;

pub use chat::{send_message, set_chat_color, user_emotes};
pub use clips::{CreatedClip, create_clip};
pub use follows::{followed_at, followed_ids, followed_live_ids};
pub use users::{User, users_by_id, users_by_login};

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
    // P4 #16 A mocked client: the mock's `/helix`, and never the real sign-in.
    let (url, client_id, token) = match http.mock_base() {
        Some(b) => (format!("{b}/helix{path}"), crate::http::MOCK_CLIENT, crate::http::MOCK_TOKEN),
        None => (format!("{BASE}{path}"), auth.client_id, auth.token),
    };
    http.0.request(method, url).header("Client-Id", client_id).bearer_auth(token)
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

#[cfg(test)]
mod tests {
    use super::*;

    const REAL: Auth<'static> = Auth { client_id: "real-client", token: "real-token" };

    /// Every header and the URL of a built request, as one string.
    fn seen(req: RequestBuilder) -> String {
        let r = req.build().unwrap();
        let headers: Vec<String> = r.headers().iter().map(|(k, v)| format!("{k}={}", v.to_str().unwrap())).collect();
        format!("{} {}", r.url(), headers.join(" "))
    }

    #[test]
    fn p4_16_a_mocked_client_sends_nothing_real() {
        let http = Http::new().unwrap().mocked("http://127.0.0.1:5240/");
        let h = seen(get(&http, REAL, "/moderation/blocked_terms"));
        assert!(h.starts_with("http://127.0.0.1:5240/helix/moderation/blocked_terms "), "{h}");
        assert!(h.contains("authorization=Bearer mock-token") && h.contains("client-id=mock-client"), "{h}");
        let gql = crate::gql::Auth { client_id: crate::clients::GQL, token: Some("real-token") };
        let g = seen(crate::gql::request_at(&http, "https://gql.twitch.tv/gql", gql));
        assert!(g.starts_with("http://127.0.0.1:5240/gql "), "{g}");
        assert!(g.contains("authorization=OAuth mock-token"), "{g}");
        for s in [&h, &g] {
            assert!(!s.contains("real-") && !s.contains("twitch.tv"), "{s}");
        }
    }

    #[test]
    fn p4_16_a_plain_client_goes_to_twitch() {
        let http = Http::new().unwrap();
        assert_eq!(http.mock_base(), None);
        let h = seen(get(&http, REAL, "/users"));
        assert!(h.starts_with("https://api.twitch.tv/helix/users ") && h.contains("Bearer real-token"), "{h}");
    }
}
