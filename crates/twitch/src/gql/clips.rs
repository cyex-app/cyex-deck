//! Clips (1.19, 1.20, 1.21): a channel's top clips, and one clip with what it takes to
//! play it. Signed out, a list stops at 100: paging needs an integrity token. Checked
//! 2026/10/09.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;
use url::Url;

use super::{Auth, nullable, query};
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
    /// V109-C05 Only the clips manager asks (Twitch’s `isViewerEditRestricted`): true, the
    /// one who clipped it may no longer edit it; the channel owner still may.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub edit_restricted: Option<bool>,
}

/// A clip and its video files, best first. Plain MP4s that play in a `<video>`.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipPlayback {
    pub clip: Clip,
    pub qualities: Vec<ClipQuality>,
    /// P11 Twitch's portrait (9:16) cut, best first; empty for older clips. `height` is
    /// the short side here too (972 is 972 × 1728), so labels read like landscape ones.
    pub portrait: Vec<ClipQuality>,
    /// The VOD it came from, if that's still up.
    pub vod_id: Option<String>,
    /// Where in that VOD, in seconds.
    pub vod_offset: Option<u32>,
}

impl ClipPlayback {
    /// V108-N02 The MP4 to save: the landscape file of `height` (or the best one when that
    /// height isn't there) and a file name for it, `<slug>-<height>p.mp4`. The URL is the
    /// signed one the player plays. `None` for a clip without files.
    pub fn download(&self, height: Option<u32>) -> Option<(String, String)> {
        let q = height
            .and_then(|h| self.qualities.iter().find(|q| q.height == h))
            .or_else(|| self.qualities.first())?;
        Some((q.url.clone(), format!("{}-{}p.mp4", file_stem(&self.clip.slug), q.height)))
    }
}

/// A slug is safe in a file name already; this guards the odd one.
fn file_stem(slug: &str) -> String {
    let s: String = slug.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if s.is_empty() { "clip".into() } else { s }
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
    Ok(edges_of(data)?.into_iter().filter_map(|e| e.node).map(Clip::from).collect())
}

/// V110-A36 No such channel reads as no clips; a channel without its clip list (or one
/// without `edges`) is an answer of the wrong shape, an error with Retry.
fn edges_of(data: ListData) -> Result<Vec<Edge>> {
    match data.user {
        None => Ok(Vec::new()),
        Some(u) => u.clips.and_then(|c| c.edges).ok_or_else(|| Error::shape("clips")),
    }
}

/// `None` if the clip was deleted or isn't ready yet: a clip just made stays `CREATING`
/// for a few seconds, and its files play black until then (V102-08). [`Error::Refused`]
/// with `CLIP_FAILED` when Twitch couldn't make it.
pub async fn clip(http: &Http, slug: &str) -> Result<Option<ClipPlayback>> {
    Ok(match clip_state(http, slug).await? {
        ClipState::Ready(p) => Some(*p),
        ClipState::Making | ClipState::Gone => None,
    })
}

/// V109-F73 Where one clip is. A clip still being made and a deleted one both read as `None`
/// from [`clip`], so a slow clip was called gone after 30 s.
#[derive(Debug)]
pub enum ClipState {
    Ready(Box<ClipPlayback>),
    /// `CREATING`, or made with no file listed yet.
    Making,
    /// No such clip (deleted, or never made).
    Gone,
}

/// Like [`clip`], telling a clip still being made from one that's gone. [`Error::Refused`]
/// with `CLIP_FAILED` when Twitch couldn't make it.
pub async fn clip_state(http: &Http, slug: &str) -> Result<ClipState> {
    let q = format!(
        "query($slug: ID!) {{ clip(slug: $slug) {{ ...C creationState videoOffsetSeconds video {{ id }} \
         playbackAccessToken(params: {{platform: \"web\", playerBackend: \"mediaplayer\", playerType: \"site\"}}) \
         {{ signature value }} videoQualities {{ frameRate quality sourceURL }} \
         assets {{ type videoQualities {{ frameRate quality sourceURL }} }} }} }} {FIELDS}"
    );
    let raw: Value = query(http, Auth::PUBLIC, &q, json!({ "slug": slug })).await?;
    state_of(raw, slug)
}

