//! Clips (1.19, 1.20, 1.21): a channel's top clips, and one clip with what it takes to
//! play it. Signed out, a list stops at 100: paging needs an integrity token. Checked
//! 2026/10/09.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;
use url::Url;

use super::{Auth, query};
use crate::{Http, Result};

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

/// `None` if the clip was deleted.
pub async fn clip(http: &Http, slug: &str) -> Result<Option<ClipPlayback>> {
    let q = format!(
        "query($slug: ID!) {{ clip(slug: $slug) {{ ...C videoOffsetSeconds video {{ id }} \
         playbackAccessToken(params: {{platform: \"web\", playerBackend: \"mediaplayer\", playerType: \"site\"}}) \
         {{ signature value }} videoQualities {{ frameRate quality sourceURL }} }} }} {FIELDS}"
    );
    let data: ClipData = query(http, Auth::PUBLIC, &q, json!({ "slug": slug })).await?;
    Ok(data.clip.and_then(playback))
}

fn playback(raw: RawPlayableClip) -> Option<ClipPlayback> {
    let token = raw.playback_access_token?;
    let mut qualities: Vec<ClipQuality> = raw
        .video_qualities
        .into_iter()
        .filter_map(|q| {
            let mut url = Url::parse(&q.source_url).ok()?;
            url.query_pairs_mut()
                .append_pair("sig", &token.signature)
                .append_pair("token", &token.value);
            Some(ClipQuality {
                height: q.quality.parse().ok()?,
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
    #[serde(default)]
    login: String,
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: String,
}

impl From<RawClip> for Clip {
    fn from(c: RawClip) -> Self {
        let (channel_login, channel_name) = c.broadcaster.map(|b| (b.login, b.display_name)).unwrap_or_default();
        Clip {
            slug: c.slug,
            title: c.title.unwrap_or_default(),
            views: c.view_count.unwrap_or(0),
            created_at: c.created_at.unwrap_or_default(),
            duration: c.duration_seconds.unwrap_or(0),
            thumbnail: c.thumbnail_url.filter(|u| !u.is_empty()),
            channel_login,
            channel_name,
            clipped_by: c.curator.map(|u| u.display_name),
            game: c.game.map(|g| g.display_name),
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
    #[serde(default)]
    video_qualities: Vec<RawQuality>,
}

#[derive(Deserialize)]
struct RawVideo {
    id: String,
}

#[derive(Deserialize)]
struct RawToken {
    signature: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawQuality {
    frame_rate: Option<f32>,
    quality: String,
    #[serde(rename = "sourceURL")]
    source_url: String,
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
