//! V107-R36 The Roles Manager for the broadcaster's own channel, like the dashboard's
//! Community › Roles Manager: everyone with a role in one row each, VIP slots, and the steps
//! one change takes. Mod and VIP changes go through `role_change::set_role` (swaps, own-echo
//! marks), lead mod through `lead_mod`; the rest are single GQL mutations. No I/O here.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use cyex_twitch::gql::roles_list::{RoleHolder, RoleLists};

/// A role in the Roles Manager, in Twitch's order.
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
}

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

/// What the VIP slots allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VipState {
    /// Room for more.
    Open,
    /// Every slot taken.
    Full,
    /// No slots yet: they unlock through Twitch achievements.
    Locked,
    /// Twitch didn't say; a grant tells.
    Unknown,
}

/// The channel's VIP slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VipSlots {
    pub used: u32,
    #[ts(optional = nullable)]
    pub allowed: Option<u32>,
    pub state: VipState,
}

impl VipSlots {
    pub fn new(used: u32, allowed: Option<u32>) -> VipSlots {
        let state = match allowed {
            None => VipState::Unknown,
            Some(0) => VipState::Locked,
            Some(n) if used >= n => VipState::Full,
            Some(_) => VipState::Open,
        };
        VipSlots { used, allowed, state }
    }

    /// What a grant's refusal teaches when Twitch gave no count.
    pub fn learn(self, code: &str) -> VipSlots {
        let state = match code.to_ascii_uppercase().as_str() {
            "MAX_VIPS_REACHED" => VipState::Full,
            "VIP_ACHIEVEMENT_INCOMPLETE" => VipState::Locked,
            _ => return self,
        };
        VipSlots { state, ..self }
    }

    /// Why a VIP can't be added now; `None` when one can (or it's unknown).
    pub fn blocked(self) -> Option<&'static str> {
        match self.state {
            VipState::Full => Some(FULL),
            VipState::Locked => Some(LOCKED),
            _ => None,
        }
    }
}

pub const FULL: &str = "Every VIP slot is taken. Remove a VIP first.";
pub const LOCKED: &str = "VIPs unlock through Twitch achievements.";

/// One mutation of a change: give (`on`) or take `role`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub role: Role,
    pub on: bool,
}

const fn step(role: Role, on: bool) -> Step {
    Step { role, on }
}

/// The mutations one change takes, given what they hold. Mod and VIP swap each other inside
/// `role_change::set_role`. Making someone lead mod makes them a mod too (seen live
/// 2026/10/21), so a VIP goes first. Lead mod and mod are separate otherwise: unmod leaves
/// lead mod, and a lead mod need not be a mod.
pub fn steps(role: Role, on: bool, holds: &[Role]) -> Vec<Step> {
    match (role, on) {
        (Role::LeadMod, true) if holds.contains(&Role::Vip) => vec![step(Role::Vip, false), step(Role::LeadMod, true)],
        _ => vec![step(role, on)],
    }
}

/// What a change needs from the app.
pub trait ManageOps: Sync {
    /// One mutation. `Err` is the line to show.
    fn step(&self, who: &str, s: Step) -> impl Future<Output = Result<(), String>> + Send;
}

/// Runs a change's steps in order. When one fails, the ones before are undone (last first)
/// so nobody is left half changed; `Err` is the failed step's line.
pub async fn change(ops: &impl ManageOps, who: &str, role: Role, on: bool, holds: &[Role]) -> Result<(), String> {
    let plan = steps(role, on, holds);
    for (i, s) in plan.iter().enumerate() {
        if let Err(e) = ops.step(who, *s).await {
            for done in plan[..i].iter().rev() {
                let _ = ops.step(who, step(done.role, !done.on)).await;
            }
            return Err(e);
        }
    }
    Ok(())
}

/// A role change Twitch refused, in a short line. Unknown codes show the code.
pub fn refused(code: &str, who: &str, role: Role, on: bool) -> String {
    let r = role.name();
    match code.to_ascii_uppercase().as_str() {
        "TARGET_NOT_FOUND" | "GRANTEE_NOT_FOUND" | "REVOKEE_NOT_FOUND" | "USER_NOT_FOUND" | "TARGET_USER_NOT_FOUND" => {
            format!("No user called {who}.")
        }
        "TARGET_IS_VIP" => format!("{who} is a VIP."),
        "TARGET_ALREADY_MOD" => format!("{who} is already a mod."),
        "GRANTEE_ALREADY_VIP" => format!("{who} is already a VIP."),
        "REVOKEE_NOT_VIP" => format!("{who} isn’t a VIP."),
        "TARGET_NOT_MOD" => format!("{who} isn’t a mod."),
        "CANNOT_BE_VIP_AND_MOD" => format!("{who} can’t be a VIP and a mod."),
        "GRANTEE_CHAT_BANNED" | "USER_IS_BANNED" | "TARGET_BANNED" | "TARGET_IS_BANNED" => {
            format!("{who} is banned here. Unban them first.")
        }
        "MAX_VIPS_REACHED" => FULL.into(),
        "VIP_ACHIEVEMENT_INCOMPLETE" => LOCKED.into(),
        "ROLE_COUNT_LIMIT_REACHED" | "MAX_ARTISTS_REACHED" => format!("No {r} slots left."),
        "ALREADY_ASSIGNED" | "ALREADY_EDITOR" | "TARGET_ALREADY_EDITOR" | "GRANTEE_ALREADY_HAS_ROLE" => {
            format!("{who} is already {}.", article(r))
        }
        "NOT_ASSIGNED" | "TARGET_NOT_EDITOR" | "REVOKEE_DOES_NOT_HAVE_ROLE" => format!("{who} isn’t {}.", article(r)),
        "RATELIMITED" | "RATE_LIMITED" => "Too many changes at once. Try again in a moment.".into(),
        "PERMISSION_DENIED" | "FORBIDDEN" | "UNAUTHORIZED" => "Only the broadcaster can do that.".into(),
        "TARGET_IS_SELF" | "CANNOT_ASSIGN_TO_SELF" => "That’s you.".into(),
        "NOT_DONE" => format!("Twitch didn’t {} it. Try again.", if on { "do" } else { "undo" }),
        other => format!("Twitch refused it ({other})."),
    }
}

