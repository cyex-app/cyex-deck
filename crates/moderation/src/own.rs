//! V107-P7 Mod actions we did ourselves, so their mod-action echo (EventSub/Hermes) adds
//! no second chat line: ours already says it. Deletes (P7), the lift before a new timeout
//! (M7) and the steps of a mod/VIP swap (M8).

use std::collections::VecDeque;

use cyex_eventsub::events::Moderate;

use crate::roles::RoleAction;

/// How long an own action waits for its echo, in milliseconds.
const KEEP_MS: i64 = 60_000;
/// Most own actions remembered at once.
const CAP: usize = 64;

/// S1 One own action whose echo adds no line. Logins are lower case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Echo {
    /// We deleted this message.
    Delete(String),
    /// We lifted this timeout only to put a new one on.
    Lift { cid: String, login: String },
    /// One step of our mod/VIP swap.
    Role { cid: String, login: String, action: RoleAction },
}

impl Echo {
    pub fn lift(cid: &str, login: &str) -> Echo {
        Echo::Lift { cid: cid.to_owned(), login: login.to_ascii_lowercase() }
    }

    pub fn role(cid: &str, login: &str, action: RoleAction) -> Echo {
        Echo::Role { cid: cid.to_owned(), login: login.to_ascii_lowercase(), action }
    }

    /// The own action a mod-action echo in channel `cid` would answer, if it can be one.
    pub fn of(m: &Moderate, cid: &str) -> Option<Echo> {
        if m.action == "delete" {
            return m.message_id.clone().map(Echo::Delete);
        }
        let t = m.target.as_ref()?;
        if m.action == "untimeout" || m.action == "unban" {
            return Some(Echo::lift(cid, &t.login));
        }
        RoleAction::parse(&m.action).map(|a| Echo::role(cid, &t.login, a))
    }
}

#[derive(Default)]
pub struct OwnEchoes(VecDeque<(Echo, i64)>);

impl OwnEchoes {
    /// We're doing `e` now.
    pub fn mark(&mut self, e: Echo, now_ms: i64) {
        self.expire(now_ms);
        if self.0.len() == CAP {
            self.0.pop_front();
        }
        self.0.push_back((e, now_ms));
    }

    /// `e` failed: another mod's same action gets its line.
    pub fn forget(&mut self, e: &Echo) {
        self.0.retain(|(i, _)| i != e);
    }

    /// Whether `e` was ours. Each mark answers once.
    pub fn take(&mut self, e: &Echo, now_ms: i64) -> bool {
        self.expire(now_ms);
        let at = self.0.iter().position(|(i, _)| i == e);
        at.map(|i| self.0.remove(i)).is_some()
    }

    fn expire(&mut self, now_ms: i64) {
        self.0.retain(|(_, at)| now_ms - at < KEEP_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_eventsub::events::Who;

    fn del(id: &str) -> Echo {
        Echo::Delete(id.into())
    }

    #[test]
    fn v107_p7_own_delete_echo_is_dropped_once() {
        let mut o = OwnEchoes::default();
        o.mark(del("a"), 0);
        assert!(!o.take(&del("b"), 10), "another mod's delete");
        assert!(o.take(&del("a"), 10), "our echo");
        assert!(!o.take(&del("a"), 20), "only once");
        o.mark(del("c"), 0);
        o.forget(&del("c"));
        assert!(!o.take(&del("c"), 10), "failed delete");
        o.mark(del("d"), 0);
        assert!(!o.take(&del("d"), KEEP_MS), "too late");
    }

    #[test]
    fn v107_p7_own_deletes_are_capped() {
        let mut o = OwnEchoes::default();
        for i in 0..=CAP {
            o.mark(del(&i.to_string()), 0);
        }
        assert!(!o.take(&del("0"), 1), "oldest dropped");
        assert!(o.take(&del(&CAP.to_string()), 1));
    }

    fn moderate(action: &str, target: Option<&str>, msg: Option<&str>) -> Moderate {
        let who = |l: &str| Who { id: "1".into(), login: l.into(), name: l.into() };
        Moderate {
            moderator: who("kuni"),
            action: action.into(),
            target: target.map(who),
            reason: String::new(),
            expires_at: None,
            message_id: msg.map(Into::into),
            message_body: None,
            amount: None,
            terms: Vec::new(),
            source_channel: None,
        }
    }

    #[test]
    fn s1_echoes_from_mod_actions() {
        assert_eq!(Echo::of(&moderate("delete", Some("x"), Some("m1")), "9"), Some(del("m1")));
        assert_eq!(Echo::of(&moderate("delete", None, None), "9"), None);
        assert_eq!(Echo::of(&moderate("untimeout", Some("CaressZ"), None), "9"), Some(Echo::lift("9", "caressz")));
        assert_eq!(Echo::of(&moderate("unban", Some("caressz"), None), "9"), Some(Echo::lift("9", "CARESSZ")));
        assert_eq!(Echo::of(&moderate("vip", Some("bob"), None), "9"), Some(Echo::role("9", "bob", RoleAction::Vip)));
        assert_eq!(Echo::of(&moderate("unmod", None, None), "9"), None);
        assert_eq!(Echo::of(&moderate("timeout", Some("bob"), None), "9"), None);
        // Same login elsewhere, or another step, is someone else's.
        let mut o = OwnEchoes::default();
        o.mark(Echo::role("9", "bob", RoleAction::Unvip), 0);
        assert!(!o.take(&Echo::role("8", "bob", RoleAction::Unvip), 1));
        assert!(!o.take(&Echo::role("9", "bob", RoleAction::Mod), 1));
        assert!(o.take(&Echo::of(&moderate("unvip", Some("Bob"), None), "9").unwrap(), 1));
    }
}
