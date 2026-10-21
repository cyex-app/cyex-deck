//! V107-R40 Dashboard › Community: your channel's followers, and the channels where you
//! hold a role. Operations as Twitch's dashboard chunks send them (`community-followers-
//! list-page`, `community-my-assigned-roles-page`, 2026/10/21; docs/research/twitch-api.md).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Refusal, check};
use crate::gql::{Auth, query};
use crate::{Error, Http, Result};

const FOLLOWERS: &str = "query($id: ID!, $first: Int!, $after: Cursor, $order: SortOrder) { user(id: $id) { \
    followers(first: $first, after: $after, order: $order) { totalCount \
    edges { cursor followedAt node { id login displayName profileImageURL(width: 50) } } \
    pageInfo { hasNextPage } } } }";
const MODERATED: &str = "query($after: Cursor) { moderatedChannels(first: 100, after: $after) { \
    edges { cursor grantedAt node { id login displayName profileImageURL(width: 50) } } pageInfo { hasNextPage } } }";
const WITH_ROLE: &str = "query($role: ID!, $after: Cursor) { currentUser { id channelsWithRole(roleID: $role, first: 100, after: $after) { \
    edges { cursor grantedAt node: channel { id login displayName profileImageURL(width: 50) } } pageInfo { hasNextPage } } } }";
const EDITABLE: &str = "query($after: Cursor) { currentUser { id editableChannels(first: 100, after: $after) { \
    edges { cursor node { id login displayName profileImageURL(width: 50) } } pageInfo { hasNextPage } } } }";
const UNMOD: &str = "mutation($input: UnmodUserInput!) { unmodUser(input: $input) { error { code } } }";
const REMOVE_EDITOR: &str = "mutation($input: RemoveEditorInput!) { removeEditor(input: $input) { error { code } } }";

/// Twitch's role ids for `channelsWithRole`.
const LEAD_MOD: &str = "lead_mod";
const MANAGER: &str = "pre_agent";
/// Most pages read for one role (100 channels each).
const MAX_PAGES: usize = 10;
/// Most followers one page asks for.
pub const PAGE: u32 = 50;

/// One follower of the channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmFollower")]
pub struct Follower {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: String,
    /// RFC 3339, when Twitch says.
    pub followed_at: Option<String>,
}

/// A page of followers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmFollowers")]
pub struct Followers {
    #[ts(type = "number")]
    pub total: u64,
    pub items: Vec<Follower>,
    /// The cursor for the next page, `None` at the end.
    pub next: Option<String>,
}

