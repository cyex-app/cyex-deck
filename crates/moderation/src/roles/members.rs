//! V107-R36 The Roles Manager's rows: everyone with a role in the channel, one row each.

use serde::Serialize;
use ts_rs::TS;

use cyex_twitch::gql::dashboard::roles::list::{RoleHolder, RoleLists};

use super::Role;

/// One row: someone and every role they hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "RoleMember")]
pub struct Member {
    pub id: String,
    pub login: String,
    pub display: String,
    pub avatar: String,
    /// In [`Role::ALL`] order.
    pub roles: Vec<Role>,
    /// The latest role grant (RFC 3339), if Twitch says.
    #[ts(optional = nullable)]
    pub modified: Option<String>,
}

/// Every list in one row per person, newest change first, then by login.
pub fn merge(lists: &RoleLists) -> Vec<Member> {
    let parts: [(Role, &Vec<RoleHolder>); 6] = [
        (Role::LeadMod, &lists.lead_mods),
        (Role::Mod, &lists.mods),
        (Role::Vip, &lists.vips),
        (Role::Editor, &lists.editors),
        (Role::Artist, &lists.artists),
        (Role::Agent, &lists.agents),
    ];
    let mut out: Vec<Member> = Vec::new();
    for (role, list) in parts {
        for h in list {
            let key = |m: &Member| if !h.id.is_empty() && !m.id.is_empty() { m.id == h.id } else { m.login == h.login };
            let i = match out.iter().position(key) {
                Some(i) => i,
                None => {
                    out.push(Member {
                        id: h.id.clone(),
                        login: h.login.clone(),
                        display: h.display.clone(),
                        avatar: h.avatar.clone(),
                        roles: Vec::new(),
                        modified: None,
                    });
                    out.len() - 1
                }
            };
            let m = &mut out[i];
            if !m.roles.contains(&role) {
                m.roles.push(role);
            }
            // RFC 3339 in UTC compares as text.
            if h.granted_at > m.modified {
                m.modified.clone_from(&h.granted_at);
            }
        }
    }
    for m in &mut out {
        m.roles.sort();
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.login.cmp(&b.login)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(login: &str, at: Option<&str>) -> RoleHolder {
        RoleHolder {
            id: format!("id-{login}"),
            login: login.into(),
            display: login.into(),
            avatar: String::new(),
            granted_at: at.map(Into::into),
        }
    }

    #[test]
    fn v107_r36_merges_rows_and_sorts_newest_first() {
        let lists = RoleLists {
            lead_mods: vec![h("ann", Some("2025-05-01T00:00:00Z"))],
            mods: vec![h("ann", Some("2024-01-01T00:00:00Z")), h("bob", Some("2023-01-01T00:00:00Z"))],
            vips: vec![h("cat", None)],
            editors: vec![h("bob", Some("2026-01-01T00:00:00Z"))],
            artists: vec![],
            agents: vec![h("ann", None)],
        };
        let m = merge(&lists);
        let rows: Vec<(&str, Vec<Role>, Option<&str>)> =
            m.iter().map(|m| (m.login.as_str(), m.roles.clone(), m.modified.as_deref())).collect();
        assert_eq!(rows, [
            ("bob", vec![Role::Mod, Role::Editor], Some("2026-01-01T00:00:00Z")),
            ("ann", vec![Role::LeadMod, Role::Mod, Role::Agent], Some("2025-05-01T00:00:00Z")),
            ("cat", vec![Role::Vip], None),
        ]);
    }
}
