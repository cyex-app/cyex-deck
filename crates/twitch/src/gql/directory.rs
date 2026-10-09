//! 1.7 Discover: top streams, categories and the streams in one. Raw queries, checked
//! anonymously 2026/10/10. Signed out, Twitch serves only the first page: any `after`
//! fails its integrity check.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use ts_rs::TS;

use super::cards::{BOX_ART, Category, ChannelCard, Page, RawCategory, RawConnection, RawStreamNode};
use super::{Auth, query};
use crate::{Http, Result};

/// `streams` takes at most 30 at once.
const TOP_STREAMS: u32 = 30;
/// `games` and `game.streams` take 100.
const PAGE: u32 = 100;

const STREAM_NODE: &str = "id title viewersCount createdAt type \
    broadcaster { id login displayName profileImageURL(width: 70) } freeformTags { name }";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StreamSort {
    #[default]
    Viewers,
    /// Fewest viewers first.
    ViewersAsc,
    /// Started most recently.
    Recent,
}

impl StreamSort {
    fn gql(self) -> &'static str {
        match self {
            StreamSort::Viewers => "VIEWER_COUNT",
            StreamSort::ViewersAsc => "VIEWER_COUNT_ASC",
            StreamSort::Recent => "RECENT",
        }
    }
}

/// What a stream list asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct StreamFilter {
    pub sort: StreamSort,
    /// Twitch's codes, uppercase (`EN`, `DE`). Empty is all.
    pub languages: Vec<String>,
    /// H4 A freeform tag, any case.
    pub tag: Option<String>,
}

impl StreamFilter {
    fn options(&self) -> Value {
        let mut o = Map::new();
        o.insert("sort".into(), self.sort.gql().into());
        if !self.languages.is_empty() {
            o.insert("languages".into(), json!(self.languages));
        }
        if let Some(tag) = self.tag.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            o.insert("freeformTags".into(), json!([tag]));
        }
        Value::Object(o)
    }
}

/// A category with what discover shows on top of its streams.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryStreams {
    pub category: Category,
    pub streams: Page<ChannelCard>,
}

#[derive(Deserialize)]
struct Streams {
    streams: Option<RawConnection<RawStreamNode>>,
}

/// 1.7 Live streams across Twitch.
pub async fn top_streams(
    http: &Http,
    auth: Auth<'_>,
    filter: &StreamFilter,
    after: Option<&str>,
) -> Result<Page<ChannelCard>> {
    let q = format!(
        "query($first: Int, $after: Cursor, $options: StreamOptions) {{ streams(first: $first, after: $after, options: $options) {{ \
         edges {{ cursor node {{ {STREAM_NODE} game {{ id slug displayName }} }} }} pageInfo {{ hasNextPage }} }} }}"
    );
    let vars = json!({ "first": TOP_STREAMS, "after": after, "options": filter.options() });
    let data: Streams = query(http, auth, &q, vars).await?;
    Ok(page(data.streams, |n| n.into_card(None)))
}

#[derive(Deserialize)]
struct Games {
    games: Option<RawConnection<RawCategory>>,
}

/// 1.7 Categories by viewers.
pub async fn top_categories(http: &Http, auth: Auth<'_>, after: Option<&str>) -> Result<Page<Category>> {
    let q = format!(
        "query($first: Int, $after: Cursor) {{ games(first: $first, after: $after, options: {{sort: VIEWER_COUNT}}) {{ \
         edges {{ cursor node {{ id slug displayName viewersCount followersCount {BOX_ART} }} }} pageInfo {{ hasNextPage }} }} }}"
    );
    let data: Games = query(http, auth, &q, json!({ "first": PAGE, "after": after })).await?;
    Ok(page(data.games, |g| Some(Category::from(g))))
}

#[derive(Deserialize)]
struct Game {
    game: Option<RawGame>,
}

#[derive(Deserialize)]
struct RawGame {
    #[serde(flatten)]
    category: RawCategory,
    streams: Option<RawConnection<RawStreamNode>>,
}

/// 1.7 One category and its streams. `None` if there's no such category.
pub async fn category(
    http: &Http,
    auth: Auth<'_>,
    slug: &str,
    filter: &StreamFilter,
    after: Option<&str>,
) -> Result<Option<CategoryStreams>> {
    let q = format!(
        "query($slug: String, $first: Int, $after: Cursor, $options: GameStreamOptions) {{ game(slug: $slug) {{ \
         id slug displayName viewersCount followersCount {BOX_ART} \
         streams(first: $first, after: $after, options: $options) {{ edges {{ cursor node {{ {STREAM_NODE} }} }} pageInfo {{ hasNextPage }} }} }} }}"
    );
    let vars = json!({ "slug": slug, "first": PAGE, "after": after, "options": filter.options() });
    let data: Game = query(http, auth, &q, vars).await?;
    Ok(data.game.map(|g| {
        let game = g.category.game_ref();
        CategoryStreams {
            streams: page(g.streams, |n| n.into_card(Some(&game))),
            category: g.category.into(),
        }
    }))
}

fn page<N, T>(conn: Option<RawConnection<N>>, f: impl FnMut(N) -> Option<T>) -> Page<T> {
    match conn {
        Some(c) => c.into_page(f),
        None => Page {
            items: Vec::new(),
            cursor: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_become_options() {
        let f = StreamFilter {
            sort: StreamSort::Recent,
            languages: vec!["DE".into()],
            tag: Some(" vtuber ".into()),
        };
        assert_eq!(
            f.options(),
            json!({ "sort": "RECENT", "languages": ["DE"], "freeformTags": ["vtuber"] })
        );
        assert_eq!(
            StreamFilter::default().options(),
            json!({ "sort": "VIEWER_COUNT" })
        );
    }

    #[test]
    fn a_category_parses_with_its_streams() {
        let d: Game = serde_json::from_str(
            r#"{"game":{"id":"509658","slug":"just-chatting","displayName":"Just Chatting","viewersCount":100,
              "followersCount":5,"boxArtURL":"b","streams":{"edges":[{"cursor":"x","node":{"id":"s","title":"t",
              "viewersCount":9,"createdAt":"c","type":"live","broadcaster":{"id":"1","login":"a","displayName":"A",
              "profileImageURL":"p"},"freeformTags":[]}}],"pageInfo":{"hasNextPage":false}}}}"#,
        )
        .unwrap();
        let g = d.game.unwrap();
        let game = g.category.game_ref();
        let streams = page(g.streams, |n| n.into_card(Some(&game)));
        assert_eq!(
            streams.items[0]
                .stream
                .as_ref()
                .unwrap()
                .game
                .as_ref()
                .unwrap()
                .slug,
            "just-chatting"
        );
        assert_eq!(Category::from(g.category).viewers, 100);
    }
}
