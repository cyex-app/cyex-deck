//! V107-M7 Timeouts stack so the last one counts: what to do when Twitch refuses a ban
//! or timeout because one is already in place. Pure, the app runs the plan.
//!
//! Helix (docs, Ban User): a timeout over a timeout replaces it, a ban over a timeout
//! works, a timeout over a ban is refused ("is already banned"); 409 means another change
//! to that user is in flight, retry. GQL replaces a timeout itself too (seen 2026/10/19:
//! 60 s, 30 s, 600 s in a row, each end replaced the last); its `TARGET_ALREADY_BANNED`
//! doesn't say what is in place, so the app looks the state up to know which.

/// How Twitch said no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal<'a> {
    Helix { status: u16, message: &'a str },
    Gql(&'a str),
}

/// What the target has right now, from Twitch's ban status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    Nothing,
    Timeout,
    Ban,
}

/// The next step after a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// A real refusal: show it.
    Fail,
    /// Twitch was busy with this user, or the old one just ran out: send it again.
    Retry,
    /// One is in place, but which isn't known yet: look it up, then ask again.
    LookUp,
    /// Lift the current timeout, then apply ours. Only the last one counts.
    Replace,
    /// A timeout over a permanent ban: the ban stays, say so.
    KeepBan,
}

/// Whether Twitch refused because the user is already banned or timed out.
fn already(r: Refusal) -> bool {
    match r {
        Refusal::Helix { status, message } => status == 400 && message.to_ascii_lowercase().contains("already banned"),
        Refusal::Gql(code) => code == "TARGET_ALREADY_BANNED" || code == "ALREADY_BANNED",
    }
}

/// The plan for a refused ban (`timeout: false`) or timeout. `held` is the looked-up
/// state, `None` before the lookup.
pub fn plan(timeout: bool, r: Refusal, held: Option<Held>) -> Plan {
    if matches!(r, Refusal::Helix { status: 409, .. }) {
        return Plan::Retry;
    }
    if !already(r) {
        return Plan::Fail;
    }
    // Helix replaces a timeout itself, so "already banned" there means a permanent ban.
    let held = match (r, held) {
        (_, Some(h)) => h,
        (Refusal::Helix { .. }, None) => Held::Ban,
        (Refusal::Gql(_), None) => return Plan::LookUp,
    };
    match (held, timeout) {
        (Held::Nothing, _) => Plan::Retry,
        (Held::Timeout, _) => Plan::Replace,
        (Held::Ban, true) => Plan::KeepBan,
        (Held::Ban, false) => Plan::Fail,
    }
}

/// The line for [`Plan::KeepBan`].
pub fn keep_ban_line(who: &str) -> String {
    format!("{who} is banned. Unban them first to time them out.")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GQL: Refusal = Refusal::Gql("TARGET_ALREADY_BANNED");
    const HELIX: Refusal = Refusal::Helix { status: 400, message: "The user specified in the user_id field is already banned." };

    #[test]
    fn v107_m7_gql_looks_up_then_replaces_a_timeout() {
        assert_eq!(plan(true, GQL, None), Plan::LookUp);
        assert_eq!(plan(true, GQL, Some(Held::Timeout)), Plan::Replace);
        // A ban over a timeout also wins.
        assert_eq!(plan(false, GQL, None), Plan::LookUp);
        assert_eq!(plan(false, GQL, Some(Held::Timeout)), Plan::Replace);
    }

    #[test]
    fn v107_m7_a_permanent_ban_stays() {
        assert_eq!(plan(true, GQL, Some(Held::Ban)), Plan::KeepBan);
        assert_eq!(plan(true, HELIX, None), Plan::KeepBan);
        assert_eq!(plan(false, GQL, Some(Held::Ban)), Plan::Fail, "already banned");
        assert_eq!(plan(false, HELIX, None), Plan::Fail);
        assert_eq!(keep_ban_line("caressz"), "caressz is banned. Unban them first to time them out.");
    }

    #[test]
    fn v107_m7_busy_or_expired_retries() {
        assert_eq!(plan(true, Refusal::Helix { status: 409, message: "" }, None), Plan::Retry);
        assert_eq!(plan(true, GQL, Some(Held::Nothing)), Plan::Retry);
    }

    #[test]
    fn v107_m7_other_refusals_fail() {
        for r in [
            Refusal::Gql("TARGET_IS_MOD"),
            Refusal::Gql("FORBIDDEN"),
            Refusal::Helix { status: 400, message: "The user specified in the user_id field may not be banned." },
            Refusal::Helix { status: 403, message: "" },
            Refusal::Helix { status: 429, message: "" },
        ] {
            assert_eq!(plan(true, r, None), Plan::Fail, "{r:?}");
            assert_eq!(plan(true, r, Some(Held::Timeout)), Plan::Fail, "{r:?}");
        }
    }
}
