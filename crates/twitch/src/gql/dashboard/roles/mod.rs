//! V107-R42 Channel roles over GQL, one place for all six: the [`Role`] every crate and the
//! IPC share, the mutation each one gives or takes with ([`assign`]), and who holds which
//! (`list`). Operations as twitch.tv sends them: its web chat for `/leadmod` (V107-R28,
//! 2026/10/20), the dashboard's Roles Manager for the rest (V107-R36, 2026/10/21);
//! docs/research/twitch-api.md. Only the broadcaster may, except mod and VIP (lead mods too).

pub mod list;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Channel, NOT_DONE};
use crate::{Error, Result};

/// A role in a channel, in Twitch's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ChannelRole")]
pub enum Role {
    LeadMod,
    Mod,
    Vip,
    Editor,
    Artist,
    Agent,
}

impl Role {
    pub const ALL: [Role; 6] = [Role::LeadMod, Role::Mod, Role::Vip, Role::Editor, Role::Artist, Role::Agent];

    /// For lines: "made them a {name}".
    pub fn name(self) -> &'static str {
        match self {
            Role::LeadMod => "lead mod",
            Role::Mod => "mod",
            Role::Vip => "VIP",
            Role::Editor => "editor",
            Role::Artist => "artist",
            Role::Agent => "agent",
        }
    }

    /// Twitch's id for the roles `assignChannelRole` and `usersWithRole` know.
    pub fn channel_role_id(self) -> Option<&'static str> {
        match self {
            Role::LeadMod => Some("lead_mod"),
            Role::Agent => Some("pre_agent"),
            _ => None,
        }
    }

    /// Whether its mutation names the user by id (else by login).
    pub fn by_id(self) -> bool {
        self.channel_role_id().is_some()
    }
}

/// Whom a change is about. `id` only matters where [`Role::by_id`].
#[derive(Clone, Copy, Debug)]
pub struct Who<'a> {
    pub login: &'a str,
    pub id: &'a str,
}

pub(crate) const UNASSIGN: &str = "mutation UnassignChannelRole($input: UnassignChannelRoleInput!){ unassignChannelRole(input:$input){ isUnassigned error{ code } } }";
const ASSIGN: &str = "mutation AssignChannelRole($input: AssignChannelRoleInput!){ assignChannelRole(input:$input){ isAssigned error{ code } } }";

/// The mutation, its payload field and input for giving (`on`) or taking `role`.
fn call(role: Role, on: bool, channel_id: &str, who: Who<'_>) -> (&'static str, &'static str, Value) {
    let by_login = |key: &str| json!({ "channelID": channel_id, key: who.login });
    match (role, on) {
        (Role::Mod, true) => ("mutation ModUser($input: ModUserInput!){ modUser(input:$input){ error{ code } } }", "modUser", by_login("targetLogin")),
        (Role::Mod, false) => ("mutation UnmodUser($input: UnmodUserInput!){ unmodUser(input:$input){ error{ code } } }", "unmodUser", by_login("targetLogin")),
        (Role::Vip, true) => ("mutation GrantVIP($input: GrantVIPInput!){ grantVIP(input:$input){ error{ code } } }", "grantVIP", by_login("granteeLogin")),
        (Role::Vip, false) => ("mutation RevokeVIP($input: RevokeVIPInput!){ revokeVIP(input:$input){ error{ code } } }", "revokeVIP", by_login("revokeeLogin")),
        (Role::Editor, true) => ("mutation AddEditorUser($input: AddEditorInput!){ addEditor(input:$input){ error{ code } } }", "addEditor", by_login("targetUserLogin")),
        (Role::Editor, false) => ("mutation RemoveEditorUser($input: RemoveEditorInput!){ removeEditor(input:$input){ error{ code } } }", "removeEditor", by_login("targetUserLogin")),
        (Role::Artist, true) => {
            let mut input = by_login("granteeLogin");
            input["role"] = json!("ARTIST");
            ("mutation grantCommunityRole($input: GrantCommunityRoleInput!){ grantCommunityRole(input:$input){ error{ code } role } }", "grantCommunityRole", input)
        }
        (Role::Artist, false) => {
            let mut input = by_login("revokeeLogin");
            input["role"] = json!("ARTIST");
            ("mutation revokeCommunityRole($input: RevokeCommunityRoleInput!){ revokeCommunityRole(input:$input){ error{ code } role } }", "revokeCommunityRole", input)
        }
        (Role::LeadMod | Role::Agent, on) => {
            let input = json!({ "channelID": channel_id, "targetUserID": who.id, "roleID": role.channel_role_id() });
            if on { (ASSIGN, "assignChannelRole", input) } else { (UNASSIGN, "unassignChannelRole", input) }
        }
    }
}

/// Gives (`on`) or takes `role` in `ch`'s channel. A refusal is `Err(Refused(code))`, by the
/// dashboard's one rule; a channel role's `isAssigned: false` with no code is [`NOT_DONE`].
pub async fn assign(ch: &Channel<'_>, role: Role, who: Who<'_>, on: bool) -> Result<()> {
    let (text, field, input) = call(role, on, ch.id, who);
    let p = ch.write(text, field, input).await?;
    let yes = if on { "isAssigned" } else { "isUnassigned" };
    if role.by_id() && p.get(yes).and_then(Value::as_bool) != Some(true) {
        return Err(Error::Refused(NOT_DONE.into()));
    }
    Ok(())
}

