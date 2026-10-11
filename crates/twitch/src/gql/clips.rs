//! Clips (1.19, 1.20, 1.21): a channel's top clips, and one clip with what it takes to
//! play it. Signed out, a list stops at 100: paging needs an integrity token. Checked
//! 2026/10/09.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;
use url::Url;

use super::{Auth, query};
use crate::{Error, Http, Result};

const FIELDS: &str = "fragment C on Clip { slug title viewCount createdAt durationSeconds \
    thumbnailURL(width: 480, height: 272) broadcaster { login displayName } curator { displayName } \
    game { displayName } }";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClipPeriod {
    Day,
    Week,
    Month,
    All,
}

/// Newest first fails signed out ("server error").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClipSort {
    Views,
    Trending,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Clip {
    pub slug: String,
    pub title: String,
    pub views: u32,
    /// RFC 3339.
    pub created_at: String,
    /// Seconds.
    pub duration: u32,
    pub thumbnail: Option<String>,
    pub channel_login: String,
    pub channel_name: String,
    pub clipped_by: Option<String>,
    pub game: Option<String>,
}

/// A clip and its video files, best first. Plain MP4s that play in a `<video>`.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipPlayback {
    pub clip: Clip,
    pub qualities: Vec<ClipQuality>,
    /// The VOD it came from, if that's still up.
    pub vod_id: Option<String>,
    /// Where in that VOD, in seconds.
    pub vod_offset: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipQuality {
    /// Height, e.g. `1080`.
    pub height: u32,
    pub frame_rate: f32,
    /// Signed for 20 hours.
    pub url: String,
}

/// Up to 100 of a channel's clips. Empty if the channel doesn't exist.
pub async fn channel_clips(http: &Http, login: &str, period: ClipPeriod, sort: ClipSort) -> Result<Vec<Clip>> {
    let period = match period {
        ClipPeriod::Day => "LAST_DAY",
        ClipPeriod::Week => "LAST_WEEK",
        ClipPeriod::Month => "LAST_MONTH",
        ClipPeriod::All => "ALL_TIME",
    };
    let sort = match sort {
        ClipSort::Views => "VIEWS_DESC",
        ClipSort::Trending => "TRENDING",
    };
    let q = format!(
        "query($login: String!) {{ user(login: $login) {{ clips(first: 100, criteria: {{period: {period}, \
         sort: {sort}}}) {{ edges {{ node {{ ...C }} }} }} }} }} {FIELDS}"
    );
    let data: ListData = query(http, Auth::PUBLIC, &q, json!({ "login": login })).await?;
    let edges = data.user.and_then(|u| u.clips).map(|c| c.edges).unwrap_or_default();
    Ok(edges.into_iter().filter_map(|e| e.node).map(Clip::from).collect())
}

/// `None` if the clip was deleted or isn't ready yet.
pub async fn clip(http: &Http, slug: &str) -> Result<Option<ClipPlayback>> {
    let q = format!(
        "query($slug: ID!) {{ clip(slug: $slug) {{ ...C videoOffsetSeconds video {{ id }} \
         playbackAccessToken(params: {{platform: \"web\", playerBackend: \"mediaplayer\", playerType: \"site\"}}) \
         {{ signature value }} videoQualities {{ frameRate quality sourceURL }} \
         assets {{ type videoQualities {{ frameRate quality sourceURL }} }} }} }} {FIELDS}"
    );
    let raw: Value = query(http, Auth::PUBLIC, &q, json!({ "slug": slug })).await?;
    match serde_json::from_value::<ClipData>(raw.clone()) {
        Ok(data) => Ok(data.clip.and_then(playback)),
        Err(e) => {
            // V101-24 The answer, for the next shape Twitch comes up with.
            eprintln!("clips: unexpected answer for {slug}: {e}: {}", redacted(raw));
            Err(Error::Decode(e))
        }
    }
}

/// The answer without its access token, cut short.
fn redacted(mut raw: Value) -> String {
    if let Some(clip) = raw.get_mut("clip").and_then(Value::as_object_mut) {
        clip.remove("playbackAccessToken");
    }
    let mut text = raw.to_string();
    if let Some((cut, _)) = text.char_indices().nth(2000) {
        text.truncate(cut);
    }
    text
}

