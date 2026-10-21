//! V107-R39 Video producer: your past broadcasts, highlights and uploads, private ones
//! too, and what Twitch's producer does to one: rename it, publish or unpublish it (its
//! `scope`, for every kind), delete it. Shapes as `VideoManagerQuery_ChannelVideosV2`,
//! `VideoManagerActions_UpdateVideo` and `VideoManagerActions_DeleteVideos`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::run;
use crate::gql::{Auth, VideoKind};
use crate::{Error, Http, Result};

pub const PAGE: u32 = 30;
/// Twitch's limit for a video title.
pub const TITLE_MAX: usize = 140;
/// Videos per delete request; the producer deletes one at a time or a selection.
pub const DELETE_MAX: usize = 25;

const LIST: &str = "query ContentVideos($id: ID!, $after: Cursor, $types: [BroadcastType!], $limit: Int) { \
    user(id: $id) { videos(first: $limit, after: $after, types: $types, sort: TIME, options: {includePrivate: true}) { \
    totalCount edges { cursor node { id title broadcastType lengthSeconds publishedAt createdAt viewCount scope status \
    previewThumbnailURL(width: 320, height: 180) game { displayName } } } } } }";
const UPDATE: &str = "mutation VideoManagerActions_UpdateVideo($input: UpdateVideoInput!) { \
    updateVideo(input: $input) { video { id title scope } } }";
const UPDATE_MANY: &str = "mutation VideoManagerActions_UpdateVideosScope($input: UpdateVideosViewabilityInput!) { \
    updateVideosViewability(input: $input) { error { code } } }";
const DELETE: &str = "mutation VideoManagerActions_DeleteVideos($input: DeleteVideosInput!) { \
    deleteVideos(input: $input) { videos { id } } }";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VideoState {
    Ready,
    /// The broadcast is still going.
    Recording,
    /// Uploaded or cut, Twitch is still working on it.
    Processing,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManagedVideo {
    pub id: String,
    pub title: String,
    pub kind: VideoKind,
    /// Seconds.
    pub length: u32,
    /// RFC 3339: when it went public, else when it was made.
    pub date: String,
    pub views: u32,
    /// Published: anyone can watch it. Unpublished ones only you see.
    pub public: bool,
    pub state: VideoState,
    pub thumbnail: Option<String>,
    pub game: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VideoPage {
    pub videos: Vec<ManagedVideo>,
    /// Set when there may be more.
    pub cursor: Option<String>,
    /// All of this kind, when Twitch says.
    pub total: Option<u32>,
}

/// One page of the channel's videos, newest first; every kind when `kind` is `None`.
pub async fn list(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    kind: Option<VideoKind>,
    cursor: Option<&str>,
) -> Result<VideoPage> {
    let types = kind.map(|k| json!([broadcast_type(k)])).unwrap_or(Value::Null);
    let vars = json!({ "id": channel_id, "after": cursor, "types": types, "limit": PAGE });
    read_list(run(http, auth, base, LIST, vars).await?)
}

fn broadcast_type(k: VideoKind) -> &'static str {
    match k {
        VideoKind::Archive => "ARCHIVE",
        VideoKind::Highlight => "HIGHLIGHT",
        VideoKind::Upload => "UPLOAD",
    }
}

pub(crate) fn read_list(data: Value) -> Result<VideoPage> {
    let raw: ListData = serde_json::from_value(data).map_err(Error::Decode)?;
    let Some(conn) = raw.user.and_then(|u| u.videos) else {
        return Ok(VideoPage { videos: Vec::new(), cursor: None, total: None });
    };
    // No `pageInfo` in Twitch's query: a full page means there may be more.
    let full = conn.edges.len() >= PAGE as usize;
    let cursor = if full { conn.edges.iter().rev().find_map(|e| e.cursor.clone()) } else { None };
    let videos = conn.edges.into_iter().filter_map(|e| e.node).map(ManagedVideo::from).collect();
    Ok(VideoPage { videos, cursor, total: conn.total_count })
}

/// A title as Twitch takes it, or why not.
pub fn clean_title(title: &str) -> std::result::Result<String, &'static str> {
    let t = title.trim();
    if t.is_empty() {
        Err("EMPTY")
    } else if t.chars().count() > TITLE_MAX {
        Err("TOO_LONG")
    } else {
        Ok(t.to_owned())
    }
}

/// Renames a video. Returns the title Twitch kept.
pub async fn rename(http: &Http, auth: Auth<'_>, base: Option<&str>, id: &str, title: &str) -> Result<String> {
    let title = clean_title(title).map_err(|c| Error::Refused(c.into()))?;
    let data = run(http, auth, base, UPDATE, json!({ "input": { "videoID": id, "title": title } })).await?;
    Ok(read_update(data)?.title.unwrap_or(title))
}

