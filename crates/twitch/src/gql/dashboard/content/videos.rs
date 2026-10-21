//! V107-R39 Video producer: your past broadcasts, highlights and uploads, private ones
//! too, and what Twitch's producer does to one: rename it, publish or unpublish it (its
//! `scope`, for every kind), delete it. Shapes as `VideoManagerQuery_ChannelVideosV2`,
//! `VideoManagerActions_UpdateVideo` and `VideoManagerActions_DeleteVideos`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::gql::VideoKind;
use crate::gql::dashboard::Channel;
use crate::{Error, Result};

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
pub async fn list(ch: &Channel<'_>, kind: Option<VideoKind>, cursor: Option<&str>) -> Result<VideoPage> {
    let types = kind.map(|k| json!([broadcast_type(k)])).unwrap_or(Value::Null);
    let vars = json!({ "id": ch.id, "after": cursor, "types": types, "limit": PAGE });
    read_list(ch.read(LIST, vars).await?)
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
pub async fn rename(ch: &Channel<'_>, id: &str, title: &str) -> Result<String> {
    let title = clean_title(title).map_err(|c| Error::Refused(c.into()))?;
    let p = ch.write(UPDATE, "updateVideo", json!({ "videoID": id, "title": title })).await?;
    Ok(read_update(p)?.title.unwrap_or(title))
}

/// Publishes (`true`) or unpublishes a video. Returns whether it's public now.
pub async fn publish(ch: &Channel<'_>, id: &str, public: bool) -> Result<bool> {
    let p = ch.write(UPDATE, "updateVideo", json!({ "videoID": id, "scope": scope(public) })).await?;
    let now = read_update(p)?.scope.map_or(public, |s| s.eq_ignore_ascii_case("PUBLIC"));
    if now == public { Ok(now) } else { Err(Error::Refused("UNCHANGED".into())) }
}

fn scope(public: bool) -> &'static str {
    if public { "PUBLIC" } else { "PRIVATE" }
}

#[derive(Deserialize)]
pub(crate) struct Updated {
    title: Option<String>,
    scope: Option<String>,
}

/// An `updateVideo` payload: the video as Twitch kept it.
pub(crate) fn read_update(payload: Value) -> Result<Updated> {
    #[derive(Deserialize)]
    struct Payload {
        video: Option<Updated>,
    }
    let p: Payload = serde_json::from_value(payload).map_err(Error::Decode)?;
    p.video.ok_or_else(|| Error::Refused(super::super::NOT_DONE.into()))
}

/// Publishes (`true`) or unpublishes up to [`DELETE_MAX`] videos at once, as the producer's
/// bulk buttons do, then reads their scopes back (CORR-04). Returns the ids that now are.
pub async fn publish_many(ch: &Channel<'_>, ids: &[String], public: bool) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > DELETE_MAX {
        return Err(Error::Refused("TOO_MANY".into()));
    }
    ch.write(UPDATE_MANY, "updateVideosViewability", json!({ "videoIDs": ids, "scope": scope(public) })).await?;
    let (text, vars) = scopes_query(ids);
    Ok(confirmed(&ch.read(&text, vars).await?, ids, public))
}

/// One read for several videos' scopes: `v0: video(id: $i0) { id scope }` each.
pub(crate) fn scopes_query(ids: &[String]) -> (String, Value) {
    let args: Vec<String> = (0..ids.len()).map(|i| format!("$i{i}: ID!")).collect();
    let fields: Vec<String> = (0..ids.len()).map(|i| format!("v{i}: video(id: $i{i}) {{ id scope }}")).collect();
    let vars: serde_json::Map<String, Value> = ids.iter().enumerate().map(|(i, id)| (format!("i{i}"), id.clone().into())).collect();
    (format!("query VideoScopes({}) {{ {} }}", args.join(", "), fields.join(" ")), Value::Object(vars))
}

/// The ids whose scope reads as asked.
pub(crate) fn confirmed(data: &Value, ids: &[String], public: bool) -> Vec<String> {
    ids.iter()
        .enumerate()
        .filter(|(i, _)| data[format!("v{i}")]["scope"].as_str().is_some_and(|s| s.eq_ignore_ascii_case(scope(public))))
        .map(|(_, id)| id.clone())
        .collect()
}

/// Deletes up to [`DELETE_MAX`] videos. Returns the ids Twitch says it deleted.
pub async fn delete(ch: &Channel<'_>, ids: &[String]) -> Result<Vec<String>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > DELETE_MAX {
        return Err(Error::Refused("TOO_MANY".into()));
    }
    read_delete(ch.write(DELETE, "deleteVideos", json!({ "videoIDs": ids })).await?)
}

