//! Channels for tabs (1.14, 1.15, 1.29): look one up by what the user typed, and fetch
//! names and live status for many at once. Public data, no token.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::channel_input::{ChannelQuery, parse};
use crate::{Http, Result};

/// `users(ids:)` takes more, but large calls sometimes fail with "server error".
const BATCH: usize = 100;

const FIELDS: &str = "fragment C on User { id login displayName profileImageURL(width: 70) offlineImageURL \
    stream { type title viewersCount createdAt game { displayName } freeformTags { name } } }";

/// A channel and whether it's live right now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelInfo {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    /// V101-22 The picture Twitch's player shows while the channel is offline.
    pub offline_image: Option<String>,
    pub stream: Option<StreamInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamInfo {
    /// 1.29 Reruns get a yellow dot instead of a red one.
    pub rerun: bool,
    pub title: String,
    pub game: Option<String>,
    pub viewers: u32,
    /// RFC 3339.
    pub started_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    login: String,
    display_name: String,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    #[serde(rename = "offlineImageURL")]
    offline_image_url: Option<String>,
    stream: Option<RawStream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStream {
    #[serde(rename = "type")]
    kind: Option<String>,
    title: Option<String>,
    viewers_count: Option<u32>,
    created_at: Option<String>,
    game: Option<RawGame>,
    #[serde(default)]
    freeform_tags: Option<Vec<RawTag>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: String,
}

#[derive(Deserialize)]
struct RawTag {
    name: String,
}

impl From<RawUser> for ChannelInfo {
    fn from(u: RawUser) -> Self {
        Self {
            id: u.id,
            login: u.login,
            name: u.display_name,
            avatar: u.profile_image_url,
            offline_image: u.offline_image_url.filter(|u| !u.is_empty()),
            stream: u.stream.map(|s| {
                // Chatterino's forks also count a "Rerun" tag.
                let tagged = s.freeform_tags.unwrap_or_default().iter().any(|t| t.name.eq_ignore_ascii_case("rerun"));
                StreamInfo {
                    rerun: tagged || s.kind.as_deref() == Some("rerun"),
                    title: s.title.unwrap_or_default(),
                    game: s.game.map(|g| g.display_name),
                    viewers: s.viewers_count.unwrap_or(0),
                    started_at: s.created_at.unwrap_or_default(),
                }
            }),
        }
    }
}

#[derive(Deserialize)]
struct Users {
    users: Vec<Option<RawUser>>,
}

/// Channels by user id, in chunks. Unknown or banned ones are left out.
pub async fn channels_by_ids(http: &Http, ids: &[String]) -> Result<Vec<ChannelInfo>> {
    let q = format!("query($ids: [ID!]) {{ users(ids: $ids) {{ ...C }} }} {FIELDS}");
    let mut out = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(BATCH) {
        let vars = json!({ "ids": chunk });
        let data: Users = match query(http, Auth::PUBLIC, &q, vars.clone()).await {
            Err(e) if e.api_message() == Some("server error") => query(http, Auth::PUBLIC, &q, vars).await?,
            other => other?,
        };
        out.extend(data.users.into_iter().flatten().map(ChannelInfo::from));
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lookup {
    by_id: Option<RawUser>,
    by_login: Option<RawUser>,
}

/// 1.14 Channels matching a name, id or link. Digits alone can match two channels: one
/// with that id and one with that login.
pub async fn lookup(http: &Http, input: &str) -> Result<Vec<ChannelInfo>> {
    let Some(q) = parse(input) else { return Ok(Vec::new()) };
    let (by_id, by_login) = match &q {
        ChannelQuery::Login(l) => (None, Some(l.as_str())),
        ChannelQuery::Id(id) => (Some(id.as_str()), None),
        ChannelQuery::Either(s) => (Some(s.as_str()), Some(s.as_str())),
    };
    let mut fields = Vec::new();
    if by_id.is_some() {
        fields.push("byId: user(id: $id) { ...C }");
    }
    if by_login.is_some() {
        fields.push("byLogin: user(login: $login) { ...C }");
    }
    let vars = match (by_id.is_some(), by_login.is_some()) {
        (true, true) => "($id: ID!, $login: String!)",
        (true, false) => "($id: ID!)",
        _ => "($login: String!)",
    };
    let q = format!("query{vars} {{ {} }} {FIELDS}", fields.join(" "));
    let mut values = serde_json::Map::new();
    if let Some(id) = by_id {
        values.insert("id".into(), id.into());
    }
    if let Some(login) = by_login {
        values.insert("login".into(), login.into());
    }
    let data: Lookup = query(http, Auth::PUBLIC, &q, values.into()).await?;
    let mut out: Vec<ChannelInfo> = data.by_id.into_iter().map(ChannelInfo::from).collect();
    if let Some(u) = data.by_login
        && !out.iter().any(|c| c.id == u.id)
    {
        out.push(u.into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn users_parse_with_reruns_and_gaps() {
        let d: Users = serde_json::from_str(
            r#"{"users":[
              {"id":"1","login":"a","displayName":"A","profileImageURL":"p","offlineImageURL":"o","stream":{"type":"live","title":"hi",
               "viewersCount":5,"createdAt":"2026-10-09T10:00:00Z","game":{"displayName":"Chess"},"freeformTags":[]}},
              null,
              {"id":"2","login":"b","displayName":"B","profileImageURL":null,"stream":{"type":"live","title":"",
               "viewersCount":1,"createdAt":"x","game":null,"freeformTags":[{"name":"Rerun"}]}},
              {"id":"3","login":"c","displayName":"C","profileImageURL":null,"offlineImageURL":"","stream":null}]}"#,
        )
        .unwrap();
        let list: Vec<ChannelInfo> = d.users.into_iter().flatten().map(Into::into).collect();
        assert_eq!(list.len(), 3);
        let live = list[0].stream.as_ref().unwrap();
        assert!(!live.rerun);
        assert_eq!((live.viewers, live.game.as_deref()), (5, Some("Chess")));
        assert!(list[1].stream.as_ref().unwrap().rerun);
        assert!(list[2].stream.is_none());
        assert_eq!((list[0].offline_image.as_deref(), list[2].offline_image.as_deref()), (Some("o"), None));
    }

    #[test]
    fn lookup_keeps_both_matches_once() {
        let d: Lookup = serde_json::from_str(
            r#"{"byId":{"id":"71092938","login":"xqc","displayName":"xQc","stream":null},
                "byLogin":{"id":"882528598","login":"71092938","displayName":"71092938","stream":null}}"#,
        )
        .unwrap();
        assert_eq!(d.by_id.unwrap().login, "xqc");
        assert_eq!(d.by_login.unwrap().id, "882528598");
    }
}
