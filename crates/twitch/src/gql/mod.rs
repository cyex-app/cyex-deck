//! GQL (gql.twitch.tv), Twitch's private API. Can change without notice. Persisted query
//! hashes live next to the operation that uses them; raw queries are used where they work.

mod about;
pub mod blocks;
mod cards;
mod channels;
mod chat;
pub mod clip_create;
mod clips;
pub mod connection;
mod comments;
pub mod dashboard;
mod directory;
pub mod drops;
mod emotes;
mod follows;
pub mod gifs;
mod identity;
pub mod miner;
pub mod moderation;
pub mod modlogs;
pub mod notifications;
mod playback;
mod points;
pub mod polls;
pub mod powerups;
pub mod predictions;
pub mod profile;
pub mod profile_images;
pub mod rewind;
mod search;
mod stories;
pub mod subs;
mod users;
mod videos;
pub mod viewer;
pub mod whispers;

pub use about::{About, LastStream, Panel, Social, about};
pub use cards::{Category, ChannelCard, GameRef, LiveStream, Page, cards};
pub use channels::{ChannelInfo, StreamInfo, channels_by_ids, lookup};
pub use clips::{Clip, ClipPeriod, ClipPlayback, ClipQuality, ClipSort, channel_clips, clip};
pub use comments::{Comment, comments};
pub use directory::{CategoryStreams, StreamFilter, StreamSort, category, top_categories, top_streams};
pub use emotes::{
    BitsEmote, ChannelEmotes, EmoteKind, EmoteOwner, EmoteSet, SubTier, TwitchEmote, TwitchEmoteInfo, available_emotes,
    channel_emotes, emote_info,
};
pub use identity::{
    BadgePick, ChatIdentity, chat_color, chat_identity, pick_channel_badge, pick_global_badge, set_chat_color,
};
pub use videos::{Storyboard, Video, VideoKind, VideoSort, channel_videos, storyboard, video};

pub use chat::{
    BadgeInfo, ChannelChat, CheerTier, Cheermote, GlobalChat, PinFragment, PinSender, PinnedMessage, channel_chat,
    global_chat, pinned,
};
pub use points::{
    AutoReward, CustomReward, EmoteChoice, EmoteChoices, EmoteModifier, Modification, Points, Redeemed, balance,
    emote_choices, redeem_custom, send_highlighted, send_in_sub_mode, unlock_chosen_emote, unlock_modified_emote,
    unlock_random_emote,
};
pub use follows::{
    LiveAlert, follow, followed_categories, followed_ids, followed_live, followed_live_alerts, is_following, unfollow,
};
pub use playback::{AccessToken, Player, stream_token, video_token};
pub use search::{SearchResults, search, search_channels};
pub use stories::{Story, StoryRing, mark_seen, stories, story_ring};
pub use users::{GqlUser, Profile, Suspension, ids_by_login, profile, user_by_id};

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
pub const TV_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 7.1; Smart Box C1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";

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
pub(crate) struct Response<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Deserialize)]
pub(crate) struct GqlError {
    message: String,
}

/// Runs a raw query.
pub(crate) async fn query<T: DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    query: &str,
    variables: Value,
) -> Result<T> {
    query_with(http, auth, &[], query, variables).await
}

/// Runs a raw query with extra headers.
pub(crate) async fn query_with<T: DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    headers: &[(&str, &str)],
    query: &str,
    variables: Value,
) -> Result<T> {
    let mut req = request(http, auth);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let req = req.json(&json!({ "query": query, "variables": variables }));
    unwrap(send_json(req).await?)
}

