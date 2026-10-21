//! 1.35 Title, category, language and tags of your own channel, the category picker and
//! stream markers. `updateBroadcastSettings` calls the title `status`; tags are their own
//! mutation and replace the whole list.

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::channel::write;
use crate::gql::{Auth, query};
use crate::helix::channel::{Category, ChannelInfo, Edit};
use crate::{Error, Http, Result};

const READ: &str = "query($id: ID!) { user(id: $id) { \
    broadcastSettings { title language game { id displayName } } freeformTags { name } } }";
const SETTINGS: &str = "mutation($input: UpdateBroadcastSettingsInput!) { \
    updateBroadcastSettings(input: $input) { error } }";
const TAGS: &str =
    "mutation($input: SetFreeformTagsInput!) { setFreeformTags(input: $input) { error { code } } }";
const SEARCH: &str = "query($q: String!) { searchCategories(query: $q, first: 20) { \
    edges { node { id displayName boxArtURL(width: 52, height: 72) } } } }";
const MARKER: &str = "mutation($input: CreateVideoBookmarkInput!) { \
    createVideoBookmark(input: $input) { error { code } } }";

#[derive(Deserialize)]
struct Read {
    user: Option<User>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    broadcast_settings: Option<Settings>,
    #[serde(default)]
    freeform_tags: Option<Vec<Tag>>,
}

#[derive(Deserialize)]
struct Settings {
    title: Option<String>,
    language: Option<String>,
    game: Option<Game>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    id: String,
    display_name: String,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

pub async fn info(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<ChannelInfo> {
    let r: Read = query(http, auth, READ, json!({ "id": channel_id })).await?;
    let Some(user) = r.user else {
        return Ok(ChannelInfo::default());
    };
    let s = user.broadcast_settings;
    let game = s.as_ref().and_then(|s| s.game.as_ref());
    Ok(ChannelInfo {
        title: s.as_ref().and_then(|s| s.title.clone()).unwrap_or_default(),
        game_id: game.map(|g| g.id.clone()).unwrap_or_default(),
        game_name: game.map(|g| g.display_name.clone()).unwrap_or_default(),
        // GQL answers `EN`; Helix and the input use `en`.
        broadcaster_language: s.and_then(|s| s.language).unwrap_or_default().to_lowercase(),
        tags: user
            .freeform_tags
            .unwrap_or_default()
            .into_iter()
            .map(|t| t.name)
            .collect(),
    })
}

/// The `UpdateBroadcastSettingsInput` for an edit; `None` when it changes none of its
/// fields. Twitch's dashboard sends the category by id and name, so both go.
fn settings_input(channel_id: &str, edit: &Edit) -> Option<Value> {
    let mut m = Map::new();
    if let Some(t) = &edit.title {
        m.insert("status".into(), t.clone().into());
    }
    match edit.game_id.as_deref() {
        // Clearing: an empty name, as the old API took it.
        Some("") => {
            m.insert("game".into(), "".into());
        }
        Some(id) => {
            m.insert("categoryID".into(), id.into());
            if let Some(name) = &edit.game_name {
                m.insert("game".into(), name.clone().into());
            }
        }
        None => {}
    }
    if let Some(l) = &edit.language {
        m.insert("broadcasterLanguage".into(), l.to_lowercase().into());
    }
    if m.is_empty() {
        return None;
    }
    m.insert("userID".into(), channel_id.into());
    Some(Value::Object(m))
}

/// Changes what the edit sets. The settings error is a bare code, not `{ code }`.
pub async fn set_info(http: &Http, auth: Auth<'_>, channel_id: &str, edit: &Edit) -> Result<()> {
    if let Some(input) = settings_input(channel_id, edit) {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Data {
            update_broadcast_settings: Option<Payload>,
        }
        #[derive(Deserialize)]
        struct Payload {
            error: Option<String>,
        }
        let d: Data = query(http, auth, SETTINGS, json!({ "input": input })).await?;
        if let Some(code) = d.update_broadcast_settings.and_then(|p| p.error) {
            return Err(Error::Refused(code));
        }
    }
    if let Some(tags) = &edit.tags {
        let input = json!({ "contentID": channel_id, "contentType": "CHANNEL", "freeformTagNames": tags });
        write(http, auth, None, TAGS, "setFreeformTags", input).await?;
    }
    Ok(())
}

/// `…/509658-52x72.jpg` → `…/509658-{width}x{height}.jpg`, the Helix form the UI sizes.
fn art_template(url: &str) -> String {
    url.replace("-52x72.", "-{width}x{height}.")
}

pub async fn categories(http: &Http, auth: Auth<'_>, text: &str) -> Result<Vec<Category>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Data {
        search_categories: Option<Edges>,
    }
    #[derive(Deserialize)]
    struct Edges {
        #[serde(default)]
        edges: Vec<Edge>,
    }
    #[derive(Deserialize)]
    struct Edge {
        node: Node,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Node {
        id: String,
        display_name: String,
        #[serde(rename = "boxArtURL", default)]
        box_art_url: String,
    }
    let d: Data = query(http, auth, SEARCH, json!({ "q": text })).await?;
    Ok(d.search_categories
        .map(|s| s.edges)
        .unwrap_or_default()
        .into_iter()
        .map(|e| Category {
            id: e.node.id,
            name: e.node.display_name,
            box_art_url: art_template(&e.node.box_art_url),
        })
        .collect())
}

/// A stream marker on the live broadcast, as chat's `/marker` places it.
pub async fn marker(http: &Http, auth: Auth<'_>, channel_id: &str, description: &str) -> Result<()> {
    let input =
        json!({ "channelID": channel_id, "description": description, "medium": "chat", "platform": "web" });
    write(http, auth, None, MARKER, "createVideoBookmark", input).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_become_settings_input() {
        let edit = Edit {
            title: Some("hi".into()),
            game_id: Some("509658".into()),
            game_name: Some("Just Chatting".into()),
            language: Some("EN".into()),
            tags: None,
        };
        assert_eq!(
            settings_input("7", &edit).unwrap(),
            json!({ "userID": "7", "status": "hi", "categoryID": "509658", "game": "Just Chatting", "broadcasterLanguage": "en" })
        );
        let clear = Edit {
            game_id: Some(String::new()),
            ..Edit::default()
        };
        assert_eq!(
            settings_input("7", &clear).unwrap(),
            json!({ "userID": "7", "game": "" })
        );
        // Tags alone don't touch the settings.
        let tags = Edit {
            tags: Some(vec!["English".into()]),
            ..Edit::default()
        };
        assert!(settings_input("7", &tags).is_none());
    }

    #[test]
    fn box_art_gets_size_placeholders() {
        assert_eq!(
            art_template("https://static-cdn.jtvnw.net/ttv-boxart/509658-52x72.jpg"),
            "https://static-cdn.jtvnw.net/ttv-boxart/509658-{width}x{height}.jpg"
        );
    }
}