/// Newest (`DESC`) or oldest first, after `cursor`.
pub async fn followers(http: &Http, auth: Auth<'_>, channel_id: &str, cursor: Option<&str>, newest: bool) -> Result<Followers> {
    let vars = json!({ "id": channel_id, "first": PAGE, "after": cursor, "order": if newest { "DESC" } else { "ASC" } });
    let data: Value = query(http, auth, FOLLOWERS, vars).await?;
    Ok(followers_from(&data))
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn followers_from(data: &Value) -> Followers {
    let f = &data["user"]["followers"];
    let edges = f["edges"].as_array().map(Vec::as_slice).unwrap_or_default();
    let items: Vec<Follower> = edges
        .iter()
        .filter_map(|e| {
            let n = &e["node"];
            let id = text(n, "id");
            if id.is_empty() {
                return None;
            }
            let login = text(n, "login");
            let name = text(n, "displayName");
            Some(Follower {
                id,
                name: if name.is_empty() { login.clone() } else { name },
                login,
                avatar: text(n, "profileImageURL"),
                followed_at: e["followedAt"].as_str().map(str::to_owned),
            })
        })
        .collect();
    let more = f["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false);
    let next = if more { edges.last().and_then(|e| e["cursor"].as_str()).map(str::to_owned) } else { None };
    Followers { total: f["totalCount"].as_u64().unwrap_or(items.len() as u64), items, next }
}

/// What the account is in a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmRole")]
pub enum Role {
    Moderator,
    LeadMod,
    Editor,
    Manager,
}

/// A channel where the account holds a role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmAssigned")]
pub struct Assigned {
    pub channel_id: String,
    pub login: String,
    pub name: String,
    pub avatar: String,
    pub role: Role,
    /// RFC 3339, when Twitch says.
    pub since: Option<String>,
}

fn assigned_from(edges: &Value, role: Role) -> Vec<Assigned> {
    edges
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|e| {
            let n = &e["node"];
            let channel_id = text(n, "id");
            if channel_id.is_empty() {
                return None;
            }
            let login = text(n, "login");
            let name = text(n, "displayName");
            Some(Assigned {
                channel_id,
                name: if name.is_empty() { login.clone() } else { name },
                login,
                avatar: text(n, "profileImageURL"),
                role,
                since: e["grantedAt"].as_str().map(str::to_owned),
            })
        })
        .collect()
}

/// Reads every page of one connection (`path` into the data), `MAX_PAGES` at most.
async fn all(http: &Http, auth: Auth<'_>, q: &str, vars: Value, path: &str, role: Role) -> Result<Vec<Assigned>> {
    let mut out = Vec::new();
    let mut after = Value::Null;
    for _ in 0..MAX_PAGES {
        let mut v = vars.clone();
        v["after"] = after.clone();
        let data: Value = query(http, auth, q, v).await?;
        let conn = data.pointer(path).unwrap_or(&Value::Null);
        out.extend(assigned_from(&conn["edges"], role));
        let last = conn["edges"].as_array().and_then(|e| e.last()).and_then(|e| e["cursor"].as_str());
        match (conn["pageInfo"]["hasNextPage"].as_bool(), last) {
            (Some(true), Some(c)) => after = Value::String(c.to_owned()),
            _ => break,
        }
    }
    Ok(out)
}

/// Every channel where the account moderates, leads mods, edits or manages business.
/// A role Twitch won't answer for is left out; all of them failing is an error.
pub async fn assigned(http: &Http, auth: Auth<'_>) -> Result<Vec<Assigned>> {
    let mut out = Vec::new();
    let mut failed: Option<Error> = None;
    let mut note = |r: Result<Vec<Assigned>>| match r {
        Ok(mut v) => out.append(&mut v),
        Err(e) => failed = Some(e),
    };
    note(all(http, auth, MODERATED, json!({}), "/moderatedChannels", Role::Moderator).await);
    note(all(http, auth, EDITABLE, json!({}), "/currentUser/editableChannels", Role::Editor).await);
    for (id, role) in [(LEAD_MOD, Role::LeadMod), (MANAGER, Role::Manager)] {
        note(all(http, auth, WITH_ROLE, json!({ "role": id }), "/currentUser/channelsWithRole", role).await);
    }
    match failed {
        Some(e) if out.is_empty() => Err(e),
        _ => Ok(out),
    }
}

/// The mutation and its input for leaving `role` in `channel_id`: Twitch's own page sends
/// `targetID` for a mod and `targetUserID` for the rest.
fn step_down_call(role: Role, channel_id: &str, me: &str) -> (&'static str, &'static str, Value) {
    let unassign = |id: &str| {
        let input = json!({ "channelID": channel_id, "targetUserID": me, "roleID": id });
        (super::super::roles::UNASSIGN, "unassignChannelRole", input)
    };
    match role {
        Role::Moderator => (UNMOD, "unmodUser", json!({ "channelID": channel_id, "targetID": me })),
        Role::Editor => (REMOVE_EDITOR, "removeEditor", json!({ "channelID": channel_id, "targetUserID": me })),
        Role::LeadMod => unassign(LEAD_MOD),
        Role::Manager => unassign(MANAGER),
    }
}

/// Takes your own `role` in a channel (`me` is your user id).
pub async fn step_down(http: &Http, auth: Auth<'_>, role: Role, channel_id: &str, me: &str) -> Result<()> {
    let (q, field, input) = step_down_call(role, channel_id, me);
    let data: Value = query(http, auth, q, json!({ "input": input })).await?;
    refused(&data, field)
}

fn refused(data: &Value, field: &str) -> Result<()> {
    let p = &data[field];
    if p.is_null() {
        return Err(Error::Refused("NOT_DONE".into()));
    }
    let code = p["error"]["code"].as_str().map(|c| Refusal { code: Some(c.to_owned()) });
    check(code)?;
    // The lead mod / manager payload says it too.
    if p.get("isUnassigned").and_then(Value::as_bool) == Some(false) {
        return Err(Error::Refused("NOT_DONE".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_r40_reads_a_followers_page() {
        let d = json!({ "user": { "followers": { "totalCount": 1234, "edges": [
            { "cursor": "a", "followedAt": "2026-10-20T21:05:00Z", "node": { "id": "1", "login": "ann", "displayName": "Ann", "profileImageURL": "u" } },
            { "cursor": "b", "followedAt": null, "node": { "id": "2", "login": "bob", "displayName": "", "profileImageURL": null } },
            { "cursor": "c", "node": null }
        ], "pageInfo": { "hasNextPage": true } } } });
        let f = followers_from(&d);
        assert_eq!(f.total, 1234);
        assert_eq!(f.items.len(), 2);
        assert_eq!(f.items[0].followed_at.as_deref(), Some("2026-10-20T21:05:00Z"));
        assert_eq!((f.items[1].name.as_str(), f.items[1].followed_at.clone()), ("bob", None));
        assert_eq!(f.next.as_deref(), Some("c"));
    }

    #[test]
    fn v107_r40_last_page_has_no_cursor_and_nothing_is_empty() {
        let d = json!({ "user": { "followers": { "totalCount": 1, "edges": [
            { "cursor": "a", "node": { "id": "1", "login": "ann", "displayName": "Ann" } }
        ], "pageInfo": { "hasNextPage": false } } } });
        assert_eq!(followers_from(&d).next, None);
        assert_eq!(followers_from(&json!({ "user": null })), Followers::default());
    }

    #[test]
    fn v107_r40_reads_assigned_channels() {
        let edges = json!([
            { "cursor": "x", "grantedAt": "2025-01-02T03:04:05Z", "node": { "id": "9", "login": "caressz", "displayName": "Caressz", "profileImageURL": "p" } },
            { "cursor": "y", "node": { "id": "", "login": "gone" } }
        ]);
        let a = assigned_from(&edges, Role::LeadMod);
        assert_eq!(a.len(), 1);
        assert_eq!((a[0].role, a[0].since.as_deref()), (Role::LeadMod, Some("2025-01-02T03:04:05Z")));
        assert!(assigned_from(&Value::Null, Role::Editor).is_empty());
    }

    #[test]
    fn v107_r40_step_down_sends_twitchs_inputs() {
        let (_, f, i) = step_down_call(Role::Moderator, "5", "6");
        assert_eq!((f, i), ("unmodUser", json!({ "channelID": "5", "targetID": "6" })));
        let (_, f, i) = step_down_call(Role::Editor, "5", "6");
        assert_eq!((f, i), ("removeEditor", json!({ "channelID": "5", "targetUserID": "6" })));
        let (_, f, i) = step_down_call(Role::LeadMod, "5", "6");
        assert_eq!((f, i), ("unassignChannelRole", json!({ "channelID": "5", "targetUserID": "6", "roleID": "lead_mod" })));
        let (_, _, i) = step_down_call(Role::Manager, "5", "6");
        assert_eq!(i["roleID"], "pre_agent");
    }

    #[test]
    fn v107_r40_reads_the_step_down_answer() {
        assert!(refused(&json!({ "unmodUser": { "error": null } }), "unmodUser").is_ok());
        assert!(refused(&json!({ "unassignChannelRole": { "isUnassigned": true, "error": null } }), "unassignChannelRole").is_ok());
        let no = refused(&json!({ "unmodUser": { "error": { "code": "FORBIDDEN" } } }), "unmodUser");
        assert!(matches!(no, Err(Error::Refused(c)) if c == "FORBIDDEN"));
        assert!(refused(&json!({ "unassignChannelRole": { "isUnassigned": false, "error": null } }), "unassignChannelRole").is_err());
        assert!(refused(&json!({ "unmodUser": null }), "unmodUser").is_err());
    }
}
