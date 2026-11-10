//! Channels, streams and categories the way home and discover show them (1.6, 1.7).
//! Checked anonymously 2026/10/10 (`research/twitch-api.md`, "Home and discover").

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

/// `users(ids:)` takes more, but large calls sometimes fail with "server error".
const BATCH: usize = 100;

/// Every query that lists channels selects this on `User`.
pub(crate) const CARD: &str = "fragment Card on User { id login displayName profileImageURL(width: 70) \
    lastBroadcast { startedAt title game { displayName } } stream { type title viewersCount createdAt \
    game { id slug displayName } freeformTags { name } } }";

/// Box art at twice its 72×96 spot.
pub(crate) const BOX_ART: &str = "boxArtURL(width: 144, height: 192)";

/// A channel, and its stream when live.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelCard {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    pub stream: Option<LiveStream>,
    /// H1 When it last went live (RFC 3339). The current stream when live.
    pub last_live: Option<String>,
    /// Category of the last broadcast, for offline channels.
    pub last_game: Option<String>,
    /// Title of the last broadcast, for offline channels.
    pub last_title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LiveStream {
    pub title: String,
    pub viewers: u32,
    /// RFC 3339.
    pub started_at: String,
    /// 1.29
    pub rerun: bool,
    pub game: Option<GameRef>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameRef {
    pub id: String,
    pub slug: String,
    pub name: String,
}

/// A category (game) in discover.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Category {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub box_art: Option<String>,
    pub viewers: u32,
    pub followers: Option<u32>,
}

