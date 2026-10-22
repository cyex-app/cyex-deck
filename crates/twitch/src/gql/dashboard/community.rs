//! V107-R40 Dashboard › Community: your channel's followers, and the channels where you
//! hold a role. Operations as Twitch's dashboard chunks send them (`community-followers-
//! list-page`, `community-my-assigned-roles-page`, 2026/10/21; docs/research/twitch-api.md).

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::Channel;
// V107-R42 One role type for every role screen.
pub use super::roles::Role;
use super::roles::UNASSIGN;
use crate::gql::connection::{GAP, Page, UserRef, page, paginate};
use crate::{Error, Result};

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

/// Most pages read for one role (100 channels each).
const MAX_PAGES: usize = 10;
/// Most followers one page asks for.
pub const PAGE: u32 = 50;

/// One follower of the channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmFollower")]
pub struct Follower {
    #[serde(flatten)]
    #[ts(flatten)]
    pub user: UserRef,
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
pub async fn followers(ch: &Channel<'_>, cursor: Option<&str>, newest: bool) -> Result<Followers> {
    let vars = json!({ "id": ch.id, "first": PAGE, "after": cursor, "order": if newest { "DESC" } else { "ASC" } });
    let data: Value = ch.read(FOLLOWERS, vars).await?;
    Ok(followers_from(&data))
}

fn followers_from(data: &Value) -> Followers {
    let f = &data["user"]["followers"];
    let p = page(f, |e| {
        Some(Follower { user: UserRef::from_node(&e["node"])?, followed_at: e["followedAt"].as_str().map(str::to_owned) })
    });
    Followers { total: f["totalCount"].as_u64().unwrap_or(p.items.len() as u64), items: p.items, next: p.next }
}

/// A channel where the account holds a role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmAssigned")]
pub struct Assigned {
    /// The channel.
    #[serde(flatten)]
    #[ts(flatten)]
    pub channel: UserRef,
    pub role: Role,
    /// RFC 3339, when Twitch says.
    pub since: Option<String>,
}

/// Every channel with a role, and whether a list was cut short.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "CmAssignedList")]
pub struct AssignedList {
    pub items: Vec<Assigned>,
    /// Twitch had more than was read (or a later page failed).
    pub truncated: bool,
}

fn assigned_page(conn: &Value, role: Role) -> Page<Assigned> {
    page(conn, |e| {
        Some(Assigned { channel: UserRef::from_node(&e["node"])?, role, since: e["grantedAt"].as_str().map(str::to_owned) })
    })
}

/// Reads every page of one connection (`path` into the data), [`MAX_PAGES`] at most.
async fn all(ch: &Channel<'_>, q: &str, vars: Value, path: &str, role: Role) -> Result<(Vec<Assigned>, bool)> {
    let p = paginate(None, MAX_PAGES, GAP, |after| {
        let mut v = vars.clone();
        v["after"] = after.map_or(Value::Null, Value::String);
        async move {
            let data: Value = ch.read(q, v).await?;
            Ok(assigned_page(data.pointer(path).unwrap_or(&Value::Null), role))
        }
    })
    .await?;
    Ok((p.items, p.truncated))
}

/// Every channel where the account (`ch`, your own) moderates, leads mods, edits or manages
/// business. A role Twitch won't answer for is left out (and says `truncated`); all of them
/// failing is an error.
pub async fn assigned(ch: &Channel<'_>) -> Result<AssignedList> {
    let mut out = AssignedList::default();
    let mut failed: Option<Error> = None;
    let mut note = |r: Result<(Vec<Assigned>, bool)>| match r {
        Ok((mut v, cut)) => {
            out.items.append(&mut v);
            out.truncated |= cut;
        }
        Err(e) => failed = Some(e),
    };
    note(all(ch, MODERATED, json!({}), "/moderatedChannels", Role::Mod).await);
    note(all(ch, EDITABLE, json!({}), "/currentUser/editableChannels", Role::Editor).await);
    for role in [Role::LeadMod, Role::Agent] {
        let id = role.channel_role_id();
        note(all(ch, WITH_ROLE, json!({ "role": id }), "/currentUser/channelsWithRole", role).await);
    }
    match failed {
        Some(e) if out.items.is_empty() => Err(e),
        Some(_) => Ok(AssignedList { truncated: true, ..out }),
        None => Ok(out),
    }
}

/// V107-Mw Whether you (`ch`) are a lead mod in `channel_id`, from Twitch's own list.
pub async fn lead_mod_in(ch: &Channel<'_>, channel_id: &str) -> Result<bool> {
    let vars = json!({ "role": Role::LeadMod.channel_role_id() });
    let (list, _) = all(ch, WITH_ROLE, vars, "/currentUser/channelsWithRole", Role::LeadMod).await?;
    Ok(list.iter().any(|a| a.channel.id == channel_id))
}

/// The mutation, its field and input for leaving `role` in `channel_id`: Twitch's own page
/// sends `targetID` for a mod and `targetUserID` for the rest. `None`: a role this list
/// never shows (VIP, artist).
fn step_down_call(role: Role, channel_id: &str, me: &str) -> Option<(&'static str, &'static str, Value)> {
    let unassign = |id: &str| {
        let input = json!({ "channelID": channel_id, "targetUserID": me, "roleID": id });
        (UNASSIGN, "unassignChannelRole", input)
    };
    Some(match role {
        Role::Mod => (UNMOD, "unmodUser", json!({ "channelID": channel_id, "targetID": me })),
        Role::Editor => (REMOVE_EDITOR, "removeEditor", json!({ "channelID": channel_id, "targetUserID": me })),
        Role::LeadMod | Role::Agent => unassign(role.channel_role_id()?),
        Role::Vip | Role::Artist => return None,
    })
}

/// Takes your own (`ch.id`) `role` in another channel.
pub async fn step_down(ch: &Channel<'_>, role: Role, channel_id: &str) -> Result<()> {
    let (q, field, input) = step_down_call(role, channel_id, ch.id).ok_or_else(|| Error::Refused("UNSUPPORTED".into()))?;
    let p = ch.write(q, field, input).await?;
    // The lead mod / manager payload says it too.
    if p.get("isUnassigned").and_then(Value::as_bool) == Some(false) {
        return Err(Error::Refused(super::NOT_DONE.into()));
    }
    Ok(())
}

/// V107-Mj What stepping down as mod did, with lead mod first.
#[derive(Debug)]
pub enum ModLeft {
    /// Mod gone; `lead`: lead mod gone too (or Twitch said it wasn't held).
    Done { lead: bool },
    /// Lead mod refused: mod was never sent.
    LeadKept(Error),
    /// Mod refused; `lead` as in `Done`.
    ModKept { lead: bool, err: Error },
}

/// V107-Mj Steps down as mod in `channel_id`. With `lead` (you hold lead mod there) lead
/// mod goes first, since `unmodUser` leaves a lead mod one; if that fails nothing else is
/// sent. NOT_ASSIGNED (a stale list) goes on to mod.
pub async fn step_down_mod(ch: &Channel<'_>, channel_id: &str, lead: bool) -> ModLeft {
    if lead {
        match step_down(ch, Role::LeadMod, channel_id).await {
            Ok(()) => {}
            Err(Error::Refused(c)) if c == "NOT_ASSIGNED" => {}
            Err(e) => return ModLeft::LeadKept(e),
        }
    }
    match step_down(ch, Role::Mod, channel_id).await {
        Ok(()) => ModLeft::Done { lead },
        Err(err) => ModLeft::ModKept { lead, err },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gql::dashboard::NOT_DONE;
    use crate::gql::dashboard::mock::{ID, query, serve};

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
        assert_eq!((f.items[1].user.display.as_str(), f.items[1].followed_at.clone()), ("bob", None));
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
    fn v107_r41_follower_serializes_flat() {
        let f = Follower {
            user: UserRef { id: "1".into(), login: "ann".into(), display: "Ann".into(), avatar: String::new() },
            followed_at: None,
        };
        assert_eq!(
            serde_json::to_value(f).unwrap(),
            json!({ "id": "1", "login": "ann", "display": "Ann", "avatar": "", "followedAt": null })
        );
    }

    #[test]
    fn v107_r40_reads_assigned_channels() {
        let conn = json!({ "edges": [
            { "cursor": "x", "grantedAt": "2025-01-02T03:04:05Z", "node": { "id": "9", "login": "caressz", "displayName": "Caressz", "profileImageURL": "p" } },
            { "cursor": "y", "node": { "id": "", "login": "gone" } }
        ] });
        let a = assigned_page(&conn, Role::LeadMod).items;
        assert_eq!(a.len(), 1);
        assert_eq!((a[0].role, a[0].since.as_deref()), (Role::LeadMod, Some("2025-01-02T03:04:05Z")));
        assert!(assigned_page(&Value::Null, Role::Editor).items.is_empty());
    }

    #[test]
    fn v107_r40_step_down_sends_twitchs_inputs() {
        let (_, f, i) = step_down_call(Role::Mod, "5", "6").unwrap();
        assert_eq!((f, i), ("unmodUser", json!({ "channelID": "5", "targetID": "6" })));
        let (_, f, i) = step_down_call(Role::Editor, "5", "6").unwrap();
        assert_eq!((f, i), ("removeEditor", json!({ "channelID": "5", "targetUserID": "6" })));
        let (_, f, i) = step_down_call(Role::LeadMod, "5", "6").unwrap();
        assert_eq!((f, i), ("unassignChannelRole", json!({ "channelID": "5", "targetUserID": "6", "roleID": "lead_mod" })));
        let (_, _, i) = step_down_call(Role::Agent, "5", "6").unwrap();
        assert_eq!(i["roleID"], "pre_agent");
        assert!(step_down_call(Role::Vip, "5", "6").is_none());
    }

    #[tokio::test]
    async fn v107_r41_step_down_against_the_mock() {
        let m = serve([
            json!({ "data": { "unmodUser": { "error": null } } }).to_string(),
            json!({ "data": { "removeEditor": { "error": { "code": "FORBIDDEN" } } } }).to_string(),
            json!({ "data": { "unassignChannelRole": { "isUnassigned": false, "error": null } } }).to_string(),
            json!({ "data": { "unassignChannelRole": null } }).to_string(),
            json!({ "data": { "unassignChannelRole": { "isUnassigned": true, "error": null } } }).to_string(),
        ]);
        let ch = m.channel();
        step_down(&ch, Role::Mod, "5").await.unwrap();
        assert!(matches!(step_down(&ch, Role::Editor, "5").await, Err(Error::Refused(c)) if c == "FORBIDDEN"));
        assert!(matches!(step_down(&ch, Role::LeadMod, "5").await, Err(Error::Refused(c)) if c == NOT_DONE));
        assert!(matches!(step_down(&ch, Role::Agent, "5").await, Err(Error::Refused(c)) if c == NOT_DONE));
        step_down(&ch, Role::Agent, "5").await.unwrap();
        let sent = m.sent();
        assert!(query(&sent[0]).contains("unmodUser(input: $input)"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "channelID": "5", "targetID": ID } }));
        assert!(query(&sent[1]).contains("removeEditor(input: $input)"));
        assert_eq!(sent[1]["variables"], json!({ "input": { "channelID": "5", "targetUserID": ID } }));
        assert!(query(&sent[2]).contains("unassignChannelRole(input:$input)"));
        assert_eq!(sent[4]["variables"], json!({ "input": { "channelID": "5", "targetUserID": ID, "roleID": "pre_agent" } }));
    }