fn state_of(raw: Value, slug: &str) -> Result<ClipState> {
    if !made(&raw)? {
        return Ok(ClipState::Making);
    }
    match serde_json::from_value::<ClipData>(raw.clone()) {
        Ok(ClipData { clip: None }) => Ok(ClipState::Gone),
        Ok(ClipData { clip: Some(c) }) => Ok(playback(c).map_or(ClipState::Making, |p| ClipState::Ready(Box::new(p)))),
        Err(e) => {
            // V101-24 The answer, for the next shape Twitch comes up with.
            eprintln!("clips: unexpected answer for {slug}: {e}: {}", redacted(raw));
            Err(Error::Decode(e))
        }
    }
}

/// V108-C25 One clip's details without its files, for a link preview. `None` if it's gone.
pub async fn clip_info(http: &Http, slug: &str) -> Result<Option<Clip>> {
    let q = format!("query($slug: ID!) {{ clip(slug: $slug) {{ ...C }} }} {FIELDS}");
    let data: InfoData = query(http, Auth::PUBLIC, &q, json!({ "slug": slug })).await?;
    Ok(data.clip.map(Clip::from))
}

#[derive(Deserialize)]
struct InfoData {
    clip: Option<RawClip>,
}

/// V102-08 Whether Twitch has made the clip. Read before the rest: a clip still being made
/// lacks what Twitch fills in later, so its answer isn't read at all until then.
fn made(raw: &Value) -> Result<bool> {
    match raw["clip"]["creationState"].as_str() {
        Some("CREATING") => Ok(false),
        Some("FAILED") => Err(Error::Refused("CLIP_FAILED".into())),
        _ => Ok(true),
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
    let portrait_of = |a: &RawAsset| a.kind.as_deref() == Some("RECOMPOSED");
    // P11 The same token signs the portrait cut.
    let portrait = raw
        .assets
        .iter()
        .find(|a| portrait_of(a))
        .map(|a| sign(&a.video_qualities, &signature, &value))
        .unwrap_or_default();
    // Newer clips keep their files under `assets`, the stream's own shape first.
    let qualities = if raw.video_qualities.is_empty() {
        let mut assets: Vec<&RawAsset> = raw.assets.iter().collect();
        assets.sort_by_key(|a| (portrait_of(a), a.kind.as_deref() != Some("SOURCE")));
        assets
            .into_iter()
            .map(|a| sign(&a.video_qualities, &signature, &value))
            .find(|q| !q.is_empty())
            .unwrap_or_default()
    } else {
        sign(&raw.video_qualities, &signature, &value)
    };
    if qualities.is_empty() {
        return None;
    }
    // Only a portrait cut: it's the clip, not an extra.
    let portrait = if qualities == portrait {
        Vec::new()
    } else {
        portrait
    };
    Some(ClipPlayback {
        vod_id: raw.video.map(|v| v.id),
        vod_offset: raw.video_offset_seconds,
        clip: raw.clip.into(),
        qualities,
        portrait,
    })
}

/// Signed file URLs, best first. Portrait files name their short side, so `height` is
/// the quality's number either way.
fn sign(files: &[RawQuality], signature: &str, value: &str) -> Vec<ClipQuality> {
    let mut out: Vec<ClipQuality> = files
        .iter()
        .filter_map(|q| {
            let mut url = Url::parse(q.source_url.as_deref()?).ok()?;
            url.query_pairs_mut().append_pair("sig", signature).append_pair("token", value);
            Some(ClipQuality {
                height: q.quality.as_deref()?.parse().ok()?,
                frame_rate: q.frame_rate.unwrap_or(30.0),
                url: url.into(),
            })
        })
        .collect();
    out.sort_by(|a, b| b.height.cmp(&a.height).then(b.frame_rate.total_cmp(&a.frame_rate)));
    out
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
    edges: Option<Vec<Edge>>,
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
            edit_restricted: None,
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
    #[test]
    fn v108_n02_download_picks_a_height_or_the_best() {
        let d: ClipData = serde_json::from_str(
            r#"{"clip":{"slug":"Funny-AbC","playbackAccessToken":{"signature":"s","value":"v"},
              "videoQualities":[{"frameRate":30,"quality":"480","sourceURL":"https://c.example/480/index.mp4"},
                                {"frameRate":60,"quality":"1080","sourceURL":"https://c.example/1080/index.mp4"}]}}"#,
        )
        .unwrap();
        let p = playback(d.clip.unwrap()).unwrap();
        let (url, name) = p.download(None).unwrap();
        assert!(url.starts_with("https://c.example/1080/") && url.contains("sig=s"));
        assert_eq!(name, "Funny-AbC-1080p.mp4");
        assert_eq!(p.download(Some(480)).unwrap().1, "Funny-AbC-480p.mp4");
        assert_eq!(p.download(Some(720)).unwrap().1, "Funny-AbC-1080p.mp4");
        assert_eq!(file_stem("a/b c"), "a_b_c");
        assert_eq!(file_stem(""), "clip");
    }

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
        // P11 The portrait cut comes along, signed the same way.
        assert_eq!(p.portrait.len(), 1);
        assert_eq!((p.portrait[0].height, p.portrait[0].url.as_str()), (1080, "https://c/p/1080.mp4?sig=s&token=v"));
        let empty: ClipData = serde_json::from_str(
            r#"{"clip":{"slug":"x","playbackAccessToken":{"signature":null,"value":null},"videoQualities":[],"assets":null}}"#,
        )
        .unwrap();
        assert!(playback(empty.clip.unwrap()).is_none());
    }

    #[test]
    fn a_clip_still_being_made_waits() {
        // V102-08 Still being made: its answer isn't read, whatever shape it has.
        let making = json!({ "clip": { "slug": "Fresh-AbC", "creationState": "CREATING", "durationSeconds": null,
            "videoQualities": "not even a list" } });
        assert!(!made(&making).unwrap());
        let failed = json!({ "clip": { "creationState": "FAILED" } });
        assert!(matches!(made(&failed), Err(Error::Refused(c)) if c == "CLIP_FAILED"));
        // Made, deleted, or an older clip without the field: read as usual.
        assert!(made(&json!({ "clip": { "creationState": "CREATED" } })).unwrap());
        assert!(made(&json!({ "clip": null })).unwrap());
        assert!(made(&json!({ "clip": { "slug": "Old" } })).unwrap());
    }

    #[test]
    fn a_slow_clip_is_making_not_gone() {
        // V109-F73 The overlay called a clip still being made "gone" after 30 s.
        let making = json!({ "clip": { "slug": "Fresh-AbC", "creationState": "CREATING" } });
        assert!(matches!(state_of(making, "s").unwrap(), ClipState::Making));
        // Made, but no file listed yet: still making.
        let bare = json!({ "clip": { "slug": "x", "creationState": "CREATED",
            "playbackAccessToken": { "signature": "s", "value": "v" }, "videoQualities": [], "assets": null } });
        assert!(matches!(state_of(bare, "s").unwrap(), ClipState::Making));
        assert!(matches!(state_of(json!({ "clip": null }), "s").unwrap(), ClipState::Gone));
        let failed = json!({ "clip": { "creationState": "FAILED" } });
        assert!(matches!(state_of(failed, "s"), Err(Error::Refused(c)) if c == "CLIP_FAILED"));
        let ready = json!({ "clip": { "slug": "x", "creationState": "CREATED",
            "playbackAccessToken": { "signature": "s", "value": "v" },
            "videoQualities": [{ "frameRate": 30, "quality": "720", "sourceURL": "https://c/720.mp4" }], "assets": null } });
        assert!(matches!(state_of(ready, "s").unwrap(), ClipState::Ready(_)));
    }

    #[test]
    fn a_clip_with_only_a_portrait_cut_plays_it() {
        let d: ClipData = serde_json::from_str(
            r#"{"clip":{"slug":"P-AbC","playbackAccessToken":{"signature":"s","value":"v"},"videoQualities":[],
              "assets":[{"type":"RECOMPOSED","videoQualities":[{"frameRate":30,"quality":"720","sourceURL":"https://c/p/720.mp4"}]}]}}"#,
        )
        .unwrap();
        let p = playback(d.clip.unwrap()).unwrap();
        assert_eq!(p.qualities.len(), 1);
        assert!(p.portrait.is_empty());
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
        let clip = Clip::from(l.user.unwrap().clips.unwrap().edges.unwrap().remove(0).node.unwrap());
        assert_eq!((clip.thumbnail, clip.game.as_deref()), (None, Some("Chess")));
    }

    #[test]
    fn v110_a36_a_clip_list_of_the_wrong_shape_is_an_error() {
        let list = |s: &str| edges_of(serde_json::from_str::<ListData>(s).unwrap());
        assert!(list(r#"{"user":null}"#).unwrap().is_empty(), "no such channel");
        assert!(list(r#"{"user":{"clips":{"edges":[]}}}"#).unwrap().is_empty(), "a channel without clips");
        for bad in [r#"{"user":{}}"#, r#"{"user":{"clips":null}}"#, r#"{"user":{"clips":{}}}"#] {
            assert!(matches!(list(bad), Err(Error::Decode(_))), "{bad}");
        }
    }
}
