//! V109-F79 What the app knows of an account's standing in a channel, kept per (account,
//! channel id) while the app runs: the last read, IRC's own word, the unban request this
//! app sent and an unacknowledged warning. Every chat box of that pair (a remount after a
//! tab switch, a pop-out, the chat closed and opened again) starts from it, so the lock
//! never waits for a fresh read or drops when one fails.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

use cyex_chat::model::Restriction;
use cyex_twitch::gql::modlogs::{BanStatus, PastUnban};
use cyex_twitch::gql::standing::{Standing, Warning};
use serde::Serialize;
use ts_rs::TS;

/// Same as `READ_LAG_MS` in `ui/src/features/chat/chatLock.ts`: a read that started this
/// soon after IRC's word may not know of it yet.
pub const READ_LAG_MS: i64 = 3_000;

/// What IRC said about the account here, and when (ms).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "SaidAt")]
pub struct Said {
    pub how: Restriction,
    #[ts(type = "number")]
    pub at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "KnownStanding")]
pub struct Known {
    pub standing: Option<Standing>,
    /// When that read started (ms), 0 before the first good one.
    #[ts(type = "number")]
    pub read_at: i64,
    pub said: Option<Said>,
    /// The request this app sent, for when Twitch won't show the requester their own.
    pub sent: Option<PastUnban>,
    pub warning: Option<Warning>,
    /// A read showed no ban or timeout where one was known: the next read decides
    /// (when the doubting read started, ms; 0 = none).
    #[ts(type = "number")]
    pub doubt: i64,
}

type Key = (String, String);
static KNOWN: LazyLock<Mutex<HashMap<Key, Known>>> = LazyLock::new(Mutex::default);

fn all() -> MutexGuard<'static, HashMap<Key, Known>> {
    KNOWN.lock().unwrap_or_else(|p| p.into_inner())
}

fn key(account: &str, channel_id: &str) -> Key {
    (account.to_owned(), channel_id.to_owned())
}

pub fn get(account: &str, channel_id: &str) -> Known {
    all().get(&key(account, channel_id)).cloned().unwrap_or_default()
}

/// Changes the pair's entry and hands back what it is now.
pub fn update(account: &str, channel_id: &str, f: impl FnOnce(&mut Known)) -> Known {
    let mut m = all();
    let k = m.entry(key(account, channel_id)).or_default();
    f(k);
    k.clone()
}

fn ms(s: Option<&str>) -> Option<i64> {
    let t = time::OffsetDateTime::parse(s?, &time::format_description::well_known::Rfc3339).ok()?;
    i64::try_from(t.unix_timestamp_nanos() / 1_000_000).ok()
}

/// A ban, or a timeout still running at `now`.
fn in_force(b: &BanStatus, now: i64) -> bool {
    b.until.is_none() || ms(b.until.as_deref()).is_none_or(|u| u > now + 1_000)
}

/// IRC's word still counts (no read started late enough to know of it).
fn said_counts(s: &Said, read_at: i64) -> bool {
    s.at + READ_LAG_MS > read_at
}

/// A ban or timeout known at `now`: from the last read, or from IRC's word no read has
/// answered yet.
fn held(k: &Known, now: i64) -> bool {
    let read = k.standing.as_ref().and_then(|s| s.ban.as_ref()).is_some_and(|b| in_force(b, now));
    let said = k.said.is_some_and(|s| {
        said_counts(&s, k.read_at)
            && match s.how {
                Restriction::Banned => true,
                Restriction::TimedOut { secs } => s.at + secs as i64 * 1_000 > now,
                Restriction::Warned => false,
            }
    });
    read || said
}

/// What came of a read.
#[derive(Debug, PartialEq, Eq)]
pub enum Took {
    /// Kept as the standing now.
    Kept,
    /// The ban or timeout ended (kept too): a chat Twitch kept from the account can join again.
    Ended,
    /// No ban where one was known: held back until the next read agrees.
    Doubted,
    /// An older read that finished late.
    Stale,
}

/// Folds a good read that started at `started` into `k`.
pub fn take_read(k: &mut Known, mut read: Standing, started: i64) -> Took {
    if started < k.read_at {
        return Took::Stale;
    }
    let was = held(k, started);
    let free = read.ban.as_ref().is_none_or(|b| !in_force(b, started));
    if free && was {
        // V109-F79 One read saying "free" while a ban is known isn't enough: ask once more.
        if k.doubt == 0 || started <= k.doubt {
            k.doubt = k.doubt.max(started);
            return Took::Doubted;
        }
    }
    // A partial read (the channel part refused) keeps what the last one knew of the same ban.
    if let (Some(new), Some(old)) = (read.ban.as_mut(), k.standing.as_ref()) {
        let old_ban = old.ban.as_ref().filter(|b| b.until.is_none() == new.until.is_none());
        if let Some(ob) = old_ban {
            if new.since.is_none() {
                new.since.clone_from(&ob.since);
            }
            if read.unban.is_none() {
                read.unban.clone_from(&old.unban);
            }
            if read.can_request.is_none() {
                read.can_request = old.can_request;
            }
            if read.request.is_none() && new.until.is_none() {
                read.request.clone_from(&old.request);
            }
        }
    }
    if free {
        k.sent = None;
    }
    k.standing = Some(read);
    k.read_at = started;
    k.doubt = 0;
    if free && was { Took::Ended } else { Took::Kept }
}