/// Publishes (`true`) or unpublishes a video. Returns whether it's public now.
pub async fn publish(http: &Http, auth: Auth<'_>, base: Option<&str>, id: &str, public: bool) -> Result<bool> {
    let scope = if public { "PUBLIC" } else { "PRIVATE" };
    let data = run(http, auth, base, UPDATE, json!({ "input": { "videoID": id, "scope": scope } })).await?;
    let now = read_update(data)?.scope.map_or(public, |s| s.eq_ignore_ascii_case("PUBLIC"));
    if now == public { Ok(now) } else { Err(Error::Refused("UNCHANGED".into())) }
}

#[derive(Deserialize)]
pub(crate) struct Updated {
    title: Option<String>,
    scope: Option<String>,
}

pub(crate) fn read_update(data: Value) -> Result<Updated> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        update_video: Option<Payload>,
    }
    #[derive(Deserialize)]
    struct Payload {
        video: Option<Updated>,
    }
    let d: Data = serde_json::from_value(data).map_err(Error::Decode)?;
    d.update_video.and_then(|p| p.video).ok_or_else(|| Error::Refused("UNKNOWN".into()))
}

/// Publishes (`true`) or unpublishes up to [`DELETE_MAX`] videos at once, as the producer's
/// bulk buttons do. Returns the ids sent when Twitch took them.
pub async fn publish_many(http: &Http, auth: Auth<'_>, base: Option<&str>, ids: &[String], public: bool) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > DELETE_MAX {
        return Err(Error::Refused("TOO_MANY".into()));
    }
    let scope = if public { "PUBLIC" } else { "PRIVATE" };
    let data = run(http, auth, base, UPDATE_MANY, json!({ "input": { "videoIDs": ids, "scope": scope } })).await?;
    read_many(data)?;
    Ok(ids.to_vec())
}

pub(crate) fn read_many(data: Value) -> Result<()> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        update_videos_viewability: Option<Payload>,
    }
    #[derive(Deserialize)]
    struct Payload {
        error: Option<super::super::Refusal>,
    }
    let d: Data = serde_json::from_value(data).map_err(Error::Decode)?;
    let Some(p) = d.update_videos_viewability else {
        return Err(Error::Refused("UNKNOWN".into()));
    };
    super::super::check(p.error)
}

/// Deletes up to [`DELETE_MAX`] videos. Returns the ids Twitch says it deleted.
pub async fn delete(http: &Http, auth: Auth<'_>, base: Option<&str>, ids: &[String]) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > DELETE_MAX {
        return Err(Error::Refused("TOO_MANY".into()));
    }
    let data = run(http, auth, base, DELETE, json!({ "input": { "videoIDs": ids } })).await?;
    read_delete(data)
}

