//! V107-R21 Mass ban and unban outcomes, without I/O: what each of Twitch's answers means
//! (done, already so, gone, protected, throttled, something else), the tally the mass-action
//! panel shows, and the backoff for throttled answers. GQL and Helix word the same things
//! differently; both land on one [`Outcome`].

use serde::Serialize;
use ts_rs::TS;

/// How one login ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Banned (or timed out, or unbanned).
    Done,
    /// Already banned, or not banned when unbanning.
    Already,
    /// No such user, or suspended (Twitch hides them).
    Gone,
    /// A mod, VIP, the broadcaster, staff or yourself.
    Protected,
    /// Still throttled after every retry.
    Throttled,
    /// Anything else; the code says what.
    Other,
}

/// What to do with one answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Final; the code for [`Outcome::Other`].
    End(Outcome, Option<String>),
    /// Throttled or busy: wait and send it again.
    Retry,
    /// V107-M7 One is in place: the single-ban path decides (timeouts stack).
    Restack,
}

/// Throttle and busy signs in a GQL code or error message (Twitch has no fixed code).
fn throttled(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    ["rate limit", "rate_limit", "ratelimit", "throttl", "too many", "too_many", "slow down"]
        .iter()
        .any(|s| t.contains(s))
}

/// Twitch was briefly unable to answer: worth one more go.
fn busy(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    ["service timeout", "service unavailable", "service error", "server error", "timed out", "temporarily"]
        .iter()
        .any(|s| t.contains(s))
}

/// A GQL `banUserFromChatRoom`/`unbanUserFromChatRoom` answer: `None` went through, else
/// the payload's code or the entry's error message. `timeout` and `unban` say what was sent.
pub fn gql_step(code: Option<&str>, timeout: bool, unban: bool) -> Step {
    let Some(c) = code else { return Step::End(Outcome::Done, None) };
    match c {
        "TARGET_ALREADY_BANNED" | "ALREADY_BANNED" if !unban => {
            if crate::restack::redo(timeout, c) {
                Step::Restack
            } else {
                Step::End(Outcome::Already, None)
            }
        }
        "TARGET_NOT_BANNED" | "NOT_BANNED" if unban => Step::End(Outcome::Already, None),
        "TARGET_NOT_FOUND" | "TARGET_IS_ANONYMOUS" | "USER_NOT_FOUND" | "TARGET_SUSPENDED" | "TARGET_DELETED" => {
            Step::End(Outcome::Gone, None)
        }
        "TARGET_IS_MOD" | "TARGET_IS_MODERATOR" | "TARGET_IS_VIP" | "TARGET_IS_BROADCASTER" | "TARGET_IS_STAFF"
        | "TARGET_IS_ADMIN" | "TARGET_IS_GLOBAL_MOD" | "TARGET_IS_SELF" => Step::End(Outcome::Protected, None),
        c if throttled(c) || busy(c) => Step::Retry,
        c => Step::End(Outcome::Other, Some(c.to_owned())),
    }
}

/// A Helix ban or unban that failed: its status and message.
pub fn helix_step(status: u16, message: &str, unban: bool) -> Step {
    let m = message.to_ascii_lowercase();
    match status {
        429 | 409 => Step::Retry,
        s if s >= 500 => Step::Retry,
        400 if !unban && m.contains("already banned") => Step::End(Outcome::Already, None),
        400 if unban && (m.contains("not banned") || m.contains("isn't banned")) => Step::End(Outcome::Already, None),
        400 if ["broadcaster", "moderator", "may not be banned", "cannot be banned", "staff", "yourself"]
            .iter()
            .any(|s| m.contains(s)) =>
        {
            Step::End(Outcome::Protected, None)
        }
        404 => Step::End(Outcome::Gone, None),
        400 if m.contains("not found") || m.contains("does not exist") || m.contains("invalid user") => {
            Step::End(Outcome::Gone, None)
        }
        s => Step::End(Outcome::Other, Some(format!("{s}"))),
    }
}

/// Sends of one login (or one batch) at most: the first and four retries.
pub const TRIES: u32 = 5;
/// The first wait after a throttled answer; it doubles each time.
const FIRST_WAIT_MS: i64 = 2_000;
/// The longest single wait.
const MAX_WAIT_MS: i64 = 30_000;

/// Wait before retry number `n` (1 for the first retry). `hint_ms` is Twitch's own reset
/// time when it sent one (Helix `Ratelimit-Reset`); the longer of the two wins, capped.
pub fn backoff_ms(n: u32, hint_ms: Option<i64>) -> i64 {
    let own = FIRST_WAIT_MS.saturating_mul(1 << n.saturating_sub(1).min(8)).min(MAX_WAIT_MS);
    hint_ms.map_or(own, |h| h.clamp(own, 61_000))
}

/// GQL batches: logins per request and the gap between requests. Spec 1.55: up to 30 in a
/// request, one request a second, always (V107-R21, user 2026/10/21: no speed-up).
pub const GQL_BATCH: usize = 30;
pub const GQL_GAP_MS: i64 = 1_000;

/// V107-R21 Most logins one run takes (50 lookups and ~3 minutes of GQL bans).
pub const MAX_LOGINS: usize = 5_000;
pub const TOO_MANY: &str = "Up to 5.000 names at once.";