/// IRC told of our own ban, timeout or warning.
pub fn take_said(k: &mut Known, how: Restriction, at: i64) {
    k.said = Some(Said { how, at });
    if how != Restriction::Warned {
        k.doubt = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    fn iso(ms: i64) -> String {
        let t = time::OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000).unwrap();
        t.format(&time::format_description::well_known::Rfc3339).unwrap()
    }

    fn banned(since: Option<i64>) -> Standing {
        Standing {
            ban: Some(BanStatus { until: None, by: None, reason: "spam".into(), since: since.map(iso) }),
            unban: Some(cyex_twitch::gql::standing::UnbanRules { enabled: true, cooldown_minutes: 180 }),
            can_request: Some(false),
            request: None,
        }
    }

    #[test]
    fn a_ban_survives_one_free_read() {
        let mut k = Known::default();
        assert_eq!(take_read(&mut k, banned(Some(NOW - 60_000)), NOW), Took::Kept);
        assert_eq!(take_read(&mut k, Standing::default(), NOW + 1_000), Took::Doubted);
        assert!(k.standing.as_ref().unwrap().ban.is_some(), "still banned");
        assert_eq!(k.read_at, NOW, "the doubted read doesn't move the clock");
        // The next read agrees: the ban is over.
        assert_eq!(take_read(&mut k, Standing::default(), NOW + 6_000), Took::Ended);
        assert!(k.standing.as_ref().unwrap().ban.is_none());
        assert_eq!(k.doubt, 0);
    }

    #[test]
    fn a_ban_read_after_a_doubt_clears_it() {
        let mut k = Known::default();
        take_read(&mut k, banned(Some(NOW)), NOW);
        take_read(&mut k, Standing::default(), NOW + 1_000);
        assert_eq!(take_read(&mut k, banned(Some(NOW)), NOW + 6_000), Took::Kept);
        assert_eq!(k.doubt, 0);
        assert!(k.standing.unwrap().ban.is_some());
    }

    #[test]
    fn irc_word_is_held_against_one_free_read() {
        // The NOTICE on a send (msg_banned) or our own CLEARCHAT, then a read that says free.
        let mut k = Known::default();
        take_read(&mut k, Standing::default(), NOW - 60_000);
        take_said(&mut k, Restriction::Banned, NOW);
        assert_eq!(take_read(&mut k, Standing::default(), NOW + 4_000), Took::Doubted);
        assert!(said_counts(&k.said.unwrap(), k.read_at), "IRC still counts");
        assert_eq!(take_read(&mut k, Standing::default(), NOW + 9_000), Took::Ended);
        assert!(!said_counts(&k.said.unwrap(), k.read_at));
    }

    #[test]
    fn an_ended_timeout_is_no_doubt() {
        let mut k = Known::default();
        let t = Standing { ban: Some(BanStatus { until: Some(iso(NOW + 60_000)), by: None, reason: String::new(), since: None }), ..Default::default() };
        take_read(&mut k, t, NOW);
        assert_eq!(take_read(&mut k, Standing::default(), NOW + 61_000), Took::Kept, "it ran out by itself");
    }

    #[test]
    fn a_partial_read_keeps_the_wait() {
        // V109-F79 The channel part refused once: the unban wait isn't lost.
        let mut k = Known::default();
        take_read(&mut k, banned(Some(NOW - 60_000)), NOW);
        let partial = Standing { ban: Some(BanStatus { until: None, by: None, reason: "spam".into(), since: None }), ..Default::default() };
        assert_eq!(take_read(&mut k, partial, NOW + 1_000), Took::Kept);
        let s = k.standing.unwrap();
        assert_eq!(s.ban.unwrap().since, Some(iso(NOW - 60_000)));
        assert_eq!(s.unban.unwrap().cooldown_minutes, 180);
        assert_eq!(s.can_request, Some(false));
    }

    #[test]
    fn a_late_read_is_dropped() {
        let mut k = Known::default();
        take_read(&mut k, banned(None), NOW);
        assert_eq!(take_read(&mut k, Standing::default(), NOW - 1), Took::Stale);
    }

    #[test]
    fn the_store_is_per_account_and_channel() {
        update("a1", "c1", |k| take_said(k, Restriction::Banned, NOW));
        assert!(get("a1", "c1").said.is_some());
        assert!(get("a1", "c2").said.is_none());
        assert!(get("a2", "c1").said.is_none());
    }
}