fn article(r: &str) -> String {
    let an = matches!(r.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u'));
    format!("{} {r}", if an { "an" } else { "a" })
}

/// The line after a change went through.
pub fn done(who: &str, role: Role, on: bool) -> String {
    let r = article(role.name());
    if on { format!("Made {who} {r}.") } else { format!("{who} is no longer {r}.") }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

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

    #[test]
    fn v107_r36_vip_slots() {
        assert_eq!(VipSlots::new(7, Some(10)).state, VipState::Open);
        assert_eq!(VipSlots::new(10, Some(10)).blocked(), Some(FULL));
        assert_eq!(VipSlots::new(0, Some(0)).blocked(), Some(LOCKED));
        let unknown = VipSlots::new(3, None);
        assert_eq!(unknown.blocked(), None);
        assert_eq!(unknown.learn("MAX_VIPS_REACHED").state, VipState::Full);
        assert_eq!(unknown.learn("vip_achievement_incomplete").state, VipState::Locked);
        assert_eq!(unknown.learn("GRANTEE_CHAT_BANNED"), unknown);
    }

    #[test]
    fn v107_r36_steps() {
        use Role::*;
        assert_eq!(steps(LeadMod, true, &[Vip]), [step(Vip, false), step(LeadMod, true)]);
        assert_eq!(steps(LeadMod, true, &[]), [step(LeadMod, true)]);
        assert_eq!(steps(LeadMod, true, &[Mod]), [step(LeadMod, true)], "mod stays");
        assert_eq!(steps(Mod, false, &[LeadMod, Mod]), [step(Mod, false)], "lead mod stays");
        assert_eq!(steps(Vip, true, &[LeadMod]), [step(Vip, true)]);
        assert_eq!(steps(Mod, true, &[Vip]), [step(Mod, true)], "set_role swaps");
        assert_eq!(steps(Editor, false, &[Editor]), [step(Editor, false)]);
    }

    struct Fake {
        fail: Option<Step>,
        sent: Mutex<Vec<Step>>,
    }

    impl ManageOps for Fake {
        fn step(&self, _who: &str, s: Step) -> impl Future<Output = Result<(), String>> + Send {
            self.sent.lock().unwrap().push(s);
            std::future::ready(if self.fail == Some(s) { Err("no".into()) } else { Ok(()) })
        }
    }

    fn run(f: &Fake, role: Role, on: bool, holds: &[Role]) -> Result<(), String> {
        let mut fut = std::pin::pin!(change(f, "bob", role, on, holds));
        match fut.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
            std::task::Poll::Ready(r) => r,
            std::task::Poll::Pending => panic!("the fake never waits"),
        }
    }

    #[test]
    fn v107_r36_a_failed_step_undoes_the_ones_before() {
        use Role::*;
        let f = Fake { fail: Some(step(LeadMod, true)), sent: Mutex::default() };
        assert_eq!(run(&f, LeadMod, true, &[Vip]), Err("no".into()));
        assert_eq!(*f.sent.lock().unwrap(), [step(Vip, false), step(LeadMod, true), step(Vip, true)]);
        let f = Fake { fail: Some(step(Vip, false)), sent: Mutex::default() };
        assert_eq!(run(&f, LeadMod, true, &[Vip]), Err("no".into()));
        assert_eq!(*f.sent.lock().unwrap(), [step(Vip, false)], "nothing to undo");
        let f = Fake { fail: None, sent: Mutex::default() };
        assert_eq!(run(&f, Artist, true, &[]), Ok(()));
    }

    #[test]
    fn v107_r36_lines() {
        use Role::*;
        assert_eq!(refused("GRANTEE_CHAT_BANNED", "bob", Vip, true), "bob is banned here. Unban them first.");
        assert_eq!(refused("CANNOT_BE_VIP_AND_MOD", "bob", Vip, true), "bob can’t be a VIP and a mod.");
        assert_eq!(refused("MAX_VIPS_REACHED", "bob", Vip, true), FULL);
        assert_eq!(refused("ROLE_COUNT_LIMIT_REACHED", "bob", Agent, true), "No agent slots left.");
        assert_eq!(refused("ALREADY_ASSIGNED", "bob", Editor, true), "bob is already an editor.");
        assert_eq!(refused("RATELIMITED", "bob", Mod, true), "Too many changes at once. Try again in a moment.");
        assert_eq!(refused("NEW_THING", "bob", Mod, true), "Twitch refused it (NEW_THING).");
        assert_eq!(done("bob", Artist, true), "Made bob an artist.");
        assert_eq!(done("bob", Vip, false), "bob is no longer a VIP.");
    }
}