    #[tokio::test]
    async fn v107_mj_step_down_mod_takes_lead_mod_first() {
        let lead_ok = json!({ "data": { "unassignChannelRole": { "isUnassigned": true, "error": null } } }).to_string();
        let unmod_ok = json!({ "data": { "unmodUser": { "error": null } } }).to_string();
        let m = serve([lead_ok.clone(), unmod_ok.clone()]);
        assert!(matches!(step_down_mod(&m.channel(), "5", true).await, ModLeft::Done { lead: true }));
        let sent = m.sent();
        assert_eq!(sent[0]["variables"]["input"]["roleID"], "lead_mod");
        assert!(query(&sent[1]).contains("unmodUser"));

        // Lead mod refused: unmod never sent (the mock would hang on a second request).
        let m = serve([json!({ "data": { "unassignChannelRole": { "isUnassigned": false, "error": { "code": "FORBIDDEN" } } } }).to_string()]);
        assert!(matches!(step_down_mod(&m.channel(), "5", true).await, ModLeft::LeadKept(Error::Refused(c)) if c == "FORBIDDEN"));
        assert_eq!(m.sent().len(), 1);

        // Stale: not a lead mod after all, so on to mod.
        let stale = json!({ "data": { "unassignChannelRole": { "isUnassigned": false, "error": { "code": "NOT_ASSIGNED" } } } }).to_string();
        let m = serve([stale, unmod_ok.clone()]);
        assert!(matches!(step_down_mod(&m.channel(), "5", true).await, ModLeft::Done { lead: true }));

        // Lead mod gone, mod refused.
        let m = serve([lead_ok, json!({ "data": { "unmodUser": { "error": { "code": "FORBIDDEN" } } } }).to_string()]);
        assert!(matches!(step_down_mod(&m.channel(), "5", true).await, ModLeft::ModKept { lead: true, .. }));

        // No lead mod held: mod only.
        let m = serve([unmod_ok]);
        assert!(matches!(step_down_mod(&m.channel(), "5", false).await, ModLeft::Done { lead: false }));
        assert!(query(&m.sent()[0]).contains("unmodUser"));
    }