/// Who holds a channel role (the first 25 logins). Test builds' live check reads it back.
pub async fn holders(ch: &Channel<'_>, role: Role) -> Result<Vec<String>> {
    let q = "query($id: ID!, $role: ID!){ channel(id:$id){ usersWithRole(id:$role, first:25){ edges{ node{ login } } } } }";
    let data: Value = ch.read(q, json!({ "id": ch.id, "role": role.channel_role_id() })).await?;
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
    use crate::gql::dashboard::mock::{ID, query, serve};

    const BOB: Who<'static> = Who { login: "bob", id: "42" };

    #[test]
    fn v107_r42_each_role_sends_twitchs_input() {
        let cases: [(Role, bool, &str, Value); 12] = [
            (Role::Mod, true, "modUser", json!({ "channelID": "1", "targetLogin": "bob" })),
            (Role::Mod, false, "unmodUser", json!({ "channelID": "1", "targetLogin": "bob" })),
            (Role::Vip, true, "grantVIP", json!({ "channelID": "1", "granteeLogin": "bob" })),
            (Role::Vip, false, "revokeVIP", json!({ "channelID": "1", "revokeeLogin": "bob" })),
            (Role::Editor, true, "addEditor", json!({ "channelID": "1", "targetUserLogin": "bob" })),
            (Role::Editor, false, "removeEditor", json!({ "channelID": "1", "targetUserLogin": "bob" })),
            (Role::Artist, true, "grantCommunityRole", json!({ "channelID": "1", "granteeLogin": "bob", "role": "ARTIST" })),
            (Role::Artist, false, "revokeCommunityRole", json!({ "channelID": "1", "revokeeLogin": "bob", "role": "ARTIST" })),
            (Role::LeadMod, true, "assignChannelRole", json!({ "channelID": "1", "targetUserID": "42", "roleID": "lead_mod" })),
            (Role::LeadMod, false, "unassignChannelRole", json!({ "channelID": "1", "targetUserID": "42", "roleID": "lead_mod" })),
            (Role::Agent, true, "assignChannelRole", json!({ "channelID": "1", "targetUserID": "42", "roleID": "pre_agent" })),
            (Role::Agent, false, "unassignChannelRole", json!({ "channelID": "1", "targetUserID": "42", "roleID": "pre_agent" })),
        ];
        for (role, on, field, input) in cases {
            let (text, f, i) = call(role, on, "1", BOB);
            assert_eq!((f, &i), (field, &input), "{role:?} {on}");
            assert!(text.contains(&format!("{field}(input:$input)")), "{text}");
        }
    }

    #[test]
    fn v107_r42_names_and_ids() {
        assert_eq!(Role::ALL.map(Role::name), ["lead mod", "mod", "VIP", "editor", "artist", "agent"]);
        assert_eq!(Role::ALL.iter().filter(|r| r.by_id()).count(), 2);
        assert_eq!(serde_json::to_value(Role::LeadMod).unwrap(), json!("leadMod"));
    }

    fn refused(r: Result<()>) -> String {
        match r {
            Err(Error::Refused(c)) => c,
            other => panic!("not a refusal: {other:?}"),
        }
    }

    #[tokio::test]
    async fn v107_r42_reads_twitchs_answers() {
        let m = serve([
            json!({ "data": { "modUser": { "error": null } } }).to_string(),
            json!({ "data": { "grantVIP": { "error": { "code": "MAX_VIPS_REACHED" } } } }).to_string(),
            json!({ "data": { "revokeVIP": null } }).to_string(),
            json!({ "data": { "assignChannelRole": { "isAssigned": true, "error": null } } }).to_string(),
            json!({ "data": { "assignChannelRole": { "isAssigned": false, "error": null } } }).to_string(),
            json!({ "data": { "unassignChannelRole": { "isUnassigned": false, "error": { "code": "NOT_ASSIGNED" } } } }).to_string(),
            json!({ "data": { "addEditor": { "error": null } } }).to_string(),
        ]);
        let ch = m.channel();
        assign(&ch, Role::Mod, BOB, true).await.unwrap();
        assert_eq!(refused(assign(&ch, Role::Vip, BOB, true).await), "MAX_VIPS_REACHED");
        assert_eq!(refused(assign(&ch, Role::Vip, BOB, false).await), NOT_DONE, "no answer is no yes");
        assign(&ch, Role::LeadMod, BOB, true).await.unwrap();
        assert_eq!(refused(assign(&ch, Role::Agent, BOB, true).await), NOT_DONE);
        assert_eq!(refused(assign(&ch, Role::LeadMod, BOB, false).await), "NOT_ASSIGNED");
        assign(&ch, Role::Editor, BOB, true).await.unwrap();
        let sent = m.sent();
        assert!(query(&sent[0]).contains("modUser(input:$input)"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "channelID": ID, "targetLogin": "bob" } }));
        assert_eq!(sent[3]["variables"]["input"]["targetUserID"], "42");
    }

    #[test]
    fn v107_r28_reads_holders() {
        let d = json!({ "channel": { "usersWithRole": { "edges": [{ "node": { "login": "caressz" } }, { "node": null }] } } });
        assert_eq!(logins(&d), ["caressz"]);
        assert!(logins(&json!({ "channel": null })).is_empty());
    }
}