fn playback(raw: RawPlayableClip) -> Option<ClipPlayback> {
    let token = raw.playback_access_token?;
    let (signature, value) = (token.signature?, token.value?);
    // Newer clips keep their files under `assets`, the stream's own shape first.
    let files = if raw.video_qualities.is_empty() {
        let mut assets = raw.assets;
        assets.sort_by_key(|a| a.kind.as_deref() != Some("SOURCE"));
        assets.into_iter().map(|a| a.video_qualities).find(|q| !q.is_empty()).unwrap_or_default()
    } else {
        raw.video_qualities
    };
    let mut qualities: Vec<ClipQuality> = files
        .into_iter()
        .filter_map(|q| {
            let mut url = Url::parse(q.source_url.as_deref()?).ok()?;
            url.query_pairs_mut().append_pair("sig", &signature).append_pair("token", &value);
            Some(ClipQuality {
                height: q.quality?.parse().ok()?,
                frame_rate: q.frame_rate.unwrap_or(30.0),
                url: url.into(),
            })
        })
        .collect();
    qualities.sort_by(|a, b| b.height.cmp(&a.height).then(b.frame_rate.total_cmp(&a.frame_rate)));
    if qualities.is_empty() {
        return None;
    }
    Some(ClipPlayback {
        vod_id: raw.video.map(|v| v.id),
        vod_offset: raw.video_offset_seconds,
        clip: raw.clip.into(),
        qualities,
    })
}

#[derive(Deserialize)]
struct ListData {
    user: Option<ListUser>,
}

#[derive(Deserialize)]
struct ListUser {
    clips: Option<Connection>,
}

#[derive(Deserialize)]
struct Connection {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Option<RawClip>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawClip {
    slug: String,
    title: Option<String>,
    view_count: Option<u32>,
    created_at: Option<String>,
    duration_seconds: Option<u32>,
    #[serde(rename = "thumbnailURL")]
    thumbnail_url: Option<String>,
    broadcaster: Option<RawUser>,
    curator: Option<RawUser>,
    game: Option<RawGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    #[serde(default, deserialize_with = "nullable")]
    login: String,
    #[serde(default, deserialize_with = "nullable")]
    display_name: String,
}

/// V101-24 `null` reads like a missing field: Twitch sends either.
fn nullable<'de, D: Deserializer<'de>, T: Default + Deserialize<'de>>(d: D) -> std::result::Result<T, D::Error> {
    Option::<T>::deserialize(d).map(Option::unwrap_or_default)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    #[serde(default, deserialize_with = "nullable")]
    display_name: String,
}

impl From<RawClip> for Clip {
    fn from(c: RawClip) -> Self {
        let (channel_login, channel_name) = c.broadcaster.map(|b| (b.login, b.display_name)).unwrap_or_default();
        let channel_name = if channel_name.is_empty() { channel_login.clone() } else { channel_name };
        Clip {
            slug: c.slug,
            title: c.title.unwrap_or_default(),
            views: c.view_count.unwrap_or(0),
            created_at: c.created_at.unwrap_or_default(),
            duration: c.duration_seconds.unwrap_or(0),
            thumbnail: c.thumbnail_url.filter(|u| !u.is_empty()),
            channel_login,
            channel_name,
            clipped_by: c.curator.map(|u| u.display_name).filter(|n| !n.is_empty()),
            game: c.game.map(|g| g.display_name).filter(|n| !n.is_empty()),
        }
    }
}

