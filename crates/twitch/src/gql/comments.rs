//! P8 VOD chat replay: the comments around a point in a VOD. By time works signed out;
//! paging by cursor needs an integrity token, so the player asks by time again as it
//! plays. Persisted operation from twitch.tv, checked 2026/10/09.

use serde::Deserialize;
use serde_json::json;

use super::{Auth, persisted};
use crate::{Http, Result};

const OP: (&str, &str) = (
    "VideoCommentsByOffsetOrCursor",
    "b70a3591ff0f4e0313d126c6a1502d79a1c02baebb288227c582044aa76adf6a",
);

#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    pub id: String,
    /// Seconds into the VOD.
    pub offset: u32,
    /// RFC 3339.
    pub created_at: String,
    pub user_id: String,
    pub login: String,
    pub display: String,
    pub color: Option<String>,
    /// `(set, version)`.
    pub badges: Vec<(String, String)>,
    /// Text pieces, each with the Twitch emote id it shows, if any.
    pub pieces: Vec<(String, Option<String>)>,
}

/// About 50 comments from `offset` on, oldest first. Deleted users' comments are left out.
pub async fn comments(http: &Http, video_id: &str, offset: u32) -> Result<Vec<Comment>> {
    let vars = json!({ "videoID": video_id, "contentOffsetSeconds": offset });
    let data: Data = persisted(http, Auth::PUBLIC, OP.0, OP.1, vars).await?;
    let edges = data.video.and_then(|v| v.comments).map(|c| c.edges).unwrap_or_default();
    Ok(edges.into_iter().filter_map(|e| e.node).filter_map(comment).collect())
}

#[derive(Deserialize)]
struct Data {
    video: Option<RawVideo>,
}

#[derive(Deserialize)]
struct RawVideo {
    comments: Option<Connection>,
}

#[derive(Deserialize)]
struct Connection {
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    node: Option<RawComment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawComment {
    id: String,
    commenter: Option<RawUser>,
    content_offset_seconds: Option<u32>,
    created_at: Option<String>,
    message: Option<RawMessage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    login: String,
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMessage {
    #[serde(default)]
    fragments: Vec<RawFragment>,
    #[serde(default)]
    user_badges: Vec<RawBadge>,
    user_color: Option<String>,
}

#[derive(Deserialize)]
struct RawFragment {
    text: Option<String>,
    emote: Option<RawEmote>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEmote {
    #[serde(rename = "emoteID")]
    emote_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBadge {
    #[serde(rename = "setID")]
    set_id: Option<String>,
    version: Option<String>,
}

fn comment(c: RawComment) -> Option<Comment> {
    let user = c.commenter?;
    let message = c.message?;
    Some(Comment {
        id: c.id,
        offset: c.content_offset_seconds.unwrap_or(0),
        created_at: c.created_at.unwrap_or_default(),
        display: user.display_name.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| user.login.clone()),
        user_id: user.id,
        login: user.login,
        color: message.user_color.filter(|c| !c.is_empty()),
        badges: message
            .user_badges
            .into_iter()
            .filter_map(|b| Some((b.set_id?, b.version?)))
            .filter(|(s, _)| !s.is_empty())
            .collect(),
        pieces: message
            .fragments
            .into_iter()
            .filter_map(|f| Some((f.text?, f.emote.and_then(|e| e.emote_id))))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_parse() {
        let d: Data = serde_json::from_str(
            r##"{"video":{"id":"1","comments":{"edges":[
              {"cursor":"x","node":{"id":"c1","commenter":{"id":"7","login":"bob","displayName":"Bob"},"contentOffsetSeconds":48,
               "createdAt":"2026-10-08T17:59:25.331Z","message":{"fragments":[{"emote":null,"text":"hi "},
               {"emote":{"emoteID":"25","from":3},"text":"Kappa"}],"userBadges":[{"id":"x","setID":"subscriber","version":"2"}],
               "userColor":"#FF0000"}}},
              {"cursor":"y","node":{"id":"c2","commenter":null,"contentOffsetSeconds":50,"message":{"fragments":[]}}}],
              "pageInfo":{"hasNextPage":true}}}}"##,
        )
        .unwrap();
        let list: Vec<Comment> = d.video.unwrap().comments.unwrap().edges.into_iter().filter_map(|e| e.node).filter_map(comment).collect();
        assert_eq!(list.len(), 1, "deleted users are left out");
        let c = &list[0];
        assert_eq!((c.offset, c.display.as_str(), c.color.as_deref()), (48, "Bob", Some("#FF0000")));
        assert_eq!(c.badges, [("subscriber".to_owned(), "2".to_owned())]);
        assert_eq!(c.pieces[1], ("Kappa".to_owned(), Some("25".to_owned())));
    }
}
