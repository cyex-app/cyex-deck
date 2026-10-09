//! Stories (1.1), as twitch.tv shows them to signed-in viewers: its own persisted
//! operations, so they need a first-party token. A story is a video (or picture) file
//! with a caption. Hashes from twitch.tv's build, 2026/10/09.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, persisted};
use crate::{Http, Result};

const CAPABILITIES: [&str; 5] = ["FULL_FLATTENING", "MENTION_STICKER", "PORTRAIT_CLIP", "VIDEO", "RESHARE"];
const PREVIEW: (&str, &str) = (
    "StoryPreviewChannel",
    "8d4de0e4721b0b78ad73f183084ff057edb6754f21553c18ccccddd748783e7c",
);
const LIST: (&str, &str) = (
    "StoriesForViewers",
    "e939ddc9d900cff636b37f9b142a70a49bea931648654702742374c2d171f898",
);
const SEEN: (&str, &str) = (
    "updateStorySummaryMutation",
    "5fb4861553cdf4d9f1d194872389e107dbddf5c6f6243e40a883d5e1bb91b19d",
);

/// The ring around a channel's avatar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StoryRing {
    /// Some aren't seen yet: the ring shows in color.
    pub unseen: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Story {
    pub id: String,
    /// RFC 3339.
    pub created_at: String,
    pub expires_at: String,
    pub subs_only: bool,
    pub views: u32,
    /// The story as one video, if it has one.
    pub video: Option<String>,
    /// Its picture: the poster, or all there is.
    pub image: Option<String>,
    pub caption: String,
    /// A story made from a clip.
    pub clip: Option<String>,
}