/// Refuses a run over [`MAX_LOGINS`].
pub fn check_size(n: usize) -> Result<(), &'static str> {
    if n > MAX_LOGINS { Err(TOO_MANY) } else { Ok(()) }
}
/// The tally behind the mass-action panel's line.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Tally {
    pub done: u32,
    pub already: u32,
    pub gone: u32,
    pub protected: u32,
    pub throttled: u32,
    pub other: u32,
    /// The codes behind `other`, each once, at most 5.
    pub codes: Vec<String>,
    /// Sends that were retries (throttled or busy).
    pub retries: u32,
}

impl Tally {
    pub fn add(&mut self, o: Outcome, code: Option<&str>) {
        match o {
            Outcome::Done => self.done += 1,
            Outcome::Already => self.already += 1,
            Outcome::Gone => self.gone += 1,
            Outcome::Protected => self.protected += 1,
            Outcome::Throttled => self.throttled += 1,
            Outcome::Other => {
                self.other += 1;
                if let Some(c) = code
                    && self.codes.len() < 5
                    && !self.codes.iter().any(|k| k == c)
                {
                    self.codes.push(c.to_owned());
                }
            }
        }
    }

    /// Logins handled so far.
    pub fn handled(&self) -> u32 {
        self.done + self.already + self.gone + self.protected + self.throttled + self.other
    }

    /// Everything that didn't go through.
    pub fn refused(&self) -> u32 {
        self.handled() - self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gql_answers() {
        assert_eq!(gql_step(None, false, false), Step::End(Outcome::Done, None));
        assert_eq!(gql_step(Some("TARGET_IS_MOD"), false, false), Step::End(Outcome::Protected, None));
        assert_eq!(gql_step(Some("TARGET_IS_BROADCASTER"), true, false), Step::End(Outcome::Protected, None));
        assert_eq!(gql_step(Some("TARGET_NOT_FOUND"), false, false), Step::End(Outcome::Gone, None));
        // A ban over a ban or timeout: the single-ban path looks it up.
        assert_eq!(gql_step(Some("TARGET_ALREADY_BANNED"), false, false), Step::Restack);
        assert_eq!(gql_step(Some("TARGET_NOT_BANNED"), false, true), Step::End(Outcome::Already, None));
        assert_eq!(
            gql_step(Some("DURATION_INVALID"), true, false),
            Step::End(Outcome::Other, Some("DURATION_INVALID".into()))
        );
    }

    #[test]
    fn gql_throttle_retries() {
        for c in ["RATE_LIMITED", "rate limit exceeded", "Too Many Requests", "service timeout", "THROTTLED"] {
            assert_eq!(gql_step(Some(c), false, false), Step::Retry, "{c}");
        }
    }

    #[test]
    fn helix_answers() {
        assert_eq!(helix_step(429, "Too Many Requests", false), Step::Retry);
        assert_eq!(helix_step(409, "", false), Step::Retry);
        assert_eq!(helix_step(503, "", false), Step::Retry);
        assert_eq!(
            helix_step(400, "The user specified in the user_id field is already banned.", false),
            Step::End(Outcome::Already, None)
        );
        assert_eq!(
            helix_step(400, "The user specified in the user_id field is not banned.", true),
            Step::End(Outcome::Already, None)
        );
        assert_eq!(
            helix_step(400, "The user specified in the user_id field may not be banned.", false),
            Step::End(Outcome::Protected, None)
        );
        assert_eq!(helix_step(400, "The broadcaster may not ban themselves", false), Step::End(Outcome::Protected, None));
        assert_eq!(helix_step(403, "forbidden", false), Step::End(Outcome::Other, Some("403".into())));
    }

    #[test]
    fn backoff_doubles_and_honors_reset() {
        assert_eq!(backoff_ms(1, None), 2_000);
        assert_eq!(backoff_ms(2, None), 4_000);
        assert_eq!(backoff_ms(4, None), 16_000);
        assert_eq!(backoff_ms(9, None), MAX_WAIT_MS);
        // Twitch's reset wins when it's later, never past a minute.
        assert_eq!(backoff_ms(1, Some(12_000)), 12_000);
        assert_eq!(backoff_ms(3, Some(100)), 8_000);
        assert_eq!(backoff_ms(1, Some(600_000)), 61_000);
    }

    #[test]
    fn v107_r21_caps_a_run() {
        assert_eq!(check_size(MAX_LOGINS), Ok(()));
        assert_eq!(check_size(MAX_LOGINS + 1), Err(TOO_MANY));
        assert_eq!(TOO_MANY, "Up to 5.000 names at once.");
    }

    #[test]
    fn tally_counts_and_keeps_codes_once() {
        let mut t = Tally::default();
        t.add(Outcome::Done, None);
        t.add(Outcome::Gone, None);
        t.add(Outcome::Other, Some("X"));
        t.add(Outcome::Other, Some("X"));
        t.add(Outcome::Other, Some("Y"));
        assert_eq!(t.handled(), 5);
        assert_eq!(t.refused(), 4);
        assert_eq!(t.codes, vec!["X", "Y"]);
    }
}
