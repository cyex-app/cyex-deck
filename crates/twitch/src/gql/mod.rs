//! GQL (gql.twitch.tv), Twitch's private API. Can change without notice. Persisted query
//! hashes live next to the operation that uses them; raw queries are used where they work.

mod channels;
mod chat;
mod follows;
mod points;
mod users;

pub use channels::{ChannelInfo, StreamInfo, channels_by_ids, lookup};

pub use chat::{
    BadgeInfo, ChannelChat, CheerTier, Cheermote, GlobalChat, PinFragment, PinSender, PinnedMessage, channel_chat,
    global_chat, pinned,
};
pub use points::{
    AutoReward, CustomReward, Points, Redeemed, balance, redeem_custom, send_highlighted, send_in_sub_mode,
    unlock_random_emote,
};
pub use follows::{follow, is_following, unfollow};
pub use users::{GqlUser, Profile, profile, user_by_id};

use std::sync::OnceLock;

use reqwest::RequestBuilder;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::http::json as send_json;
use crate::random::hex;
use crate::{Error, Http, Result, clients};

const URL: &str = "https://gql.twitch.tv/gql";
/// Fallback for the `Client-Version` twitch.tv sends; GQL accepts a stale one.
const CLIENT_VERSION: &str = "ef928475-9403-42f2-8a34-55784bd08e16";
const TV_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 7.1; Smart Box C1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";

/// Client ID plus an optional user token. Without a token, only public data is readable.
#[derive(Clone, Copy)]
pub struct Auth<'a> {
    pub client_id: &'a str,
    pub token: Option<&'a str>,
}

impl Auth<'static> {
    /// Public data, as twitch.tv reads it signed out.
    pub const PUBLIC: Auth<'static> = Auth {
        client_id: clients::WEB,
        token: None,
    };
}

#[derive(Deserialize)]
struct Response<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

/// Runs a raw query.
pub(crate) async fn query<T: DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    query: &str,
    variables: Value,
) -> Result<T> {
    let req = request(http, auth).json(&json!({ "query": query, "variables": variables }));
    unwrap(send_json(req).await?)
}

/// Runs a persisted query by its hash.
pub(crate) async fn persisted<T: DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    operation: &str,
    hash: &str,
    variables: Value,
) -> Result<T> {
    let req = request(http, auth).json(&json!({
        "operationName": operation,
        "variables": variables,
        "extensions": { "persistedQuery": { "version": 1, "sha256Hash": hash } },
    }));
    unwrap(send_json(req).await?)
}

/// Signed-in requests look like the Android TV app's: the tools that redeem rewards with
/// its tokens send these headers and need no integrity token (research, phase 3).
fn request(http: &Http, auth: Auth<'_>) -> RequestBuilder {
    let mut req = http.0.post(URL).header("Client-Id", auth.client_id);
    if let Some(token) = auth.token {
        req = req.header("Authorization", format!("OAuth {token}"));
        if auth.client_id == clients::ANDROID_TV {
            let ids = session_ids();
            req = req
                .header("Client-Session-Id", &ids.0)
                .header("X-Device-Id", &ids.1)
                .header("Client-Version", CLIENT_VERSION)
                .header("User-Agent", TV_USER_AGENT)
                .header("Origin", "https://android.tv.twitch.tv")
                .header("Referer", "https://android.tv.twitch.tv/");
        }
    }
    req
}

/// Session and device ids, new each run.
fn session_ids() -> &'static (String, String) {
    static IDS: OnceLock<(String, String)> = OnceLock::new();
    IDS.get_or_init(|| (hex(16), hex(16)))
}

fn unwrap<T>(res: Response<T>) -> Result<T> {
    match (res.data, res.errors.into_iter().next()) {
        (Some(data), None) => Ok(data),
        (_, Some(e)) => Err(Error::Api {
            status: 200,
            message: e.message,
        }),
        (None, None) => Err(Error::Api {
            status: 200,
            message: "empty answer".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_win_over_partial_data() {
        let res: Response<Value> =
            serde_json::from_str(r#"{"data":{"user":null},"errors":[{"message":"service timeout"}]}"#)
                .unwrap();
        assert_eq!(unwrap(res).unwrap_err().api_message(), Some("service timeout"));
        let res: Response<Value> = serde_json::from_str(r#"{"data":{"user":null}}"#).unwrap();
        assert!(unwrap(res).is_ok());
    }
}
