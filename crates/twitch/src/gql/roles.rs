//! V107-R28 Channel roles, as twitch.tv's `/leadmod` and `/unleadmod` send them (its web
//! chat bundle, 2026/10/20; docs/research/twitch-api.md, phase 10). Only the broadcaster may.

use serde_json::{Value, json};

use super::{Auth, query};
use crate::{Http, Result};

/// The lead moderator role's id.
pub const LEAD_MOD: &str = "lead_mod";

const ASSIGN: &str = "mutation AssignLeadMod($input: AssignChannelRoleInput!){ assignChannelRole(input:$input){ isAssigned error{ code } } }";
pub(crate) const UNASSIGN: &str = "mutation UnassignLeadMod($input: UnassignChannelRoleInput!){ unassignChannelRole(input:$input){ isUnassigned error{ code } } }";

/// Said when Twitch answers with neither a code nor a yes.
pub const NOT_DONE: &str = "NOT_DONE";

fn vars(channel_id: &str, target_id: &str, role: &str) -> Value {
    json!({ "input": { "channelID": channel_id, "targetUserID": target_id, "roleID": role } })
}

/// `None` when it went through, else Twitch's error code.
fn outcome(data: &Value, on: bool) -> Option<String> {
    let (field, done) = if on { ("assignChannelRole", "isAssigned") } else { ("unassignChannelRole", "isUnassigned") };
    let d = data.get(field);
    if let Some(code) = d.and_then(|d| d.get("error")).and_then(|e| e.get("code")).and_then(Value::as_str) {
        return Some(code.to_owned());
    }
    match d.and_then(|d| d.get(done)).and_then(Value::as_bool) {
        Some(true) => None,
        _ => Some(NOT_DONE.to_owned()),
    }
}

/// Gives (`on`) or takes `role` from the user with id `target_id`. `Ok(None)` when it went
/// through, `Ok(Some(code))` when Twitch refused.
pub async fn set_role(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str, role: &str, on: bool) -> Result<Option<String>> {
    let q = if on { ASSIGN } else { UNASSIGN };
    let data: Value = query(http, auth, q, vars(channel_id, target_id, role)).await?;
    Ok(outcome(&data, on))
}

/// V107-R36 The agent role's id (the Roles Manager's "Agent").
pub const AGENT: &str = "pre_agent";

/// The error code in `{ <field>: { error { code } } }`, if any.
fn code_of(data: &Value, field: &str) -> Option<String> {
    data.get(field)?.get("error")?.get("code")?.as_str().map(str::to_owned)
}

/// V107-R36 Gives or takes the editor role (`addEditor` / `removeEditor`). `Ok(Some(code))`
/// when Twitch refused.
pub async fn set_editor(http: &Http, auth: Auth<'_>, channel_id: &str, login: &str, on: bool) -> Result<Option<String>> {
    let (q, field) = if on {
        ("mutation AddEditorUser($input: AddEditorInput!){ addEditor(input:$input){ error{ code } } }", "addEditor")
    } else {
        ("mutation RemoveEditorUser($input: RemoveEditorInput!){ removeEditor(input:$input){ error{ code } } }", "removeEditor")
    };
    let vars = json!({ "input": { "channelID": channel_id, "targetUserLogin": login } });
    let data: Value = query(http, auth, q, vars).await?;
    Ok(code_of(&data, field))
}

/// V107-R36 Gives or takes the artist role (`grantCommunityRole` / `revokeCommunityRole`).
pub async fn set_artist(http: &Http, auth: Auth<'_>, channel_id: &str, login: &str, on: bool) -> Result<Option<String>> {
    let (q, field, key) = if on {
        ("mutation grantCommunityRole($input: GrantCommunityRoleInput!){ grantCommunityRole(input:$input){ error{ code } role } }", "grantCommunityRole", "granteeLogin")
    } else {
        ("mutation revokeCommunityRole($input: RevokeCommunityRoleInput!){ revokeCommunityRole(input:$input){ error{ code } role } }", "revokeCommunityRole", "revokeeLogin")
    };
    let vars = json!({ "input": { "channelID": channel_id, key: login, "role": "ARTIST" } });
    let data: Value = query(http, auth, q, vars).await?;
    Ok(code_of(&data, field))
}

/// Who holds `role` in the channel (the first 25 logins), as the dashboard's Roles page reads
/// it. Only the broadcaster may ask.
pub async fn holders(http: &Http, auth: Auth<'_>, channel_id: &str, role: &str) -> Result<Vec<String>> {
    let q = "query($id: ID!, $role: ID!){ channel(id:$id){ usersWithRole(id:$role, first:25){ edges{ node{ login } } } } }";
    let data: Value = query(http, auth, q, json!({ "id": channel_id, "role": role })).await?;
    Ok(logins(&data))
}

fn logins(data: &Value) -> Vec<String> {
    data.pointer("/channel/usersWithRole/edges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| e.pointer("/node/login").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_r28_input_is_the_web_chats() {
        let v = vars("1", "2", LEAD_MOD);
        assert_eq!(v, json!({ "input": { "channelID": "1", "targetUserID": "2", "roleID": "lead_mod" } }));
    }

    #[test]
    fn v107_r28_reads_twitchs_answers() {
        let ok = json!({ "assignChannelRole": { "isAssigned": true, "error": null } });
        assert_eq!(outcome(&ok, true), None);
        let ok = json!({ "unassignChannelRole": { "isUnassigned": true, "error": null } });
        assert_eq!(outcome(&ok, false), None);
        for code in ["ALREADY_ASSIGNED", "PERMISSION_DENIED", "ROLE_COUNT_LIMIT_REACHED", "USER_IS_BANNED", "CANNOT_BE_ASSIGNED"] {
            let no = json!({ "assignChannelRole": { "isAssigned": false, "error": { "code": code } } });
            assert_eq!(outcome(&no, true).as_deref(), Some(code));
        }
        let no = json!({ "unassignChannelRole": { "isUnassigned": false, "error": { "code": "NOT_ASSIGNED" } } });
        assert_eq!(outcome(&no, false).as_deref(), Some("NOT_ASSIGNED"));
        // No code and no yes: not done.
        let odd = json!({ "assignChannelRole": { "isAssigned": false, "error": null } });
        assert_eq!(outcome(&odd, true).as_deref(), Some(NOT_DONE));
        assert_eq!(outcome(&json!({ "assignChannelRole": null }), true).as_deref(), Some(NOT_DONE));
        // The other field doesn't count.
        assert_eq!(outcome(&ok, true).as_deref(), Some(NOT_DONE));
    }

    #[test]
    fn v107_r36_reads_error_codes() {
        let d = json!({ "addEditor": { "error": { "code": "TARGET_NOT_FOUND" } } });
        assert_eq!(code_of(&d, "addEditor").as_deref(), Some("TARGET_NOT_FOUND"));
        assert_eq!(code_of(&json!({ "addEditor": { "error": null } }), "addEditor"), None);
        assert_eq!(code_of(&json!({ "grantCommunityRole": { "error": null, "role": "ARTIST" } }), "grantCommunityRole"), None);
    }

    #[test]
    fn v107_r28_reads_holders() {
        let d = json!({ "channel": { "usersWithRole": { "edges": [{ "node": { "login": "caressz" } }, { "node": null }] } } });
        assert_eq!(logins(&d), ["caressz"]);
        assert!(logins(&json!({ "channel": null })).is_empty());
    }
}
