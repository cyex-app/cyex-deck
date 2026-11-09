//! Watch time for the slots (1.30, 1.33): every 20 s, spread over the chosen channels,
//! each gets a look at its stream and a `minute-watched` (`cyex_twitch::spade`), like
//! the reference script. Slots left by streams watched in the app (W1) aren't used.
//! V110-U68 A live channel that can earn a wanted badge takes the watch alone (`plan`),
//! every segment of its stream is looked at (also for channels with drops), a minute spade
//! turns down reads spade's address again, and watch streaks are confirmed from Twitch's
//! count once a minute while one is due. Hidden with nothing to do, the loop parks.

use std::collections::HashSet;
use std::time::Duration;

use cyex_miner::plan::{self, Plan};
use cyex_miner::slots::{Candidate, SETTLE_MS, SLOTS, Streak};
use cyex_miner::stats::{Kind as StatKind, Record};
use cyex_twitch::gql::{self, Player, TV_USER_AGENT};
use cyex_twitch::hermes::Kind;
use cyex_twitch::spade;
use tauri::{AppHandle, Manager};
use tokio::time::{Instant, sleep, sleep_until};

use super::log::{self, LogKind};
use super::{Chan, gql_auth, http, lock, now_ms};

const CYCLE: Duration = Duration::from_secs(20);
/// Playback tokens last 20 minutes.
const TOKEN_AGE: i64 = 10 * 60_000;
/// Watched minutes go to the stats in chunks of at least this.
const LOG_SECS: u64 = 5 * 60;
/// Time between two credited minutes that still counts as watching.
const GAP_SECS: i64 = 30;
/// Pause after Twitch says too many requests (the reference script has none).
const BACKOFF: Duration = Duration::from_secs(60);
/// What the miner follows on each watched channel.
const CHANNEL_KINDS: [Kind; 5] = [
    Kind::Playback,
    Kind::Raid,
    Kind::Moments,
    Kind::Predictions,
    Kind::Points,
];
/// V110-U68 What the miner follows on a helper channel: no raids, no bets on strangers.
const HELPER_KINDS: [Kind; 3] = [Kind::Playback, Kind::Moments, Kind::Points];
/// V110-U68 Twitch's streak count is read this often while a streak is due.
const STREAK_CHECK: i64 = 60_000;
/// V110-U68 With the count to confirm it, a streak stays due this long (Twitch grants it
/// within minutes; past this the stream likely gives none).
const STREAK_CONFIRM_SECS: u64 = 20 * 60;

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let idle_wake = app.state::<super::Miner>().idle_wake.clone();
        loop {
            let picked = pick(&app);
            if picked.is_empty() {
                if super::idle::idle(&app) {
                    // Hidden with nothing to do: wait for a window or a stream to come up.
                    tokio::select! {
                        () = idle_wake.notified() => {}
                        () = sleep(cyex_miner::cadence::POLL_HIDDEN) => {}
                    }
                } else {
                    sleep(CYCLE).await;
                }
                continue;
            }
            let url = super::spade_url(&app).await;
            let each = CYCLE / picked.len() as u32;
            for id in picked {
                let next = Instant::now() + each;
                if watch_one(&app, &id, &url).await {
                    sleep(BACKOFF).await;
                    break;
                }
                sleep_until(next).await;
            }
        }
    });
}

/// Logs why a minute failed: Twitch's brake, else `text`. Returns `slow`.
fn slow_or(app: &AppHandle, id: &str, slow: bool, text: &str) -> bool {
    let name = super::claims::label(app, id);
    let text = if slow { "Twitch asked to slow down." } else { text };
    log::add(app, LogKind::Error, Some(&name), text);
    slow
}

/// Who joined and who left between two rounds of slots.
pub(super) fn joined_left(old: &[String], new: &[String]) -> (Vec<String>, Vec<String>) {
    let joined = new.iter().filter(|i| !old.contains(i)).cloned().collect();
    let left = old.iter().filter(|i| !new.contains(i)).cloned().collect();
    (joined, left)
}

/// V110-U68 A streak is due: with Twitch's count to confirm it, until confirmed (or
/// [`STREAK_CONFIRM_SECS`]); without, the 7-minute guess.
pub(super) fn streak_due(streak: &Streak, confirmable: bool, now: i64) -> bool {
    if !confirmable {
        return streak.due(now);
    }
    let fresh = Streak {
        watched_secs: 0,
        ..streak.clone()
    };
    fresh.due(now) && streak.watched_secs < STREAK_CONFIRM_SECS
}

