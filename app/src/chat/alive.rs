//! V111-08 No silent dead rooms. Twitch stops sending a channel to an account banned there
//! while it stays joined (no PART, no NOTICE), and a ban that ends sends nothing on IRC. Every
//! few seconds the app looks for an account's joined room that no longer hears its channel:
//! our own line came back only from the writer (`Room::stalled`), or another room of the
//! channel (anonymous, another account, a banned one's mirror) keeps hearing people while
//! this one has heard nobody for minutes. Those rejoin for real (`session::force_rejoin`).
//! A banned room also tries its JOIN again now and then, in case the ban ended unseen.

use std::time::Duration;

use cyex_chat::model::ConnStatus;
use tauri::{AppHandle, Manager};

use super::{Chat, Identity, Inner, RoomKey, now_ms, session};

/// How often rooms are looked at.
const EVERY: Duration = Duration::from_secs(10);
/// An automatic rejoin of the same room waits at least this long after the last one.
const AUTO_GAP_MS: i64 = 60_000;
/// Another room heard someone this recently: the channel is talking.
const ACTIVE_MS: i64 = 60_000;
/// ...and this room has heard nobody for this much longer: it's dead.
const BEHIND_MS: i64 = 180_000;
/// A rejoin for silence alone waits this long after the last one.
const SILENT_GAP_MS: i64 = 300_000;
/// A banned room asks to join again this often (a standing read or the account's topic
/// usually sees the unban sooner).
const BANNED_RETRY_MS: i64 = 15 * 60_000;

/// An automatic rejoin may go out.
pub fn may_force(forced_at: i64, now: i64) -> bool {
    now - forced_at >= AUTO_GAP_MS
}

/// One room as the check sees it.
#[derive(Clone, Debug)]
struct Seen {
    key: RoomKey,
    status: ConnStatus,
    stalled: bool,
    /// Last heard the channel (someone's line, or getting in).
    heard: Option<i64>,
    /// Last heard someone else, joined or read anonymously.
    others: Option<i64>,
    forced_at: i64,
}

#[derive(Debug, PartialEq, Eq)]
enum Act {
    /// Rejoin for real.
    Dead(RoomKey, &'static str),
    /// A banned room tries its JOIN again.
    Retry(RoomKey),
    /// First seen banned: its clock starts.
    Stamp(RoomKey),
}

fn plan(rooms: &[Seen], now: i64) -> Vec<Act> {
    let mut acts = Vec::new();
    for r in rooms {
        if !matches!(r.key.0, Identity::Account(_)) {
            continue;
        }
        match r.status {
            ConnStatus::Joined if r.stalled && may_force(r.forced_at, now) => {
                acts.push(Act::Dead(r.key.clone(), "our line never came back"));
            }
            ConnStatus::Joined => {
                let Some(mine) = r.heard else { continue };
                let best = rooms.iter().filter(|o| o.key != r.key && o.key.1 == r.key.1).filter_map(|o| o.others).max();
                if let Some(best) = best
                    && now - best <= ACTIVE_MS
                    && best - mine >= BEHIND_MS
                    && now - r.forced_at >= SILENT_GAP_MS
                {
                    acts.push(Act::Dead(r.key.clone(), "other chats hear it, this one doesn't"));
                }
            }
            ConnStatus::Banned if r.forced_at == 0 => acts.push(Act::Stamp(r.key.clone())),
            ConnStatus::Banned if now - r.forced_at >= BANNED_RETRY_MS => acts.push(Act::Retry(r.key.clone())),
            _ => {}
        }
    }
    acts
}

fn seen(inner: &Inner, now: i64) -> Vec<Seen> {
    inner
        .rooms
        .iter()
        .map(|(key, e)| Seen {
            key: key.clone(),
            status: e.room.status,
            stalled: e.room.stalled(now),
            heard: e.room.heard(),
            others: e.room.heard_others(),
            forced_at: e.forced_at,
        })
        .collect()
}

pub fn init(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            check(&app);
        }
    });
}

fn check(app: &AppHandle) {
    let now = now_ms();
    let acts = {
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        let acts = plan(&seen(&inner, now), now);
        for a in &acts {
            if let Act::Stamp(k) = a
                && let Some(e) = inner.rooms.get_mut(k)
            {
                e.forced_at = now;
            }
        }
        acts
    };
    for a in acts {
        match a {
            Act::Dead(key, why) => session::auto_rejoin(app, &key, why),
            Act::Retry(key) => {
                session::rejoin(app, &key, true);
            }
            Act::Stamp(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 10_000_000;

    fn acc(login: &str) -> RoomKey {
        (Identity::Account("1".into()), login.into())
    }

    fn room(key: RoomKey, status: ConnStatus) -> Seen {
        Seen { key, status, stalled: false, heard: Some(NOW - 1_000), others: Some(NOW - 1_000), forced_at: 0 }
    }

    #[test]
    fn v111_08_our_line_only_from_the_writer_rejoins_once_a_minute() {
        let mut r = room(acc("a"), ConnStatus::Joined);
        r.stalled = true;
        assert!(matches!(plan(&[r.clone()], NOW)[..], [Act::Dead(..)]));
        r.forced_at = NOW - 30_000;
        assert!(plan(&[r], NOW).is_empty(), "just rejoined");
    }

    #[test]
    fn v111_08_a_room_deaf_to_a_talking_channel_rejoins() {
        // Banned at 00:24, still "joined"; the anonymous chat of the channel hears people.
        let mut dead = room(acc("satxris"), ConnStatus::Joined);
        dead.heard = Some(NOW - 16 * 3_600_000);
        dead.others = dead.heard;
        let anon = room((Identity::Anon, "satxris".into()), ConnStatus::Joined);
        assert_eq!(plan(&[dead.clone(), anon.clone()], NOW), [Act::Dead(acc("satxris"), "other chats hear it, this one doesn't")]);
        // A quiet channel (nobody heard anywhere lately) is just quiet.
        let mut quiet = anon.clone();
        quiet.others = Some(NOW - 10 * 60_000);
        assert!(plan(&[dead.clone(), quiet], NOW).is_empty());
        // Another channel's chat says nothing about this one.
        let other = room((Identity::Anon, "kuni".into()), ConnStatus::Joined);
        assert!(plan(&[dead.clone(), other], NOW).is_empty());
        // Not again within five minutes.
        dead.forced_at = NOW - 120_000;
        assert!(plan(&[dead, anon], NOW).is_empty());
    }

    #[test]
    fn v111_08_a_banned_room_tries_again_now_and_then() {
        let mut b = room(acc("a"), ConnStatus::Banned);
        b.forced_at = 0;
        assert_eq!(plan(&[b.clone()], NOW), [Act::Stamp(acc("a"))], "its clock starts first");
        b.forced_at = NOW - 60_000;
        assert!(plan(&[b.clone()], NOW).is_empty());
        b.forced_at = NOW - BANNED_RETRY_MS;
        assert_eq!(plan(&[b], NOW), [Act::Retry(acc("a"))]);
    }

    #[test]
    fn v111_08_anonymous_and_stuck_rooms_are_left_alone() {
        let mut anon = room((Identity::Anon, "a".into()), ConnStatus::Joined);
        anon.stalled = true;
        assert!(plan(&[anon], NOW).is_empty());
        for s in [ConnStatus::Connecting, ConnStatus::Reconnecting, ConnStatus::NoAnswer, ConnStatus::Refused] {
            let mut r = room(acc("a"), s);
            r.stalled = true;
            assert!(plan(&[r], NOW).is_empty(), "{s:?}");
        }
    }
}
