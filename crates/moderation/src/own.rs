//! V107-P7 Mod actions we did ourselves, so their mod-action echo (EventSub/Hermes) adds
//! no second chat line: ours already says it. Deletes (P7), the lift before a new timeout
//! (M7) and the steps of a mod/VIP swap (M8). V107-R44 Our deletes' echoes do add the
//! line (the app shows its own only when no echo comes), and our timeouts' lengths are
//! kept so the echo says what we sent. V107-R44 One role change shows once, though
//! Twitch's feeds can say it twice ([`OwnEchoes::repeat`]).

use std::collections::VecDeque;

use cyex_eventsub::events::Moderate;

use crate::roles::RoleAction;

/// How long an own action waits for its echo, in milliseconds.
const KEEP_MS: i64 = 60_000;
/// Most own actions remembered at once.
const CAP: usize = 64;
/// V107-R44 How long a role change's second copy is still taken for the first.
const REPEAT_MS: i64 = 10_000;

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
pub struct OwnEchoes {
    marks: VecDeque<(Echo, i64)>,
    /// V107-R44 Timeouts we sent: (channel id, login, seconds, when).
    lengths: VecDeque<(String, String, u32, i64)>,
    /// V107-R44 The last role change seen per person: (channel id, login, action, when).
    roles: VecDeque<(String, String, RoleAction, i64)>,
}

impl OwnEchoes {
    /// We're doing `e` now.
    pub fn mark(&mut self, e: Echo, now_ms: i64) {
        self.expire(now_ms);
        if self.marks.len() == CAP {
            self.marks.pop_front();
        }
        self.marks.push_back((e, now_ms));
    }

    /// `e` failed: another mod's same action gets its line.
    pub fn forget(&mut self, e: &Echo) {
        self.marks.retain(|(i, _)| i != e);
    }

    /// Whether `e` was ours. Each mark answers once.
    pub fn take(&mut self, e: &Echo, now_ms: i64) -> bool {
        self.expire(now_ms);
        let at = self.marks.iter().position(|(i, _)| i == e);
        at.map(|i| self.marks.remove(i)).is_some()
    }

    /// V107-R44 We're timing `login` out for `secs` in channel `cid` (`None`: it failed).
    pub fn timeout(&mut self, cid: &str, login: &str, secs: Option<u32>, now_ms: i64) {
        self.expire(now_ms);
        let login = login.to_ascii_lowercase();
        self.lengths.retain(|(c, l, ..)| !(c == cid && *l == login));
        let Some(secs) = secs else { return };
        if self.lengths.len() == CAP {
            self.lengths.pop_front();
        }
        self.lengths.push_back((cid.to_owned(), login, secs, now_ms));
    }

    /// V107-R44 The length we sent for this timeout echo, once.
    pub fn sent_secs(&mut self, cid: &str, login: &str, now_ms: i64) -> Option<u32> {
        self.expire(now_ms);
        let at = self.lengths.iter().position(|(c, l, ..)| c == cid && l.eq_ignore_ascii_case(login))?;
        self.lengths.remove(at).map(|(.., secs, _)| secs)
    }

    /// V107-R44 Whether this role echo repeats the last role change seen for that person in
    /// that channel, within [`REPEAT_MS`]: Twitch can say one change twice (e.g. a mod timed
    /// out loses mod). Any other change is noted, so mod, unmod, mod each still show.
    pub fn repeat(&mut self, e: &Echo, now_ms: i64) -> bool {
        let Echo::Role { cid, login, action } = e else { return false };
        self.expire(now_ms);
        let at = self.roles.iter().position(|(c, l, ..)| c == cid && l == login);
        if let Some(i) = at
            && self.roles[i].2 == *action
        {
            return true;
        }
        if let Some(i) = at {
            self.roles.remove(i);
        }
        if self.roles.len() == CAP {
            self.roles.pop_front();
        }
        self.roles.push_back((cid.clone(), login.clone(), *action, now_ms));
        false
    }

    fn expire(&mut self, now_ms: i64) {
        self.marks.retain(|(_, at)| now_ms - at < KEEP_MS);
        self.lengths.retain(|(.., at)| now_ms - at < KEEP_MS);
        self.roles.retain(|(.., at)| now_ms - at < REPEAT_MS);
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

    /// V107-R44 Our delete's echo takes the mark (its line shows); the app's fallback line
    /// shows only when the mark is still there.
    #[test]
    fn v107_r44_delete_echo_or_fallback() {
        let mut o = OwnEchoes::default();
        o.mark(del("a"), 0);
        assert!(o.take(&del("a"), 500), "echo came");
        assert!(!o.take(&del("a"), 4_000), "no fallback line");
        o.mark(del("b"), 0);
        assert!(o.take(&del("b"), 4_000), "no echo: the fallback line shows");
    }

    #[test]
    fn v107_r44_own_timeout_lengths() {
        let mut o = OwnEchoes::default();
        o.timeout("9", "CaressZ", Some(7), 0);
        assert_eq!(o.sent_secs("8", "caressz", 10), None, "another channel");
        assert_eq!(o.sent_secs("9", "caressz", 10), Some(7));
        assert_eq!(o.sent_secs("9", "caressz", 20), None, "only once");
        o.timeout("9", "x", Some(5), 0);
        o.timeout("9", "x", Some(60), 1);
        assert_eq!(o.sent_secs("9", "X", 2), Some(60), "the last one sent counts");
        o.timeout("9", "y", Some(5), 0);
        o.timeout("9", "y", None, 1);
        assert_eq!(o.sent_secs("9", "y", 2), None, "refused");
        o.timeout("9", "z", Some(5), 0);
        assert_eq!(o.sent_secs("9", "z", KEEP_MS), None, "too late");
    }

    #[test]
    fn v107_r44_one_role_change_shows_once() {
        let mut o = OwnEchoes::default();
        let unmod = Echo::of(&moderate("unmod", Some("caressz"), None), "9").unwrap();
        assert!(!o.repeat(&unmod, 0), "first copy shows");
        assert!(o.repeat(&unmod, 300), "second copy, other feed");
        assert!(!o.repeat(&Echo::role("8", "caressz", RoleAction::Unmod), 400), "another channel");
        assert!(!o.repeat(&Echo::role("9", "bob", RoleAction::Unmod), 400), "another person");
        let modded = Echo::role("9", "caressz", RoleAction::Mod);
        assert!(!o.repeat(&modded, 500), "modded again");
        assert!(!o.repeat(&unmod, 600), "and unmodded again: a real change");
        assert!(!o.repeat(&unmod, 600 + REPEAT_MS), "long after: shows");
        assert!(!o.repeat(&del("a"), 0), "not a role change");
        assert!(!o.repeat(&del("a"), 1));
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