/// V110-U68 Twitch's streak count read again: the count at the stream's start, and whether
/// this stream's streak is in. `fresh`: the stream was barely watched yet, so a first count
/// is the one from before it.
pub(super) fn streak_step(start: Option<u32>, count: Option<u32>, fresh: bool) -> (Option<u32>, bool) {
    match (start, count) {
        (None, Some(_)) if fresh => (count, false),
        (Some(old), Some(new)) if new < old => (count, false),
        (Some(old), Some(new)) if new > old => (start, true),
        _ => (start, false),
    }
}

fn candidate(c: &Chan, claim_drops: bool, now: i64) -> Candidate {
    Candidate {
        id: c.id.clone(),
        ready: c.stream.is_some() && c.live_since > 0 && now - c.live_since >= SETTLE_MS,
        streak_due: streak_due(&c.streak, c.streak_start.is_some(), now),
        drops: claim_drops && c.drops,
        multiplier: c.multiplier,
        balance: c.balance.unwrap_or(0),
    }
}

/// Picks the slots for this round and follows their Hermes topics.
fn pick(app: &AppHandle) -> Vec<String> {
    let s = super::settings(app);
    let now = now_ms();
    let (picked, changed, old, news) = {
        let mut st = lock(app);
        let (plan, focus) = if st.account.is_none() {
            (Plan::default(), None)
        } else {
            let targets = super::badges::targets(&st, &s);
            let watching: HashSet<&str> = st.watching.iter().map(String::as_str).collect();
            let free = SLOTS.saturating_sub(watching.len());
            let list: Vec<Candidate> = st
                .channels
                .iter()
                .filter(|c| !watching.contains(c.id.as_str()))
                .map(|c| candidate(c, s.claim_drops, now))
                .collect();
            let first: Vec<String> = s.first.iter().map(|c| c.id.clone()).collect();
            match targets.playing {
                // The stream playing here earns the badge (W1 sends its minutes): nothing else.
                Some(f) => (Plan::default(), Some(f)),
                None => {
                    let p = plan::plan(&list, &targets.ids, &first, &s.order, free);
                    let f = p
                        .focus
                        .as_ref()
                        .and_then(|ch| super::badges::campaign_of(&st, &s, ch).map(|camp| (ch.clone(), camp)));
                    (p, f)
                }
            }
        };
        let picked = plan.slots;
        let changed = picked != st.slots;
        let old = std::mem::replace(&mut st.slots, picked.clone());
        let news = focus_news(&mut st, focus);
        (picked, changed, old, news)
    };
    let focus_changed = !news.is_empty();
    for (channel, text) in news {
        log::add(app, LogKind::Badge, channel.as_deref(), text);
    }
    if changed {
        let (joined, left) = joined_left(&old, &picked);
        for (ids, text) in [(joined, "Started mining."), (left, "Stopped mining.")] {
            for id in ids {
                let name = super::claims::label(app, &id);
                log::add(app, LogKind::Channel, Some(&name), text);
            }
        }
        topics(app);
    }
    if changed || focus_changed {
        super::emit_status(app);
    }
    // V110-F78 A badge focus that starts wakes the drops loop: it read the inventory before the
    // focus (busy false) and would otherwise wait 15 minutes, not 1, for the badge's progress.
    if focus_changed && let Some(m) = app.try_state::<super::Miner>() {
        m.idle_wake.notify_waiters();
    }
    picked
}

/// Sets the badge focus; log lines for what changed.
fn focus_news(st: &mut super::State, focus: Option<(String, String)>) -> Vec<(Option<String>, String)> {
    if st.focus == focus {
        return Vec::new();
    }
    let mut out = Vec::new();
    let old = std::mem::replace(&mut st.focus, focus.clone());
    st.focus_warned = false;
    match focus {
        Some((ch, camp)) => {
            let name = st
                .channels
                .iter()
                .find(|c| c.id == ch)
                .map_or_else(|| ch.clone(), |c| c.name.clone());
            let badge = super::badges::name_of(st, &camp);
            if st.watching.contains(&ch) {
                out.push((Some(name), format!("Earning the {} with the stream playing here.", cyex_miner::badges::phrase(&badge))));
            } else {
                out.push((Some(name.clone()), format!("Watching only {name} for the {}.", cyex_miner::badges::phrase(&badge))));
                if !st.watching.is_empty() && !st.focus_warned {
                    st.focus_warned = true;
                    out.push((None, "A stream plays here too: the badge gets the other slot.".to_owned()));
                }
            }
        }
        None => {
            if let Some((_, camp)) = old {
                let badge = super::badges::name_of(st, &camp);
                let open = cyex_miner::badges::open_campaigns(&st.badges, now_ms() / 1000).contains(&camp);
                if open {
                    out.push((None, format!("The {} waits: nothing live can earn it now.", cyex_miner::badges::phrase(&badge))));
                }
            }
        }
    }
    out
}