pub(crate) fn read_delete(data: Value) -> Result<Vec<String>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        delete_videos: Option<Payload>,
    }
    #[derive(Deserialize)]
    struct Payload {
        videos: Option<Vec<Option<Id>>>,
    }
    #[derive(Deserialize)]
    struct Id {
        id: Option<String>,
    }
    let d: Data = serde_json::from_value(data).map_err(Error::Decode)?;
    let Some(p) = d.delete_videos else {
        return Err(Error::Refused("UNKNOWN".into()));
    };
    Ok(p.videos.unwrap_or_default().into_iter().flatten().filter_map(|v| v.id).collect())
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
#[serde(rename_all = "camelCase")]
struct Connection {
    total_count: Option<u32>,
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    cursor: Option<String>,
    node: Option<RawVideo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVideo {
    id: String,
    title: Option<String>,
    broadcast_type: Option<String>,
    length_seconds: Option<u32>,
    published_at: Option<String>,
    created_at: Option<String>,
    view_count: Option<u32>,
    scope: Option<String>,
    status: Option<String>,
    #[serde(rename = "previewThumbnailURL")]
    preview_thumbnail_url: Option<String>,
    game: Option<RawGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: Option<String>,
}

impl From<RawVideo> for ManagedVideo {
    fn from(v: RawVideo) -> Self {
        let status = v.status.unwrap_or_default().to_ascii_uppercase();
        let state = match status.as_str() {
            "RECORDING" => VideoState::Recording,
            "FAILED" => VideoState::Failed,
            "CREATED" | "UPLOADING" | "PENDING_TRANSCODE" | "TRANSCODING" | "UNPROCESSED" | "PROCESSING" => {
                VideoState::Processing
            }
            _ => VideoState::Ready,
        };
        let kind = match v.broadcast_type.as_deref().map(str::to_ascii_uppercase).as_deref() {
            Some("ARCHIVE") => VideoKind::Archive,
            Some("HIGHLIGHT") => VideoKind::Highlight,
            _ => VideoKind::Upload,
        };
        let thumbnail = v
            .preview_thumbnail_url
            .filter(|u| !u.is_empty() && !u.contains("/404_processing_") && state == VideoState::Ready);
        ManagedVideo {
            id: v.id,
            title: v.title.unwrap_or_default(),
            kind,
            length: v.length_seconds.unwrap_or(0),
            date: v.published_at.or(v.created_at).unwrap_or_default(),
            views: v.view_count.unwrap_or(0),
            public: v.scope.is_some_and(|s| s.eq_ignore_ascii_case("PUBLIC")),
            state,
            thumbnail,
            game: v.game.and_then(|g| g.display_name).filter(|n| !n.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::mock::{AUTH, serve};
    use super::*;

    fn http() -> Http {
        Http::new().unwrap()
    }

    #[test]
    fn reads_every_kind_and_state() {
        let data = json!({ "user": { "videos": { "totalCount": 3, "edges": [
            { "cursor": "a", "node": { "id": "1", "title": "Stream", "broadcastType": "ARCHIVE", "lengthSeconds": 3600,
              "publishedAt": "2026-10-20T10:00:00Z", "createdAt": "2026-10-20T09:00:00Z", "viewCount": 1234, "scope": "PUBLIC",
              "status": "RECORDED", "previewThumbnailURL": "https://x/a.jpg", "game": { "displayName": "Just Chatting" } } },
            { "cursor": "b", "node": { "id": "2", "title": null, "broadcastType": "highlight", "lengthSeconds": null,
              "publishedAt": null, "createdAt": "2026-10-19T09:00:00Z", "viewCount": null, "scope": "PRIVATE",
              "status": "PROCESSING", "previewThumbnailURL": "https://x/b.jpg", "game": null } },
            { "cursor": "c", "node": { "id": "3", "title": "Live", "broadcastType": "ARCHIVE", "lengthSeconds": 60,
              "publishedAt": null, "createdAt": null, "viewCount": 0, "scope": null, "status": "RECORDING",
              "previewThumbnailURL": "https://vod-secure.twitch.tv/_404/404_processing_320x180.png", "game": null } },
            { "cursor": "d", "node": null }
        ] } } });
        let p = read_list(data).unwrap();
        assert_eq!((p.total, p.cursor), (Some(3), None));
        let [a, b, c] = &p.videos[..] else { panic!("three videos") };
        assert_eq!((a.kind, a.state, a.public, a.views, a.length), (VideoKind::Archive, VideoState::Ready, true, 1234, 3600));
        assert_eq!(a.date, "2026-10-20T10:00:00Z");
        assert_eq!(a.game.as_deref(), Some("Just Chatting"));
        assert_eq!((b.kind, b.state, b.public, b.thumbnail.as_deref()), (VideoKind::Highlight, VideoState::Processing, false, None));
        assert_eq!(b.date, "2026-10-19T09:00:00Z");
        assert_eq!((c.state, c.public, c.thumbnail.as_deref(), c.date.as_str()), (VideoState::Recording, false, None, ""));
    }

    #[test]
    fn a_full_page_has_a_cursor() {
        let edges: Vec<Value> = (0..PAGE).map(|i| json!({ "cursor": format!("c{i}"), "node": { "id": i.to_string() } })).collect();
        let p = read_list(json!({ "user": { "videos": { "totalCount": null, "edges": edges } } })).unwrap();
        assert_eq!(p.cursor, Some(format!("c{}", PAGE - 1)));
        assert_eq!(p.videos[0].kind, VideoKind::Upload);
        assert_eq!(read_list(json!({ "user": null })).unwrap().videos, Vec::new());
    }

    #[test]
    fn titles_are_checked() {
        assert_eq!(clean_title("  New  "), Ok("New".into()));
        assert_eq!(clean_title("   "), Err("EMPTY"));
        assert_eq!(clean_title(&"é".repeat(TITLE_MAX)).map(|t| t.chars().count()), Ok(TITLE_MAX));
        assert_eq!(clean_title(&"x".repeat(TITLE_MAX + 1)), Err("TOO_LONG"));
    }

    #[tokio::test]
    async fn list_asks_for_private_ones_of_a_kind() {
        let (base, h) = serve(vec![r#"{"data":{"user":{"videos":{"totalCount":0,"edges":[]}}}}"#]);
        list(&http(), AUTH, Some(&base), "42", Some(VideoKind::Highlight), Some("cur")).await.unwrap();
        let sent = &h.join().unwrap()[0];
        assert!(sent["query"].as_str().unwrap().contains("options: {includePrivate: true}"));
        assert_eq!(sent["variables"], json!({ "id": "42", "after": "cur", "types": ["HIGHLIGHT"], "limit": PAGE }));
    }

    #[tokio::test]
    async fn rename_sends_the_trimmed_title() {
        let (base, h) = serve(vec![r#"{"data":{"updateVideo":{"video":{"id":"7","title":"Best run","scope":"PUBLIC"}}}}"#]);
        assert_eq!(rename(&http(), AUTH, Some(&base), "7", "  Best run ").await.unwrap(), "Best run");
        let sent = &h.join().unwrap()[0];
        assert!(sent["query"].as_str().unwrap().contains("updateVideo(input: $input)"));
        assert_eq!(sent["variables"], json!({ "input": { "videoID": "7", "title": "Best run" } }));
        // An empty title never leaves the app.
        assert!(matches!(rename(&http(), AUTH, Some("http://127.0.0.1:9"), "7", " ").await, Err(Error::Refused(c)) if c == "EMPTY"));
    }

    #[tokio::test]
    async fn publish_many_sends_ids_and_scope() {
        let (base, h) = serve(vec![
            r#"{"data":{"updateVideosViewability":{"error":null}}}"#,
            r#"{"data":{"updateVideosViewability":{"error":{"code":"FORBIDDEN"}}}}"#,
        ]);
        let ids = vec!["1".to_owned(), "2".to_owned()];
        assert_eq!(publish_many(&http(), AUTH, Some(&base), &ids, false).await.unwrap(), ids);
        assert!(matches!(publish_many(&http(), AUTH, Some(&base), &ids, true).await, Err(Error::Refused(c)) if c == "FORBIDDEN"));
        let sent = h.join().unwrap();
        assert!(sent[0]["query"].as_str().unwrap().contains("updateVideosViewability(input: $input)"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "videoIDs": ["1", "2"], "scope": "PRIVATE" } }));
        assert_eq!(sent[1]["variables"]["input"]["scope"], "PUBLIC");
        assert!(matches!(read_many(json!({ "updateVideosViewability": null })), Err(Error::Refused(_))));
        let many: Vec<String> = (0..=DELETE_MAX).map(|i| i.to_string()).collect();
        assert!(matches!(publish_many(&http(), AUTH, Some("http://127.0.0.1:9"), &many, true).await, Err(Error::Refused(_))));
    }

    #[tokio::test]
    async fn publish_sets_the_scope_and_checks_it() {
        let (base, h) = serve(vec![
            r#"{"data":{"updateVideo":{"video":{"id":"7","title":"x","scope":"PRIVATE"}}}}"#,
            r#"{"data":{"updateVideo":{"video":{"id":"7","title":"x","scope":"PRIVATE"}}}}"#,
            r#"{"errors":[{"message":"service error"}],"data":{"updateVideo":null}}"#,
        ]);
        assert!(!publish(&http(), AUTH, Some(&base), "7", false).await.unwrap());
        // Twitch kept it private: not reported as done.
        assert!(matches!(publish(&http(), AUTH, Some(&base), "7", true).await, Err(Error::Refused(c)) if c == "UNCHANGED"));
        assert_eq!(publish(&http(), AUTH, Some(&base), "7", true).await.unwrap_err().api_message(), Some("service error"));
        let sent = h.join().unwrap();
        assert_eq!(sent[0]["variables"], json!({ "input": { "videoID": "7", "scope": "PRIVATE" } }));
        assert_eq!(sent[1]["variables"], json!({ "input": { "videoID": "7", "scope": "PUBLIC" } }));
    }

    #[tokio::test]
    async fn delete_sends_ids_and_reads_what_went() {
        let (base, h) = serve(vec![r#"{"data":{"deleteVideos":{"videos":[{"id":"1"},null]}}}"#]);
        let ids = vec!["1".to_owned(), "2".to_owned()];
        assert_eq!(delete(&http(), AUTH, Some(&base), &ids).await.unwrap(), vec!["1".to_owned()]);
        let sent = &h.join().unwrap()[0];
        assert!(sent["query"].as_str().unwrap().starts_with("mutation VideoManagerActions_DeleteVideos"));
        assert_eq!(sent["variables"], json!({ "input": { "videoIDs": ["1", "2"] } }));
        let many: Vec<String> = (0..=DELETE_MAX).map(|i| i.to_string()).collect();
        assert!(matches!(delete(&http(), AUTH, Some("http://127.0.0.1:9"), &many).await, Err(Error::Refused(_))));
        assert!(matches!(read_delete(json!({ "deleteVideos": null })), Err(Error::Refused(_))));
    }
}