/// One page of a list. `cursor` is set when Twitch has more.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCard {
    // V110-A20 One odd card (null name, no login) must not fail a whole list.
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    login: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    last_broadcast: Option<RawBroadcast>,
    stream: Option<RawStream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBroadcast {
    started_at: Option<String>,
    title: Option<String>,
    game: Option<RawLastGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLastGame {
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawStream {
    #[serde(rename = "type")]
    kind: Option<String>,
    title: Option<String>,
    viewers_count: Option<u32>,
    created_at: Option<String>,
    game: Option<RawGameRef>,
    #[serde(default)]
    freeform_tags: Option<Vec<RawTag>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawGameRef {
    id: String,
    slug: Option<String>,
    display_name: String,
}

#[derive(Deserialize)]
struct RawTag {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawCategory {
    id: String,
    slug: Option<String>,
    display_name: String,
    #[serde(rename = "boxArtURL")]
    box_art_url: Option<String>,
    viewers_count: Option<u32>,
    followers_count: Option<u32>,
}

/// A node of `streams` or `game.streams`: the stream, with its channel inside.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawStreamNode {
    #[serde(flatten)]
    stream: RawStream,
    broadcaster: Option<RawBroadcaster>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBroadcaster {
    id: String,
    login: String,
    display_name: String,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawConnection<N> {
    // V110-A36 A connection without `edges` is a wrong answer (an error), not an empty list.
    edges: Vec<RawEdge<N>>,
    page_info: Option<RawPageInfo>,
}

#[derive(Deserialize)]
pub(crate) struct RawEdge<N> {
    cursor: Option<String>,
    node: Option<N>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPageInfo {
    has_next_page: bool,
}

impl<N> RawConnection<N> {
    /// The nodes, and the last cursor if Twitch says there's more.
    pub(crate) fn into_page<T>(self, f: impl FnMut(N) -> Option<T>) -> Page<T> {
        let more = self.page_info.is_some_and(|p| p.has_next_page);
        let cursor = if more {
            self.edges.last().and_then(|e| e.cursor.clone())
        } else {
            None
        };
        Page {
            items: self
                .edges
                .into_iter()
                .filter_map(|e| e.node)
                .filter_map(f)
                .collect(),
            cursor: cursor.filter(|c| !c.is_empty()),
        }
    }
}

impl RawStream {
    fn into_live(self, game: Option<RawGameRef>) -> LiveStream {
        // Chatterino's forks also count a "Rerun" tag.
        let tags: Vec<String> = self
            .freeform_tags
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.name)
            .collect();
        let tagged = tags.iter().any(|t| t.eq_ignore_ascii_case("rerun"));
        LiveStream {
            rerun: tagged || self.kind.as_deref() == Some("rerun"),
            title: self.title.unwrap_or_default(),
            viewers: self.viewers_count.unwrap_or(0),
            started_at: self.created_at.unwrap_or_default(),
            game: self.game.or(game).map(GameRef::from),
            tags,
        }
    }
}

impl From<RawGameRef> for GameRef {
    fn from(g: RawGameRef) -> Self {
        Self {
            slug: g.slug.unwrap_or_default(),
            id: g.id,
            name: g.display_name,
        }
    }
}

impl RawCard {
    /// V110-A20 A null or empty display name falls back to the login; a card with no id or
    /// login is left out (`None`).
    pub(crate) fn into_card(self) -> Option<ChannelCard> {
        let u = self;
        let id = u.id.filter(|s| !s.is_empty())?;
        let login = u.login.filter(|s| !s.trim().is_empty())?;
        let name = u.display_name.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| login.clone());
        let (last_live, last_game, last_title) = match u.last_broadcast {
            Some(b) => (
                b.started_at,
                b.game.and_then(|g| g.display_name).filter(|s| !s.is_empty()),
                b.title.filter(|s| !s.trim().is_empty()),
            ),
            None => (None, None, None),
        };
        Some(ChannelCard {
            id,
            login,
            name,
            avatar: u.profile_image_url,
            stream: u.stream.map(|s| s.into_live(None)),
            last_live,
            last_game,
            last_title,
        })
    }
}

impl RawStreamNode {
    /// `game` fills in for lists inside a category, whose nodes leave it out.
    pub(crate) fn into_card(self, game: Option<&RawGameRef>) -> Option<ChannelCard> {
        let b = self.broadcaster?;
        let stream = self.stream.into_live(game.cloned());
        Some(ChannelCard {
            id: b.id,
            login: b.login,
            name: b.display_name,
            avatar: b.profile_image_url,
            last_live: Some(stream.started_at.clone()),
            last_game: None,
            last_title: None,
            stream: Some(stream),
        })
    }
}

impl RawCategory {
    pub(crate) fn game_ref(&self) -> RawGameRef {
        RawGameRef {
            id: self.id.clone(),
            slug: self.slug.clone(),
            display_name: self.display_name.clone(),
        }
    }
}

impl From<RawCategory> for Category {
    fn from(g: RawCategory) -> Self {
        Self {
            slug: g.slug.unwrap_or_default(),
            id: g.id,
            name: g.display_name,
            box_art: g.box_art_url,
            viewers: g.viewers_count.unwrap_or(0),
            followers: g.followers_count,
        }
    }
}

#[derive(Deserialize)]
struct Users {
    users: Vec<Option<RawCard>>,
}

/// Channels by user id, in the order asked. Unknown or banned ones are left out. Public.
pub async fn cards(http: &Http, ids: &[String]) -> Result<Vec<ChannelCard>> {
    let q = format!("query($ids: [ID!]) {{ users(ids: $ids) {{ ...Card }} }} {CARD}");
    let mut out = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(BATCH) {
        let vars = json!({ "ids": chunk });
        let data: Users = match query(http, Auth::PUBLIC, &q, vars.clone()).await {
            Err(e) if e.api_message() == Some("server error") => query(http, Auth::PUBLIC, &q, vars).await?,
            other => other?,
        };
        out.extend(data.users.into_iter().flatten().filter_map(RawCard::into_card));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_user_turns_into_a_card() {
        let raw: RawCard = serde_json::from_str(
            r#"{"id":"1","login":"a","displayName":"A","profileImageURL":"p","lastBroadcast":{"startedAt":"2026-10-01T10:00:00.123456Z","title":"old one","game":{"displayName":"Just Chatting"}},
              "stream":{"type":"live","title":"hi","viewersCount":5,"createdAt":"2026-10-10T10:00:00Z",
              "game":{"id":"9","slug":"chess","displayName":"Chess"},"freeformTags":[{"name":"Rerun"}]}}"#,
        )
        .unwrap();
        let card = raw.into_card().unwrap();
        let s = card.stream.unwrap();
        assert!(s.rerun);
        assert_eq!((s.viewers, s.game.unwrap().slug.as_str()), (5, "chess"));
        assert_eq!(card.last_live.as_deref(), Some("2026-10-01T10:00:00.123456Z"));
        assert_eq!(card.last_game.as_deref(), Some("Just Chatting"));
        assert_eq!(card.last_title.as_deref(), Some("old one"));
    }

    fn last_of(broadcast: &str) -> ChannelCard {
        let raw: RawCard = serde_json::from_str(&format!(
            r#"{{"id":"1","login":"a","displayName":"A","profileImageURL":null,"stream":null{broadcast}}}"#
        ))
        .unwrap();
        raw.into_card().unwrap()
    }

    #[test]
    fn the_last_game_and_title_may_be_missing_null_or_there() {
        let missing = last_of("");
        assert_eq!((missing.last_live, missing.last_game, missing.last_title), (None, None, None));

        let null = last_of(r#","lastBroadcast":null"#);
        assert_eq!((null.last_game, null.last_title), (None, None));

        let bare = last_of(r#","lastBroadcast":{"startedAt":"x","title":null,"game":null}"#);
        assert_eq!((bare.last_live.as_deref(), bare.last_game, bare.last_title), (Some("x"), None, None));

        let empty = last_of(r#","lastBroadcast":{"startedAt":"x","title":" ","game":{"displayName":null}}"#);
        assert_eq!((empty.last_game, empty.last_title), (None, None));

        let full = last_of(r#","lastBroadcast":{"startedAt":"x","title":"t","game":{"displayName":"Chess"}}"#);
        assert_eq!((full.last_game.as_deref(), full.last_title.as_deref()), (Some("Chess"), Some("t")));
    }

    #[test]
    fn stream_nodes_page_with_the_category_filled_in() {
        let raw: RawConnection<RawStreamNode> = serde_json::from_str(
            r#"{"edges":[{"cursor":"c1","node":{"type":"live","title":"t","viewersCount":3,"createdAt":"x",
              "broadcaster":{"id":"2","login":"b","displayName":"B","profileImageURL":null},"freeformTags":null}},
              {"cursor":"c2","node":{"type":"live","broadcaster":null}}],"pageInfo":{"hasNextPage":true}}"#,
        )
        .unwrap();
        let game = RawGameRef {
            id: "9".into(),
            slug: Some("chess".into()),
            display_name: "Chess".into(),
        };
        let page = raw.into_page(|n| n.into_card(Some(&game)));
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.cursor.as_deref(), Some("c2"));
        assert_eq!(
            page.items[0].stream.as_ref().unwrap().game.as_ref().unwrap().name,
            "Chess"
        );

        let last: RawConnection<RawStreamNode> = serde_json::from_str(
            r#"{"edges":[{"cursor":"c","node":null}],"pageInfo":{"hasNextPage":false}}"#,
        )
        .unwrap();
        assert!(last.into_page(|n| n.into_card(None)).cursor.is_none());
    }

    #[test]
    fn v110_a20_one_odd_card_does_not_fail_the_list() {
        let mut cards: Vec<String> = (0..300)
            .map(|i| format!(r#"{{"id":"{i}","login":"u{i}","displayName":"U{i}","profileImageURL":null,"stream":null}}"#))
            .collect();
        cards[7] = r#"{"id":"7","login":"u7","displayName":null,"profileImageURL":null,"stream":null}"#.into();
        cards[8] = r#"{"id":"8","displayName":"X","stream":null}"#.into();
        cards[9] = r#"{"id":"9","login":"","displayName":"X"}"#.into();
        let raw: Users = serde_json::from_str(&format!(r#"{{"users":[{},null]}}"#, cards.join(","))).unwrap();
        let out: Vec<ChannelCard> = raw.users.into_iter().flatten().filter_map(RawCard::into_card).collect();
        assert_eq!(out.len(), 298);
        assert_eq!(out[7].name, "u7");
        assert_eq!(out[6].name, "U6");
    }

    #[test]
    fn v110_a36_a_connection_without_edges_does_not_decode() {
        assert!(serde_json::from_str::<RawConnection<RawCard>>(r#"{"pageInfo":{"hasNextPage":false}}"#).is_err());
        assert!(serde_json::from_str::<RawConnection<RawCard>>(r#"{"edges":[],"pageInfo":null}"#).is_ok());
    }
}
