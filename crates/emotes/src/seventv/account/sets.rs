//! V106-31/32 Reads for set editing: what an account can edit, the target sets, one emote.

use serde::Deserialize;
use serde_json::{json, Value};

use super::set_query::{EDITOR_FOR, EDIT_SETS, EMOTE_BRIEF};
use super::{Client, Error};

/// The 7TV account behind a Twitch id and the channels it may edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reachable {
    pub user_id: String,
    pub name: Option<String>,
    pub set_id: Option<String>,
    pub grants: Vec<Grant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub user_id: String,
    pub name: Option<String>,
    pub set_id: Option<String>,
    pub accepted: bool,
    pub manage: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetKind {
    Normal,
    Personal,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetBrief {
    pub id: String,
    pub kind: SetKind,
    pub capacity: Option<u32>,
    pub owner_id: Option<String>,
    /// Live entries only (emote not null), as (emote id, alias).
    pub entries: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmoteBrief {
    pub id: String,
    /// The emote's own (default) name.
    pub name: String,
    pub private: bool,
    pub owner_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConn {
    #[serde(default)]
    platform_display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStyle {
    #[serde(default)]
    active_emote_set_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    #[serde(default)]
    main_connection: Option<RawConn>,
    #[serde(default)]
    style: Option<RawStyle>,
}

impl RawUser {
    fn name(&self) -> Option<String> {
        self.main_connection
            .as_ref()
            .and_then(|c| c.platform_display_name.clone())
    }
    fn set_id(&self) -> Option<String> {
        self.style.as_ref().and_then(|s| s.active_emote_set_id.clone())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOwner {
    id: String,
    #[serde(default)]
    main_connection: Option<RawConn>,
    #[serde(default)]
    style: Option<RawStyle>,
    #[serde(default)]
    editor_for: Vec<RawGrant>,
}

#[derive(Deserialize)]
struct RawGrant {
    #[serde(default)]
    state: String,
    #[serde(default)]
    permissions: Option<RawPerms>,
    #[serde(default)]
    user: Option<RawUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPerms {
    #[serde(default)]
    emote_set: Option<RawSetPerms>,
}

#[derive(Deserialize)]
struct RawSetPerms {
    #[serde(default)]
    manage: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBrief {
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    capacity: Option<i64>,
    #[serde(default)]
    owner_id: Option<String>,
    #[serde(default)]
    emotes: Option<RawItems>,
}

#[derive(Deserialize)]
struct RawItems {
    #[serde(default)]
    items: Vec<RawItem>,
}

#[derive(Deserialize)]
struct RawItem {
    id: String,
    alias: String,
    #[serde(default)]
    emote: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEmoteBrief {
    id: String,
    default_name: String,
    #[serde(default)]
    owner_id: Option<String>,
    #[serde(default)]
    flags: Option<RawPrivate>,
}

#[derive(Deserialize)]
struct RawPrivate {
    #[serde(default)]
    private: bool,
}

fn parse<T: serde::de::DeserializeOwned>(data: &Value, path: &str) -> Result<Option<T>, Error> {
    match data.pointer(path) {
        None => Err(Error::Bad),
        Some(v) if v.is_null() => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|_| Error::Bad),
    }
}

impl Client {
    /// Who this Twitch id is on 7TV and what it may edit. `None`: no 7TV account.
    pub async fn reach(&self, twitch_id: &str) -> Result<Option<Reachable>, Error> {
        let data = self.gql(None, EDITOR_FOR, json!({ "tw": twitch_id })).await?;
        let Some(raw) = parse::<RawOwner>(&data, "/users/userByConnection")? else {
            return Ok(None);
        };
        let grants = raw
            .editor_for
            .into_iter()
            .filter_map(|g| {
                let u = g.user?;
                Some(Grant {
                    name: u.name(),
                    set_id: u.set_id(),
                    user_id: u.id,
                    accepted: g.state == "ACCEPTED",
                    manage: g.permissions.and_then(|p| p.emote_set).is_some_and(|s| s.manage),
                })
            })
            .collect();
        Ok(Some(Reachable {
            name: raw.main_connection.and_then(|c| c.platform_display_name),
            set_id: raw.style.and_then(|s| s.active_emote_set_id),
            user_id: raw.id,
            grants,
        }))
    }

    /// Ids and aliases of the given sets (the first 50).
    pub async fn edit_sets(&self, ids: &[String]) -> Result<Vec<SetBrief>, Error> {
        let ids = &ids[..ids.len().min(50)];
        let data = self.gql(None, EDIT_SETS, json!({ "ids": ids })).await?;
        let list = parse::<Vec<RawBrief>>(&data, "/emoteSets/emoteSets")?.unwrap_or_default();
        Ok(list
            .into_iter()
            .map(|s| SetBrief {
                id: s.id,
                kind: match s.kind.as_str() {
                    "NORMAL" => SetKind::Normal,
                    "PERSONAL" => SetKind::Personal,
                    _ => SetKind::Other,
                },
                capacity: s.capacity.and_then(|c| u32::try_from(c).ok()),
                owner_id: s.owner_id,
                entries: s
                    .emotes
                    .map(|e| e.items)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|i| i.emote.as_ref().is_some_and(|e| !e.is_null()))
                    .map(|i| (i.id, i.alias))
                    .collect(),
            })
            .collect())
    }

    /// I106-3 One emote by id. `None`: deleted or unknown.
    pub async fn emote_brief(&self, id: &str) -> Result<Option<EmoteBrief>, Error> {
        let data = self.gql(None, EMOTE_BRIEF, json!({ "id": id })).await?;
        Ok(
            parse::<RawEmoteBrief>(&data, "/emotes/emote")?.map(|e| EmoteBrief {
                id: e.id,
                name: e.default_name,
                private: e.flags.is_some_and(|f| f.private),
                owner_id: e.owner_id,
            }),
        )
    }
}