/// Follows the Hermes topics of the slots and of streams watched in the app.
pub(super) fn topics(app: &AppHandle) {
    // (id, kinds) to add and to remove; a channel keeps the kinds it was followed with.
    let (add, remove) = {
        let mut st = lock(app);
        let want: HashSet<String> = if st.account.is_some() {
            st.slots.iter().chain(st.watching.iter()).cloned().collect()
        } else {
            HashSet::new()
        };
        let add: Vec<String> = want.difference(&st.followed).cloned().collect();
        let remove: Vec<String> = st.followed.difference(&want).cloned().collect();
        st.followed = want;
        let add: Vec<(String, &'static [Kind])> = add
            .into_iter()
            .map(|id| {
                let helper = st.helpers.iter().any(|h| h.channel_id == id);
                if helper {
                    st.helper_topics.insert(id.clone());
                }
                (id, kinds(helper))
            })
            .collect();
        let remove: Vec<(String, &'static [Kind])> = remove
            .into_iter()
            .map(|id| {
                let helper = st.helper_topics.remove(&id);
                (id, kinds(helper))
            })
            .collect();
        (add, remove)
    };
    for (id, kinds) in add {
        for kind in kinds {
            crate::hermes::add(app, *kind, &id);
        }
    }
    for (id, kinds) in remove {
        for kind in kinds {
            crate::hermes::remove(app, *kind, &id);
        }
    }
    super::idle::topics(app);
}

fn kinds(helper: bool) -> &'static [Kind] {
    if helper { &HELPER_KINDS } else { &CHANNEL_KINDS }
}

/// One minute for one channel. True when Twitch asked to slow down.
async fn watch_one(app: &AppHandle, id: &str, spade_url: &str) -> bool {
    let Some((account, token, client_id)) = super::signed_in(app) else {
        return false;
    };
    let now = now_ms();
    let claim_drops = super::settings(app).claim_drops;
    let found = {
        let st = lock(app);
        let focus = st.focus.as_ref().is_some_and(|f| f.0 == id);
        st.channels.iter().find(|c| c.id == id).and_then(|c| {
            let stream = c.stream.clone()?;
            let cached = c
                .token
                .as_ref()
                .filter(|t| now - t.1 < TOKEN_AGE)
                .map(|t| t.0.clone());
            Some((c.login.clone(), stream, cached, focus || (claim_drops && c.drops)))
        })
    };
    let Some((login, stream, cached, all)) = found else {
        return false;
    };
    let http = http(app);
    let playback = match cached {
        Some(t) => t,
        None => {
            match gql::stream_token(&http, gql_auth(&token, &client_id), None, &login, Player::SITE).await {
                Ok(t) => {
                    set_token(app, id, Some((t.clone(), now)));
                    t
                }
                Err(e) => {
                    eprintln!("miner: no playback token for {login}: {e}");
                    super::badges::reached(app, id, false);
                    return slow_or(app, id, e.is_rate_limited(), "Couldn’t reach the stream.");
                }
            }
        }
    };
    // V110-U68 Every segment for a badge or drops: that is what moves their progress.
    if let Err(e) = spade::touch_segments(&http, &login, &playback, TV_USER_AGENT, all).await {
        eprintln!("miner: couldn't reach {login}'s stream: {e}");
        set_token(app, id, None);
        super::badges::reached(app, id, false);
        return slow_or(app, id, e.is_rate_limited(), "Couldn’t reach the stream.");
    }
    super::badges::reached(app, id, true);
    let minute = spade::Minute {
        channel_id: id,
        login: &login,
        broadcast_id: &stream.id,
        user_id: &account,
        game: stream.game.as_ref().map(|(i, n)| (i.as_str(), n.as_str())),
    };
    match spade::send(&http, spade_url, &spade::payload(&minute), TV_USER_AGENT).await {
        Ok(204) => {
            credit(app, &account, id);
            confirm_streak(app, &token, &client_id, id).await;
            false
        }
        Ok(status) => {
            eprintln!("miner: spade didn't take a minute for {login} ({status})");
            if status == 429 {
                slow_or(app, id, true, "");
            } else {
                // V110-U68 The address may have moved: read it again for the next minute.
                super::forget_spade(app);
            }
            status == 429
        }
        Err(e) => {
            eprintln!("miner: spade: {e}");
            e.is_rate_limited()
        }
    }
}

/// V110-U68 Once a minute while a streak is due on `id`, Twitch's streak count: when it
/// rose since the stream began, the streak is in (even if its points event was missed).
async fn confirm_streak(app: &AppHandle, token: &str, client_id: &str, id: &str) {
    let now = now_ms();
    let due = {
        let st = lock(app);
        st.channels.iter().find(|c| c.id == id).is_some_and(|c| {
            c.streak.missing
                && now - c.streak_checked_at >= STREAK_CHECK
                && streak_due(&c.streak, c.streak_start.is_some(), now)
        })
    };
    if !due {
        return;
    }
    let count = match gql::miner::watch_streak(&http(app), gql_auth(token, client_id), id).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("miner: streak count: {e}");
            None
        }
    };
    let earned = {
        let mut st = lock(app);
        let Some(c) = st.channels.iter_mut().find(|c| c.id == id) else {
            return;
        };
        c.streak_checked_at = now;
        let (start, earned) = streak_step(c.streak_start, count, c.streak.watched_secs <= 2 * 60);
        c.streak_start = start;
        if earned {
            c.streak.earned();
        }
        earned
    };
    if earned {
        let name = super::claims::label(app, id);
        log::add(app, LogKind::Points, Some(&name), "Watch streak earned.");
        super::emit_status(app);
    }
}

