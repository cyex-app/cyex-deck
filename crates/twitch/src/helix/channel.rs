//! 1.35 The broadcaster's own channel: the current title, category, language and tags
//! (`GET /channels`), editing them (`PATCH /channels`, `channel:manage:broadcast`) and
//! searching categories to pick one (`GET /search/categories`).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

use super::{Auth, Data, get, request};
use crate::http::{empty, json};
use crate::{Http, Result};

/// What the channel shows now.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export, rename = "BroadcastInfo")]
pub struct ChannelInfo {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub game_id: String,
    #[serde(default)]
    pub game_name: String,
    /// Two-letter language code, e.g. `en`.
    #[serde(default)]
    pub broadcaster_language: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Reads the channel's title, category, language and tags.
pub async fn info(http: &Http, auth: Auth<'_>, broadcaster_id: &str) -> Result<ChannelInfo> {
    let req = get(http, auth, "/channels").query(&[("broadcaster_id", broadcaster_id)]);
    let d: Data<ChannelInfo> = json(req).await?;
    Ok(d.data.into_iter().next().unwrap_or_default())
}

/// A change to the channel. Only the set fields are sent.
#[derive(Clone, Debug, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ChannelEdit")]
pub struct Edit {
    pub title: Option<String>,
    /// `""` clears the category.
    pub game_id: Option<String>,
    /// The category's name. GQL sends it next to the id (D30); Helix ignores it.
    #[serde(default)]
    pub game_name: Option<String>,
    pub language: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl Edit {
    fn body(&self) -> Map<String, Value> {
        let mut m = Map::new();
        if let Some(t) = &self.title {
            m.insert("title".into(), t.clone().into());
        }
        if let Some(g) = &self.game_id {
            m.insert("game_id".into(), g.clone().into());
        }
        if let Some(l) = &self.language {
            m.insert("broadcaster_language".into(), l.clone().into());
        }
        if let Some(tags) = &self.tags {
            m.insert("tags".into(), tags.clone().into());
        }
        m
    }
}

/// 1.35 Edits the title, category, language or tags. Needs `channel:manage:broadcast`.
pub async fn set_info(http: &Http, auth: Auth<'_>, broadcaster_id: &str, edit: &Edit) -> Result<()> {
    let req = request(http, auth, reqwest::Method::PATCH, "/channels")
        .query(&[("broadcaster_id", broadcaster_id)])
        .json(&Value::Object(edit.body()));
    empty(req).await
}

/// A category (game) to set as the channel's.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export, rename = "CategoryHit")]
pub struct Category {
    pub id: String,
    pub name: String,
    /// Box art, with `{width}x{height}` placeholders.
    #[serde(default)]
    pub box_art_url: String,
}

/// 1.35 Categories whose name matches `query`, to change the channel's category.
pub async fn search_categories(http: &Http, auth: Auth<'_>, query: &str) -> Result<Vec<Category>> {
    let req = get(http, auth, "/search/categories").query(&[("query", query), ("first", "20")]);
    let d: Data<Category> = json(req).await?;
    Ok(d.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_parses() {
        let d: Data<ChannelInfo> = serde_json::from_str(
            r#"{"data":[{"broadcaster_id":"1","broadcaster_login":"b","broadcaster_name":"B",
            "broadcaster_language":"en","game_id":"509658","game_name":"Just Chatting","title":"hello",
            "delay":0,"tags":["English","VTuber"],"content_classification_labels":[],"is_branded_content":false}]}"#,
        )
        .unwrap();
        let c = d.data.into_iter().next().unwrap();
        assert_eq!(
            (c.title.as_str(), c.game_name.as_str()),
            ("hello", "Just Chatting")
        );
        assert_eq!(c.tags, ["English", "VTuber"]);
    }

    #[test]
    fn edit_sends_only_set_fields() {
        let b = Edit {
            title: Some("new".into()),
            game_id: Some(String::new()),
            ..Edit::default()
        }
        .body();
        assert_eq!(b["title"], "new");
        assert_eq!(b["game_id"], "");
        assert!(!b.contains_key("tags") && !b.contains_key("broadcaster_language"));
    }

    #[test]
    fn categories_parse() {
        let d: Data<Category> = serde_json::from_str(
            r#"{"data":[{"id":"33214","name":"Fortnite","box_art_url":"https://x/{width}x{height}.jpg"}]}"#,
        )
        .unwrap();
        assert_eq!(
            (d.data[0].id.as_str(), d.data[0].name.as_str()),
            ("33214", "Fortnite")
        );
    }
}