/// V106-21 Runs a raw query whose fields are all optional: see `partial`.
pub(crate) async fn query_partial<T: DeserializeOwned>(
    http: &Http,
    auth: Auth<'_>,
    query: &str,
    variables: Value,
    keep: impl FnOnce(&T) -> bool,
) -> Result<T> {
    let req = request(http, auth).json(&json!({ "query": query, "variables": variables }));
    partial(send_json(req).await?, keep)
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

/// A persisted operation, sent as raw text when Twitch no longer knows the hash.
/// `op` is (name, sha256, text).
pub(crate) async fn persisted_or_raw(http: &Http, auth: Auth<'_>, op: (&str, &str, &str), variables: Value) -> Result<Value> {
    let first = persisted::<Value>(http, auth, op.0, op.1, variables.clone()).await;
    or_raw(first, || query(http, auth, op.2, variables)).await
}

/// 1.43 Twitch answers an unknown hash with `PersistedQueryNotFound` (or
/// `PersistedQueryNotSupported`); only then the raw text is sent.
fn persisted_missing(e: &Error) -> bool {
    e.api_message()
        .is_some_and(|m| m.contains("PersistedQueryNotFound") || m.contains("PersistedQueryNotSupported"))
}

async fn or_raw<F>(first: Result<Value>, raw: impl FnOnce() -> F) -> Result<Value>
where
    F: std::future::Future<Output = Result<Value>>,
{
    match first {
        Err(e) if persisted_missing(&e) => raw().await,
        other => other,
    }
}

/// Twitch's GQL endpoint runs a JSON array of operations in one request (verified: 35 at
/// most, more fails the batch whole). Answers come back in order, errors kept per entry.
pub const BATCH_MAX: usize = 35;

/// Runs several raw operations in one request. `ops` is (query, variables) each; the
/// result is each operation's `data` or an `Err` carrying its first GQL error, in order.
pub(crate) async fn batch(
    http: &Http,
    auth: Auth<'_>,
    ops: &[(&str, Value)],
) -> Result<Vec<Result<Value>>> {
    if ops.is_empty() {
        return Ok(Vec::new());
    }
    let body: Vec<Value> = ops
        .iter()
        .map(|(q, v)| json!({ "query": q, "variables": v }))
        .collect();
    let req = request(http, auth).json(&body);
    let responses: Vec<Response<Value>> = send_json(req).await?;
    Ok(responses.into_iter().map(unwrap).collect())
}

/// V107-R41 GQL's address: Twitch's, or a local mock's `{base}/gql` (tests, mocks builds).
pub(crate) fn url_at(base: Option<&str>) -> String {
    base.map_or_else(|| URL.to_owned(), |b| format!("{b}/gql"))
}

/// V107-R41 One raw operation at `base`: its `data`, or its first GQL error.
pub(crate) async fn run_at(http: &Http, auth: Auth<'_>, base: Option<&str>, text: &str, variables: Value) -> Result<Value> {
    let req = request_at(http, &url_at(base), auth).json(&json!({ "query": text, "variables": variables }));
    unwrap(send_json::<Response<Value>>(req).await?)
}

/// V107-R41 The same for a read whose fields are all optional: see `partial`.
pub(crate) async fn run_partial_at(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    text: &str,
    variables: Value,
    keep: impl FnOnce(&Value) -> bool,
) -> Result<Value> {
    let req = request_at(http, &url_at(base), auth).json(&json!({ "query": text, "variables": variables }));
    partial(send_json::<Response<Value>>(req).await?, keep)
}

/// Signed-in requests look like Twitch's TV app's (`clients::GQL`): the tools that
/// redeem rewards with its tokens send these headers and need no integrity token
/// (research, phase 3).
fn request(http: &Http, auth: Auth<'_>) -> RequestBuilder {
    request_at(http, URL, auth)
}

/// V107-R19 The same request at another GQL address (a local mock in tests).
pub(crate) fn request_at(http: &Http, url: &str, auth: Auth<'_>) -> RequestBuilder {
    let mut req = http.0.post(url).header("Client-Id", auth.client_id);
    if let Some(token) = auth.token {
        req = req.header("Authorization", format!("OAuth {token}"));
        if auth.client_id == clients::GQL {
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

/// Data wins when there is some: a failing side field (Twitch nulls it and adds an
/// `errors` entry) must not blank a whole page (V106-21). Only for queries whose fields
/// are all optional. `keep` says whether the data is worth keeping (e.g. the user itself
/// came back); if not, the error wins as in `unwrap`.
pub(crate) fn partial<T>(res: Response<T>, keep: impl FnOnce(&T) -> bool) -> Result<T> {
    match res.data {
        Some(data) if res.errors.is_empty() || keep(&data) => Ok(data),
        data => unwrap(Response { data, errors: res.errors }),
    }
}

/// V101-24 `null` reads like a missing field: Twitch sends either.
pub(crate) fn nullable<'de, D: serde::Deserializer<'de>, T: Default + Deserialize<'de>>(
    d: D,
) -> std::result::Result<T, D::Error> {
    Option::<T>::deserialize(d).map(Option::unwrap_or_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_hash_falls_back_to_the_raw_query() {
        let missing: Response<Value> =
            serde_json::from_str(include_str!("../../tests/fixtures/gifs/persisted_not_found.json")).unwrap();
        let raw = || async { Ok(json!({ "sendGifMessage": { "error": null } })) };
        let v = or_raw(unwrap(missing), raw).await.unwrap();
        assert!(v["sendGifMessage"]["error"].is_null());
        // Any other error stands; the raw query isn't sent.
        let other = Err(Error::Api { status: 200, message: "service timeout".into() });
        let sent = std::cell::Cell::new(false);
        let r = or_raw(other, || async {
            sent.set(true);
            Ok(Value::Null)
        })
        .await;
        assert_eq!(r.unwrap_err().api_message(), Some("service timeout"));
        assert!(!sent.get());
        // A known hash answers as is.
        let ok = or_raw(Ok(json!(1)), || async { Ok(json!(2)) }).await.unwrap();
        assert_eq!(ok, json!(1));
    }

    #[test]
    fn partial_keeps_data_with_errors() {
        let raw = r#"{"data":{"user":{"id":"1"}},"errors":[{"message":"service timeout"}]}"#;
        let res: Response<Value> = serde_json::from_str(raw).unwrap();
        let data = partial(res, |d| !d["user"].is_null()).unwrap();
        assert_eq!(data["user"]["id"], "1");
    }

    #[test]
    fn partial_without_data_is_the_error() {
        let res: Response<Value> =
            serde_json::from_str(r#"{"data":null,"errors":[{"message":"service timeout"}]}"#).unwrap();
        assert_eq!(partial(res, |_| true).unwrap_err().api_message(), Some("service timeout"));
        // A missing user plus an error is the error, not "no such channel".
        let res: Response<Value> =
            serde_json::from_str(r#"{"data":{"user":null},"errors":[{"message":"service timeout"}]}"#).unwrap();
        assert_eq!(partial(res, |d| !d["user"].is_null()).unwrap_err().api_message(), Some("service timeout"));
        let res: Response<Value> = serde_json::from_str(r#"{"data":{"user":null}}"#).unwrap();
        assert!(partial(res, |d| !d["user"].is_null()).is_ok());
    }

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
