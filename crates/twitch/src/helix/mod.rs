//! Helix (api.twitch.tv/helix). One file per endpoint group.

mod clips;
mod users;

pub use clips::{CreatedClip, create_clip};
pub use users::{User, users_by_id};

use reqwest::RequestBuilder;
use serde::Deserialize;

use crate::Http;

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

fn get(http: &Http, auth: Auth<'_>, path: &str) -> RequestBuilder {
    http.0
        .get(format!("{BASE}{path}"))
        .header("Client-Id", auth.client_id)
        .bearer_auth(auth.token)
}

fn post(http: &Http, auth: Auth<'_>, path: &str) -> RequestBuilder {
    http.0
        .post(format!("{BASE}{path}"))
        .header("Client-Id", auth.client_id)
        .bearer_auth(auth.token)
}