/// The ids a `deleteVideos` payload lists.
pub(crate) fn read_delete(payload: Value) -> Result<Vec<String>> {
    let videos = payload.get("videos").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    Ok(videos.iter().filter_map(|v| v.get("id")?.as_str().map(str::to_owned)).collect())
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
    use crate::gql::dashboard::mock::{ID, off, query, serve};
    use crate::gql::dashboard::NOT_DONE;
    use super::*;

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
        let m = serve([r#"{"data":{"user":{"videos":{"totalCount":0,"edges":[]}}}}"#]);
        list(&m.channel(), Some(VideoKind::Highlight), Some("cur")).await.unwrap();
        let sent = &m.sent()[0];
        assert!(sent["query"].as_str().unwrap().contains("options: {includePrivate: true}"));
        assert_eq!(sent["variables"], json!({ "id": ID, "after": "cur", "types": ["HIGHLIGHT"], "limit": PAGE }));
    }

    #[tokio::test]
    async fn rename_sends_the_trimmed_title() {
        let m = serve([r#"{"data":{"updateVideo":{"video":{"id":"7","title":"Best run","scope":"PUBLIC"}}}}"#]);
        assert_eq!(rename(&m.channel(), "7", "  Best run ").await.unwrap(), "Best run");
        let sent = &m.sent()[0];
        assert!(sent["query"].as_str().unwrap().contains("updateVideo(input: $input)"));
        assert_eq!(sent["variables"], json!({ "input": { "videoID": "7", "title": "Best run" } }));
        // An empty title never leaves the app.
        assert!(matches!(rename(&off(), "7", " ").await, Err(Error::Refused(c)) if c == "EMPTY"));
    }

    #[tokio::test]
    async fn v107_r41_publish_many_reports_only_what_reads_back() {
        let m = serve([
            r#"{"data":{"updateVideosViewability":{"error":null}}}"#,
            r#"{"data":{"v0":{"id":"1","scope":"PRIVATE"},"v1":{"id":"2","scope":"PUBLIC"}}}"#,
            r#"{"data":{"updateVideosViewability":null}}"#,
            r#"{"data":{"v0":{"id":"1","scope":"PUBLIC"},"v1":null}}"#,
            r#"{"data":{"updateVideosViewability":{"error":{"code":"FORBIDDEN"}}}}"#,
        ]);
        let ids = vec!["1".to_owned(), "2".to_owned()];
        assert_eq!(publish_many(&m.channel(), &ids, false).await.unwrap(), ["1"], "2 stayed public");
        // `null` is done there; the read says what went.
        assert_eq!(publish_many(&m.channel(), &ids, true).await.unwrap(), ["1"]);
        assert!(matches!(publish_many(&m.channel(), &ids, true).await, Err(Error::Refused(c)) if c == "FORBIDDEN"));
        let sent = m.sent();
        assert!(query(&sent[0]).contains("updateVideosViewability(input: $input)"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "videoIDs": ["1", "2"], "scope": "PRIVATE" } }));
        assert_eq!(query(&sent[1]), "query VideoScopes($i0: ID!, $i1: ID!) { v0: video(id: $i0) { id scope } v1: video(id: $i1) { id scope } }");
        assert_eq!(sent[1]["variables"], json!({ "i0": "1", "i1": "2" }));
        assert_eq!(sent[2]["variables"]["input"]["scope"], "PUBLIC");
        assert_eq!(sent.len(), 5, "a refusal reads nothing back");
        let many: Vec<String> = (0..=DELETE_MAX).map(|i| i.to_string()).collect();
        assert!(matches!(publish_many(&off(), &many, true).await, Err(Error::Refused(_))));
    }

    #[tokio::test]
    async fn v107_r41_rename_null_is_not_done() {
        let m = serve([r#"{"data":{"updateVideo":null}}"#, r#"{"data":{"updateVideo":{"video":null}}}"#]);
        assert!(matches!(rename(&m.channel(), "7", "x").await, Err(Error::Refused(c)) if c == NOT_DONE));
        assert!(matches!(publish(&m.channel(), "7", true).await, Err(Error::Refused(c)) if c == NOT_DONE));
        m.sent();
    }

    #[tokio::test]
    async fn publish_sets_the_scope_and_checks_it() {
        let m = serve([
            r#"{"data":{"updateVideo":{"video":{"id":"7","title":"x","scope":"PRIVATE"}}}}"#,
            r#"{"data":{"updateVideo":{"video":{"id":"7","title":"x","scope":"PRIVATE"}}}}"#,
            r#"{"errors":[{"message":"service error"}],"data":{"updateVideo":null}}"#,
        ]);
        assert!(!publish(&m.channel(), "7", false).await.unwrap());
        // Twitch kept it private: not reported as done.
        assert!(matches!(publish(&m.channel(), "7", true).await, Err(Error::Refused(c)) if c == "UNCHANGED"));
        assert_eq!(publish(&m.channel(), "7", true).await.unwrap_err().api_message(), Some("service error"));
        let sent = m.sent();
        assert_eq!(sent[0]["variables"], json!({ "input": { "videoID": "7", "scope": "PRIVATE" } }));
        assert_eq!(sent[1]["variables"], json!({ "input": { "videoID": "7", "scope": "PUBLIC" } }));
    }

    #[tokio::test]
    async fn delete_sends_ids_and_reads_what_went() {
        let m = serve([r#"{"data":{"deleteVideos":{"videos":[{"id":"1"},null]}}}"#]);
        let ids = vec!["1".to_owned(), "2".to_owned()];
        assert_eq!(delete(&m.channel(), &ids).await.unwrap(), vec!["1".to_owned()]);
        let sent = &m.sent()[0];
        assert!(sent["query"].as_str().unwrap().starts_with("mutation VideoManagerActions_DeleteVideos"));
        assert_eq!(sent["variables"], json!({ "input": { "videoIDs": ["1", "2"] } }));
        let many: Vec<String> = (0..=DELETE_MAX).map(|i| i.to_string()).collect();
        assert!(matches!(delete(&off(), &many).await, Err(Error::Refused(_))));
        let m = serve([r#"{"data":{"deleteVideos":null}}"#]);
        assert!(matches!(delete(&m.channel(), &ids).await, Err(Error::Refused(c)) if c == NOT_DONE));
        m.sent();
    }
}
