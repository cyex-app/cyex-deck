//! Watch time for the slots (1.30, 1.33): every 20 s, spread over the chosen channels,
//! each gets a look at its stream and a `minute-watched` (`cyex_twitch::spade`), like
//! the reference script. Slots left by streams watched in the app (W1) aren't used.

use std::collections::HashSet;
use std::time::Duration;

use cyex_miner::slots::{self, Candidate, SETTLE_MS, SLOTS};
use cyex_miner::stats::{Kind as StatKind, Record};
use cyex_twitch::gql::{self, Player, TV_USER_AGENT};
use cyex_twitch::hermes::Kind;
use cyex_twitch::spade;
use tauri::AppHandle;
use tokio::time::{Instant, sleep, sleep_until};

use super::{gql_auth, http, lock, now_ms};

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

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let picked = pick(&app);
            if picked.is_empty() {
                sleep(CYCLE).await;
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

/// Picks the slots for this round and follows their Hermes topics.
fn pick(app: &AppHandle) -> Vec<String> {
    let s = super::settings(app);
    let now = now_ms();
    let (picked, changed) = {
        let mut st = lock(app);
        let picked = if st.account.is_none() {
            Vec::new()
        } else {
            let watching: HashSet<&str> = st.watching.iter().map(String::as_str).collect();
            let free = SLOTS.saturating_sub(watching.len());
            let list: Vec<Candidate> = st
                .channels
                .iter()
                .filter(|c| !watching.contains(c.id.as_str()))
                .map(|c| Candidate {
                    id: c.id.clone(),
                    ready: c.stream.is_some() && c.live_since > 0 && now - c.live_since >= SETTLE_MS,
                    streak_due: c.streak.due(now),
                    drops: s.claim_drops && c.drops,
                    multiplier: c.multiplier,
                    balance: c.balance.unwrap_or(0),
                })
                .collect();
            let first: Vec<String> = s.first.iter().map(|c| c.id.clone()).collect();
            slots::pick(&list, &first, &s.order, free)
        };
        let changed = picked != st.slots;
        st.slots = picked.clone();
        (picked, changed)
    };
    if changed {
        topics(app);
        super::emit_status(app);
    }
    picked
}

/// Follows the Hermes topics of the slots and of streams watched in the app.
pub(super) fn topics(app: &AppHandle) {
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
        (add, remove)
    };
    for id in add {
        for kind in CHANNEL_KINDS {
            crate::hermes::add(app, kind, &id);
        }
    }
    for id in remove {
        for kind in CHANNEL_KINDS {
            crate::hermes::remove(app, kind, &id);
        }
    }
}

/// One minute for one channel. True when Twitch asked to slow down.
async fn watch_one(app: &AppHandle, id: &str, spade_url: &str) -> bool {
    let Some((account, token, client_id)) = super::signed_in(app) else {
        return false;
    };
    let now = now_ms();
    let found = {
        let st = lock(app);
        st.channels.iter().find(|c| c.id == id).and_then(|c| {
            let stream = c.stream.clone()?;
            let cached = c
                .token
                .as_ref()
                .filter(|t| now - t.1 < TOKEN_AGE)
                .map(|t| t.0.clone());
            Some((c.login.clone(), stream, cached))
        })
    };
    let Some((login, stream, cached)) = found else {
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
                    return e.is_rate_limited();
                }
            }
        }
    };
    if let Err(e) = spade::touch(&http, &login, &playback, TV_USER_AGENT).await {
        eprintln!("miner: couldn't reach {login}'s stream: {e}");
        set_token(app, id, None);
        return e.is_rate_limited();
    }
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
            false
        }
        Ok(status) => {
            eprintln!("miner: spade didn't take a minute for {login} ({status})");
            status == 429
        }
        Err(e) => {
            eprintln!("miner: spade: {e}");
            e.is_rate_limited()
        }
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