/// `None` when the channel has no stories now.
pub async fn story_ring(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Option<StoryRing>> {
    let vars = json!({ "channelID": channel_id, "capabilities": CAPABILITIES });
    let data: PreviewData = persisted(http, auth, PREVIEW.0, PREVIEW.1, vars).await?;
    Ok(data
        .story_preview_for_channel
        .filter(|p| p.story.as_ref().is_some_and(|s| !s.is_null()))
        .map(|p| StoryRing {
            unseen: p.has_unseen_story.unwrap_or(false),
        }))
}

/// The channel's stories, oldest first.
pub async fn stories(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<Story>> {
    let vars = json!({
        "channelID": channel_id, "first": 20, "after": "",
        "capabilities": CAPABILITIES, "includesOnlyRecaps": false,
    });
    let data: ListData = persisted(http, auth, LIST.0, LIST.1, vars).await?;
    let edges = data.stories_for_viewers.map(|c| c.edges).unwrap_or_default();
    let mut list: Vec<Story> = edges.into_iter().filter_map(|e| e.node).filter_map(story).collect();
    list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(list)
}

/// Marks the channel's stories up to `created_at` as seen, like twitch.tv after viewing.
pub async fn mark_seen(http: &Http, auth: Auth<'_>, channel_id: &str, created_at: &str) -> Result<()> {
    let vars = json!({ "input": {
        "channelID": channel_id, "lastSeenStoryCreatedAt": created_at, "anchorType": "STORY",
    }});
    let _: Value = persisted(http, auth, SEEN.0, SEEN.1, vars).await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewData {
    story_preview_for_channel: Option<RawPreview>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPreview {
    has_unseen_story: Option<bool>,
    story: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListData {
    stories_for_viewers: Option<Connection>,
}

#[derive(Deserialize)]
struct Connection {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Option<RawStory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStory {
    id: String,
    created_at: Option<String>,
    expired_at: Option<String>,
    segment_type: Option<String>,
    views: Option<u32>,
    content: Option<RawContent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawContent {
    #[serde(default)]
    caption: Option<Vec<RawToken>>,
    #[serde(rename = "thumbnailAssetURL")]
    thumbnail_asset_url: Option<String>,
    flattened_asset: Option<RawAsset>,
    base_layer: Option<RawBase>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawToken {
    plain_text: Option<String>,
    emote_code: Option<String>,
    user_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAsset {
    #[serde(rename = "thumbnailURL")]
    thumbnail_url: Option<String>,
    #[serde(default)]
    renditions: Option<Vec<RawRendition>>,
}

#[derive(Deserialize)]
struct RawRendition {
    url: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Deserialize)]
struct RawBase {
    clip: Option<RawClip>,
}

#[derive(Deserialize)]
struct RawClip {
    slug: Option<String>,
}

fn story(s: RawStory) -> Option<Story> {
    let content = s.content?;
    let asset = content.flattened_asset;
    let video = asset.as_ref().and_then(|a| {
        let mut list: Vec<&RawRendition> = a.renditions.iter().flatten().collect();
        // The largest one; Twitch lists a single rendition today.
        list.sort_by_key(|r| std::cmp::Reverse(r.width.unwrap_or(0) * r.height.unwrap_or(0)));
        list.first().map(|r| r.url.clone())
    });
    let image = asset.and_then(|a| a.thumbnail_url).or(content.thumbnail_asset_url);
    if video.is_none() && image.is_none() {
        return None;
    }
    let caption = content
        .caption
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| t.plain_text.or(t.emote_code).or(t.user_name.map(|u| format!("@{u}"))))
        .collect::<Vec<_>>()
        .join("");
    Some(Story {
        id: s.id,
        created_at: s.created_at.unwrap_or_default(),
        expires_at: s.expired_at.unwrap_or_default(),
        subs_only: s.segment_type.as_deref() == Some("SUBSCRIBERS"),
        views: s.views.unwrap_or(0),
        video,
        image,
        caption: caption.trim().to_owned(),
        clip: content.base_layer.and_then(|b| b.clip?.slug),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stories_parse() {
        let d: ListData = serde_json::from_str(
            r#"{"storiesForViewers":{"pageInfo":{"hasNextPage":false},"edges":[
              {"cursor":"b","node":{"id":"2","createdAt":"2026-10-09T12:00:00Z","expiredAt":"2026-10-10T12:00:00Z",
               "segmentType":"PUBLIC","views":40,"content":{"caption":[{"plainText":"hi "},{"emoteCode":"Kappa"},{"userName":"bob"}],
               "thumbnailAssetURL":null,"flattenedAsset":{"thumbnailURL":"https://s/t.jpg",
               "renditions":[{"url":"https://s/small.mp4","width":360,"height":640},{"url":"https://s/big.mp4","width":720,"height":1280}]},
               "baseLayer":{"clip":{"slug":"Clip-1"}}}}},
              {"cursor":"a","node":{"id":"1","createdAt":"2026-10-09T10:00:00Z","expiredAt":null,"segmentType":"SUBSCRIBERS",
               "views":null,"content":{"caption":null,"thumbnailAssetURL":"https://s/img.png","flattenedAsset":null,"baseLayer":{}}}},
              {"cursor":"c","node":{"id":"3","content":{"flattenedAsset":null}}}]}}"#,
        )
        .unwrap();
        let mut list: Vec<Story> = d.stories_for_viewers.unwrap().edges.into_iter().filter_map(|e| e.node).filter_map(story).collect();
        list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        assert_eq!(list.len(), 2, "a story with nothing to show is left out");
        assert!(list[0].subs_only && list[0].video.is_none());
        assert_eq!(list[0].image.as_deref(), Some("https://s/img.png"));
        assert_eq!(list[1].video.as_deref(), Some("https://s/big.mp4"));
        assert_eq!(list[1].caption, "hi Kappa@bob");
        assert_eq!(list[1].clip.as_deref(), Some("Clip-1"));
    }

    #[test]
    fn ring_needs_a_story() {
        let d: PreviewData = serde_json::from_str(r#"{"storyPreviewForChannel":{"hasUnseenStory":true,"story":null}}"#).unwrap();
        assert!(d.story_preview_for_channel.unwrap().story.is_none());
    }
}
