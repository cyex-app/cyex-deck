//! A channel's videos (1.21): past broadcasts, highlights and uploads. Signed out, a
//! list stops at 100: paging needs an integrity token. Checked 2026/10/09.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

const FIELDS: &str = "fragment V on Video { id title lengthSeconds publishedAt viewCount broadcastType status \
    previewThumbnailURL(width: 320, height: 180) owner { login displayName } game { displayName } \
    restriction { type } }";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VideoKind {
    /// Past broadcast.
    Archive,
    Highlight,
    Upload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VideoSort {
    Newest,
    Views,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Video {
    pub id: String,
    pub title: String,
    /// Seconds.
    pub length: u32,
    /// RFC 3339.
    pub published_at: String,
    pub views: u32,
    pub kind: VideoKind,
    /// The broadcast is still going.
    pub recording: bool,
    /// Subscribers only.
    pub restricted: bool,
    pub thumbnail: Option<String>,
    pub channel_login: String,
    pub channel_name: String,
    pub game: Option<String>,
}

/// Up to 100 of a channel's videos, all kinds when `kind` is `None`.
pub async fn channel_videos(
    http: &Http,
    login: &str,
    kind: Option<VideoKind>,
    sort: VideoSort,
) -> Result<Vec<Video>> {
    let kind = match kind {
        Some(VideoKind::Archive) => ", type: ARCHIVE",
        Some(VideoKind::Highlight) => ", type: HIGHLIGHT",
        Some(VideoKind::Upload) => ", type: UPLOAD",
        None => "",
    };
    let sort = match sort {
        VideoSort::Newest => "TIME",
        VideoSort::Views => "VIEWS",
    };
    let q = format!(
        "query($login: String!) {{ user(login: $login) {{ videos(first: 100{kind}, sort: {sort}) \
         {{ edges {{ node {{ ...V }} }} }} }} }} {FIELDS}"
    );
    let data: ListData = query(http, Auth::PUBLIC, &q, json!({ "login": login })).await?;
    let edges = data.user.and_then(|u| u.videos).map(|v| v.edges).unwrap_or_default();
    Ok(edges.into_iter().filter_map(|e| e.node).map(Video::from).collect())
}

/// One video, for the player's header. `None` if it was deleted.
pub async fn video(http: &Http, id: &str) -> Result<Option<Video>> {
    let q = format!("query($id: ID!) {{ video(id: $id) {{ ...V }} }} {FIELDS}");
    let data: VideoData = query(http, Auth::PUBLIC, &q, json!({ "id": id })).await?;
    Ok(data.video.map(Video::from))
}

/// P6 Thumbnails along a VOD's timeline: sprite sheets of `cols` × `rows` pictures,
/// one every `interval` seconds.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Storyboard {
    pub images: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub cols: u32,
    pub rows: u32,
    pub count: u32,
    pub interval: f32,
}

#[derive(Deserialize)]
struct RawBoard {
    count: u32,
    width: u32,
    height: u32,
    rows: u32,
    cols: u32,
    #[serde(default)]
    images: Vec<String>,
    interval: f32,
    quality: Option<String>,
}

/// `None` when the VOD has none. The JSON sits on a server the webview can't read.
pub async fn storyboard(http: &Http, id: &str) -> Result<Option<Storyboard>> {
    #[derive(Deserialize)]
    struct Data {
        video: Option<RawSeek>,
    }
    #[derive(Deserialize)]
    struct RawSeek {
        #[serde(rename = "seekPreviewsURL")]
        url: Option<String>,
    }
    let data: Data = query(http, Auth::PUBLIC, "query($id: ID!) { video(id: $id) { seekPreviewsURL } }", json!({ "id": id })).await?;
    let Some(url) = data.video.and_then(|v| v.url).filter(|u| u.starts_with("https://")) else { return Ok(None) };
    let boards: Vec<RawBoard> = crate::http::json(http.0.get(&url)).await?;
    Ok(pick_board(boards, &url))
}

/// The sharper set when there is one. Image names are relative to the JSON.
fn pick_board(boards: Vec<RawBoard>, url: &str) -> Option<Storyboard> {
    let base = &url[..url.rfind('/')? + 1];
    let mut boards = boards;
    boards.sort_by_key(|b| b.quality.as_deref() != Some("high"));
    let b = boards.into_iter().find(|b| !b.images.is_empty() && b.cols > 0 && b.rows > 0)?;
    Some(Storyboard {
        images: b.images.iter().map(|i| format!("{base}{i}")).collect(),
        width: b.width,
        height: b.height,
        cols: b.cols,
        rows: b.rows,
        count: b.count,
        interval: b.interval,
    })
}

#[derive(Deserialize)]
struct ListData {
    user: Option<ListUser>,
}

#[derive(Deserialize)]
struct ListUser {
    videos: Option<Connection>,
}

#[derive(Deserialize)]
struct Connection {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Option<RawVideo>,
}

#[derive(Deserialize)]
struct VideoData {
    video: Option<RawVideo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVideo {
    id: String,
    title: Option<String>,
    length_seconds: Option<u32>,
    published_at: Option<String>,
    view_count: Option<u32>,
    broadcast_type: Option<String>,
    status: Option<String>,
    #[serde(rename = "previewThumbnailURL")]
    preview_thumbnail_url: Option<String>,
    owner: Option<RawUser>,
    game: Option<RawGame>,
    restriction: Option<RawRestriction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    login: String,
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: String,
}

#[derive(Deserialize)]
struct RawRestriction {
    #[serde(rename = "type")]
    kind: Option<String>,
}

impl From<RawVideo> for Video {
    fn from(v: RawVideo) -> Self {
        let recording = v.status.as_deref() == Some("RECORDING");
        let (channel_login, channel_name) = v.owner.map(|o| (o.login, o.display_name)).unwrap_or_default();
        Video {
            id: v.id,
            title: v.title.unwrap_or_default(),
            length: v.length_seconds.unwrap_or(0),
            published_at: v.published_at.unwrap_or_default(),
            views: v.view_count.unwrap_or(0),
            kind: match v.broadcast_type.as_deref() {
                Some("ARCHIVE") => VideoKind::Archive,
                Some("HIGHLIGHT") => VideoKind::Highlight,
                _ => VideoKind::Upload,
            },
            recording,
            restricted: v.restriction.and_then(|r| r.kind).is_some_and(|k| !k.is_empty()),
            // A recording broadcast shows Twitch's "processing" picture.
            thumbnail: v
                .preview_thumbnail_url
                .filter(|u| !u.is_empty() && !u.contains("/_404/")),
            channel_login,
            channel_name,
            game: v.game.map(|g| g.display_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storyboards_prefer_the_sharp_set() {
        let boards: Vec<RawBoard> = serde_json::from_str(
            r#"[{"count":200,"width":160,"rows":40,"images":["9-low-0.jpg"],"interval":133,"quality":"low","cols":5,"height":90},
                {"count":200,"width":220,"rows":10,"images":["9-high-0.jpg","9-high-1.jpg"],"interval":133,"quality":"high","cols":5,"height":124}]"#,
        )
        .unwrap();
        let b = pick_board(boards, "https://cdn.example/x_y/storyboards/9-info.json").unwrap();
        assert_eq!(b.images, ["https://cdn.example/x_y/storyboards/9-high-0.jpg", "https://cdn.example/x_y/storyboards/9-high-1.jpg"]);
        assert_eq!((b.width, b.height, b.cols, b.rows, b.interval), (220, 124, 5, 10, 133.0));
    }

    #[test]
    fn videos_parse_with_gaps() {
        let d: ListData = serde_json::from_str(
            r#"{"user":{"videos":{"edges":[
              {"node":{"id":"1","title":"Live now","lengthSeconds":3600,"publishedAt":"2026-10-09T10:00:00Z",
               "viewCount":5,"broadcastType":"ARCHIVE","status":"RECORDING",
               "previewThumbnailURL":"https://vod-secure.twitch.tv/_404/404_processing_320x180.png",
               "owner":{"login":"xqc","displayName":"xQc"},"game":null,"restriction":null}},
              {"node":{"id":"2","title":null,"lengthSeconds":null,"publishedAt":null,"viewCount":null,
               "broadcastType":"HIGHLIGHT","status":"RECORDED","previewThumbnailURL":"https://t/2.jpg",
               "owner":null,"game":{"displayName":"Chess"},"restriction":{"type":"SUB_ONLY_LIVE"}}},
              {"node":null}]}}}"#,
        )
        .unwrap();
        let list: Vec<Video> = d
            .user
            .unwrap()
            .videos
            .unwrap()
            .edges
            .into_iter()
            .filter_map(|e| e.node)
            .map(Video::from)
            .collect();
        assert_eq!(list.len(), 2);
        assert!(list[0].recording && list[0].thumbnail.is_none());
        assert_eq!((list[0].kind, list[0].channel_name.as_str()), (VideoKind::Archive, "xQc"));
        assert!(list[1].restricted && !list[0].restricted);
        assert_eq!(list[1].kind, VideoKind::Highlight);
    }
}