fn set_token(app: &AppHandle, id: &str, token: Option<(gql::AccessToken, i64)>) {
    let mut st = lock(app);
    if let Some(c) = st.channels.iter_mut().find(|c| c.id == id) {
        c.token = token;
    }
}

/// Counts the time since the last credited minute toward the streak and the stats.
fn credit(app: &AppHandle, account: &str, id: &str) {
    let now = now_ms();
    let minutes = {
        let mut st = lock(app);
        let Some(c) = st.channels.iter_mut().find(|c| c.id == id) else {
            return;
        };
        let secs = if c.watched_at > 0 {
            ((now - c.watched_at) / 1000).clamp(0, GAP_SECS) as u64
        } else {
            CYCLE.as_secs()
        };
        c.watched_at = now;
        c.streak.watched_secs += secs;
        c.unlogged_secs += secs;
        (c.unlogged_secs >= LOG_SECS).then(|| {
            let m = c.unlogged_secs / 60;
            c.unlogged_secs -= m * 60;
            m
        })
    };
    if let Some(m) = minutes {
        let rec = Record {
            m: m as u32,
            ..Record::new(now, StatKind::Watched, 0)
        };
        super::stats::record(app, account, id, &rec);
    }
}

#[cfg(test)]
mod tests {
    use super::{Streak, joined_left, streak_due, streak_step};

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn joined_and_left_between_rounds() {
        let (j, l) = joined_left(&v(&["a", "b"]), &v(&["b", "c"]));
        assert_eq!(j, v(&["c"]));
        assert_eq!(l, v(&["a"]));
        assert_eq!(joined_left(&v(&["a"]), &v(&["a"])), (vec![], vec![]));
    }

    #[test]
    fn v110_u68_streaks_are_confirmed_by_the_count() {
        // First read early in the stream: the count from before it.
        assert_eq!(streak_step(None, Some(4), true), (Some(4), false));
        // Read late without a start: nothing to compare with.
        assert_eq!(streak_step(None, Some(4), false), (None, false));
        assert_eq!(streak_step(Some(4), Some(4), false), (Some(4), false));
        assert_eq!(streak_step(Some(4), Some(5), false), (Some(4), true));
        // A missed stream reset it: count from the new value.
        assert_eq!(streak_step(Some(4), Some(1), false), (Some(1), false));
        assert_eq!(streak_step(Some(4), None, false), (Some(4), false));
    }

    #[test]
    fn v110_u68_a_confirmable_streak_stays_due_past_seven_minutes() {
        let s = Streak {
            watched_secs: 8 * 60,
            ..Streak::default()
        };
        assert!(!streak_due(&s, false, 1_000_000));
        assert!(streak_due(&s, true, 1_000_000));
        let long = Streak {
            watched_secs: 21 * 60,
            ..Streak::default()
        };
        assert!(!streak_due(&long, true, 1_000_000));
    }
}
