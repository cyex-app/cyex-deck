//! 1.7 Search for channels and categories. `searchFor` is the one search whose paging
//! works signed out (checked 2026/10/10).

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::cards::{BOX_ART, CARD, Category, ChannelCard, Page, RawCard, RawCategory};
use super::{Auth, query};
use crate::{Http, Result};

/// Channels per page when asking for more.
const MORE: u32 = 30;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchResults {
    /// In Twitch's order of relevance.
    pub channels: Page<ChannelCard>,
    pub categories: Vec<Category>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    search_for: Option<Found>,
}

#[derive(Deserialize)]
struct Found {
    channels: Option<Section<RawCard>>,
    games: Option<Section<RawCategory>>,
}

#[derive(Deserialize)]
struct Section<T> {
    cursor: Option<String>,
    #[serde(default = "Vec::new")]
    edges: Vec<Item<T>>,
}

#[derive(Deserialize)]
struct Item<T> {
    item: Option<T>,
}

impl<T> Section<T> {
    fn items<U>(self, f: impl Fn(T) -> U) -> (Vec<U>, Option<String>) {
        let items: Vec<U> = self.edges.into_iter().filter_map(|e| e.item).map(f).collect();
        // The last page still has a cursor; an empty one means there's nothing after.
        let cursor = if items.is_empty() {
            None
        } else {
            self.cursor.filter(|c| !c.is_empty())
        };
        (items, cursor)
    }
}

fn search_query() -> String {
    format!(
        "query($q: String!, $options: SearchForOptions) {{ searchFor(userQuery: $q, platform: \"web\", options: $options) {{ \
         channels {{ cursor edges {{ item {{ ... on User {{ ...Card }} }} }} }} \
         games {{ cursor edges {{ item {{ ... on Game {{ id slug displayName viewersCount followersCount {BOX_ART} }} }} }} }} }} }} {CARD}"
    )
}

/// The best matches: about 10 channels and 10 categories.
pub async fn search(http: &Http, auth: Auth<'_>, text: &str) -> Result<SearchResults> {
    let data: Data = query(
        http,
        auth,
        &search_query(),
        json!({ "q": text, "options": { "shouldSkipDiscoveryControl": true } }),
    )
    .await?;
    let Some(found) = data.search_for else {
        return Ok(SearchResults {
            channels: Page {
                items: Vec::new(),
                cursor: None,
            },
            categories: Vec::new(),
        });
    };
    let (items, cursor) = found
        .channels
        .map_or((Vec::new(), None), |s| s.items(ChannelCard::from));
    let categories = found.games.map_or(Vec::new(), |s| s.items(Category::from).0);
    Ok(SearchResults {
        channels: Page { items, cursor },
        categories,
    })
}

/// More channels for the same text.
pub async fn search_channels(
    http: &Http,
    auth: Auth<'_>,
    text: &str,
    after: &str,
) -> Result<Page<ChannelCard>> {
    let options = json!({
        "shouldSkipDiscoveryControl": true,
        "targets": [{ "index": "CHANNEL", "cursor": after, "limit": MORE }],
    });
    let data: Data = query(
        http,
        auth,
        &search_query(),
        json!({ "q": text, "options": options }),
    )
    .await?;
    let (items, cursor) = data
        .search_for
        .and_then(|f| f.channels)
        .map_or((Vec::new(), None), |s| s.items(ChannelCard::from));
    Ok(Page { items, cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_parse() {
        let d: Data = serde_json::from_str(
            r#"{"searchFor":{"channels":{"cursor":"MTA=","edges":[{"item":{"id":"1","login":"a","displayName":"A",
              "profileImageURL":null,"lastBroadcast":null,"stream":null}},{"item":null}]},
              "games":{"cursor":"","edges":[{"item":{"id":"9","slug":"chess","displayName":"Chess","viewersCount":7,
              "followersCount":1,"boxArtURL":"b"}}]}}}"#,
        )
        .unwrap();
        let f = d.search_for.unwrap();
        let (channels, cursor) = f.channels.unwrap().items(ChannelCard::from);
        assert_eq!((channels.len(), cursor.as_deref()), (1, Some("MTA=")));
        let (games, none) = f.games.unwrap().items(Category::from);
        assert_eq!((games[0].slug.as_str(), none), ("chess", None));
    }
}