    fn conn(field: &str, ids: &[&str], cursor: &str, more: bool) -> String {
        let edges: Vec<Value> = ids.iter().map(|i| json!({ "cursor": cursor, "node": { "id": i, "login": format!("l{i}") } })).collect();
        let c = json!({ "edges": edges, "pageInfo": { "hasNextPage": more } });
        let data = if field == "moderatedChannels" { json!({ field: c }) } else { json!({ "currentUser": { field: c } }) };
        json!({ "data": data }).to_string()
    }

    #[tokio::test]
    async fn v107_r41_assigned_pages_each_role_and_stops_on_a_repeat() {
        let m = serve([
            conn("moderatedChannels", &["1"], "m1", true),
            conn("moderatedChannels", &["2"], "m1", true),
            conn("editableChannels", &[], "", false),
            r#"{"errors":[{"message":"service timeout"}],"data":null}"#.to_owned(),
            conn("channelsWithRole", &["3"], "a", false),
        ]);
        let out = assigned(&m.channel()).await.unwrap();
        let ids: Vec<(&str, Role)> = out.items.iter().map(|a| (a.channel.id.as_str(), a.role)).collect();
        assert_eq!(ids, [("1", Role::Mod), ("2", Role::Mod), ("3", Role::Agent)]);
        assert!(out.truncated);
        let sent = m.sent();
        assert_eq!(sent[0]["variables"], json!({ "after": null }));
        assert_eq!(sent[1]["variables"], json!({ "after": "m1" }));
        assert_eq!(sent[3]["variables"], json!({ "role": "lead_mod", "after": null }));
        assert_eq!(sent[4]["variables"]["role"], "pre_agent");
    }
}
