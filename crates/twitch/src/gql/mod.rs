//! GQL (gql.twitch.tv), Twitch's private API. Can change without notice. Persisted query
//! hashes live next to the operation that uses them; raw queries are used where they work.

mod about;
pub mod blocks;
mod cards;
mod channels;
mod chat;
pub mod clip_create;
mod clips;
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
mod search;
mod stories;
pub mod subs;
mod users;
mod videos;
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
    match persisted::<Value>(http, auth, op.0, op.1, variables.clone()).await {
        Err(e) if e.api_message().is_some_and(|m| m.contains("PersistedQueryNotFound")) => {
            query(http, auth, op.2, variables).await
        }
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

/// Signed-in requests look like Twitch's TV app's (`clients::GQL`): the tools that
/// redeem rewards with its tokens send these headers and need no integrity token
/// (research, phase 3).
fn request(http: &Http, auth: Auth<'_>) -> RequestBuilder {
    let mut req = http.0.post(URL).header("Client-Id", auth.client_id);
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