#[derive(Deserialize)]
struct ClipData {
    clip: Option<RawPlayableClip>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPlayableClip {
    #[serde(flatten)]
    clip: RawClip,
    video_offset_seconds: Option<u32>,
    video: Option<RawVideo>,
    playback_access_token: Option<RawToken>,
    #[serde(default, deserialize_with = "nullable")]
    video_qualities: Vec<RawQuality>,
    #[serde(default, deserialize_with = "nullable")]
    assets: Vec<RawAsset>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAsset {
    /// `SOURCE` is the stream's own shape, `RECOMPOSED` a portrait cut.
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    video_qualities: Vec<RawQuality>,
}

#[derive(Deserialize)]
struct RawVideo {
    id: String,
}

#[derive(Deserialize)]
struct RawToken {
    signature: Option<String>,
    value: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawQuality {
    frame_rate: Option<f32>,
    quality: Option<String>,
    #[serde(rename = "sourceURL")]
    source_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playable_clip_signs_its_urls_best_first() {
        let d: ClipData = serde_json::from_str(
            r#"{"clip":{"slug":"Funny-AbC","title":"lol","viewCount":12,"createdAt":"2026-10-09T10:00:00Z",
              "durationSeconds":30,"thumbnailURL":"https://t/x.jpg","broadcaster":{"login":"xqc","displayName":"xQc"},
              "curator":{"login":"a","displayName":"A"},"game":null,"videoOffsetSeconds":120,"video":{"id":"9"},
              "playbackAccessToken":{"signature":"abc","value":"{\"clip_uri\":\"x\"}"},
              "videoQualities":[{"frameRate":30,"quality":"480","sourceURL":"https://c.example/480/index.mp4"},
                                {"frameRate":60.004,"quality":"1080","sourceURL":"https://c.example/1080/index.mp4"}]}}"#,
        )
        .unwrap();
        let p = playback(d.clip.unwrap()).unwrap();
        assert_eq!(p.clip.channel_name, "xQc");
        assert_eq!(p.clip.clipped_by.as_deref(), Some("A"));
        assert_eq!((p.vod_id.as_deref(), p.vod_offset), (Some("9"), Some(120)));
        assert_eq!(p.qualities[0].height, 1080);
        assert_eq!(
            p.qualities[0].url,
            "https://c.example/1080/index.mp4?sig=abc&token=%7B%22clip_uri%22%3A%22x%22%7D"
        );
    }

    #[test]
    fn nulls_read_as_missing_and_assets_stand_in() {
        // V101-24 The shapes that used to end in "unexpected answer from Twitch".
        let d: ClipData = serde_json::from_str(
            r#"{"clip":{"slug":"Fresh-AbC","title":null,"viewCount":0,"createdAt":"2026-10-11T08:00:00Z",
              "durationSeconds":30,"thumbnailURL":null,"broadcaster":{"login":"kuni","displayName":null},
              "curator":{"login":null,"displayName":null},"game":{"displayName":null},"videoOffsetSeconds":null,
              "video":null,"playbackAccessToken":{"signature":"s","value":"v"},"videoQualities":null,
              "assets":[{"type":"RECOMPOSED","videoQualities":[{"frameRate":30,"quality":"1080","sourceURL":"https://c/p/1080.mp4"}]},
                        {"type":"SOURCE","videoQualities":[{"frameRate":60,"quality":"720","sourceURL":"https://c/l/720.mp4"},
                                                           {"frameRate":null,"quality":null,"sourceURL":null}]}]}}"#,
        )
        .unwrap();
        let p = playback(d.clip.unwrap()).unwrap();
        assert_eq!(p.clip.channel_name, "kuni");
        assert_eq!((p.clip.clipped_by, p.clip.game), (None, None));
        assert_eq!(p.qualities.len(), 1);
        assert_eq!(p.qualities[0].url, "https://c/l/720.mp4?sig=s&token=v");
        let empty: ClipData = serde_json::from_str(
            r#"{"clip":{"slug":"x","playbackAccessToken":{"signature":null,"value":null},"videoQualities":[],"assets":null}}"#,
        )
        .unwrap();
        assert!(playback(empty.clip.unwrap()).is_none());
    }

    #[test]
    fn redacted_answers_keep_no_token() {
        let raw = json!({ "clip": { "slug": "a", "playbackAccessToken": { "value": "secret" } } });
        let text = redacted(raw);
        assert!(text.contains("\"slug\":\"a\"") && !text.contains("secret"));
    }

    #[test]
    fn deleted_clips_and_lists() {
        let d: ClipData = serde_json::from_str(r#"{"clip":null}"#).unwrap();
        assert!(d.clip.is_none());
        let l: ListData = serde_json::from_str(
            r#"{"user":{"clips":{"edges":[{"node":{"slug":"a","title":null,"viewCount":null,"createdAt":null,
              "durationSeconds":null,"thumbnailURL":"","broadcaster":null,"curator":null,"game":{"displayName":"Chess"}}}]}}}"#,
        )
        .unwrap();
        let clip = Clip::from(l.user.unwrap().clips.unwrap().edges.remove(0).node.unwrap());
        assert_eq!((clip.thumbnail, clip.game.as_deref()), (None, Some("Chess")));
    }
}
