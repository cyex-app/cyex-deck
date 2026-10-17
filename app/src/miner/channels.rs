//! The miner's channels (1.31, W3): the two picked first (1.44), then your list or your
//! follows. Looked up once a minute (and when Hermes says a stream went up) for who's
//! live; each live one's points context is read on going live and every 30 minutes.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use cyex_miner::slots::SETTLE_MS;
use cyex_settings::schema::{MineSource, Miner as MinerSettings};
use cyex_twitch::gql::miner::{self as gm, MinerChannel};
use cyex_twitch::hermes::Kind;
use tauri::{AppHandle, Manager};

use super::log::{self, LogKind};
use super::{Chan, Miner, gql_auth, http, lock, now_ms};

const POLL: Duration = Duration::from_secs(60);
const CONTEXT_AGE: i64 = 30 * 60_000;
const DROPS_AGE: i64 = 15 * 60_000;
/// Drops are looked up for the first few live channels only.
const DROPS_CHECKED: usize = 20;
const FOLLOWS_AGE: Duration = Duration::from_secs(15 * 60);
const USER_KINDS: [Kind; 2] = [Kind::UserPoints, Kind::UserPredictions];

/// (id, login, name)
type Named = (String, String, String);

#[derive(Default)]
struct Follows {
    account: String,
    at: Option<Instant>,
    list: Vec<Named>,
}

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let wake = app.state::<Miner>().wake.clone();
        let mut follows = Follows::default();
        loop {
            refresh(&app, &mut follows).await;
            tokio::select! {
                () = tokio::time::sleep(POLL) => {}
                () = wake.notified() => {}
            }
        }
    });
}

async fn refresh(app: &AppHandle, follows: &mut Follows) {
    let s = super::settings(app);
    let picked = if s.enabled {
        pick_account(app, &s)
    } else {
        Err(String::new())
    };
    let account = picked.as_ref().ok().cloned();
    set_account(app, account.clone(), picked.err().filter(|e| !e.is_empty()));

    if s.source == MineSource::Follows {
        match &account {
            Some(a) => load_follows(app, a, follows).await,
            None => follows.list.clear(),
        }
    }
    let wanted = wanted(&s, &follows.list);
    let Some(account) = account else {
        set_channels(app, &wanted, None);
        super::emit_status(app);
        return;
    };
    let ids: Vec<String> = wanted.iter().map(|w| w.0.clone()).collect();
    let found = match gm::streams(&http(app), &ids).await {
        Ok(found) => Some(found),
        Err(e) => {
            eprintln!("miner: live check failed: {e}");
            log::add(app, LogKind::Error, None, "Couldn’t refresh the channel list.");
            None
        }
    };
    set_channels(app, &wanted, found.as_deref());
    contexts(app, &account, &s).await;
    super::emit_status(app);
    super::tabs::sync_presence(app);
}

/// The account set in the miner if it has a GQL sign-in, else the default account, else
/// any account that has one.
fn pick_account(app: &AppHandle, s: &MinerSettings) -> Result<String, String> {
    let accounts = crate::auth::accounts(app);
    let has = |id: &str| crate::auth::gql_token(app, id).is_some();
    if let Some(id) = &s.account {
        if has(id) {
            return Ok(id.clone());
        }
        if let Some(a) = accounts.list.iter().find(|a| a.user_id == *id) {
            return Err(format!(
                "{} needs a GQL sign-in to mine (Settings → Accounts).",
                a.display_name
            ));
        }
    }
    if let Some(id) = accounts.default_id.as_deref().filter(|id| has(id)) {
        return Ok(id.to_owned());
    }
    if let Some(a) = accounts.list.iter().find(|a| has(&a.user_id)) {
        return Ok(a.user_id.clone());
    }
    Err(if accounts.list.is_empty() {
        "Sign in to mine.".into()
    } else {
        "Mining needs a GQL sign-in (Settings → Accounts).".into()
    })
}

fn set_account(app: &AppHandle, account: Option<String>, problem: Option<String>) {
    let old = {
        let mut st = lock(app);
        st.problem = problem;
        let old = std::mem::replace(&mut st.account, account.clone());
        if old != account {
            st.slots.clear();
            for c in &mut st.channels {
                c.balance = None;
                c.context_at = 0;
                c.drops_at = 0;
                c.token = None;
            }
        }
        old
    };
    if old == account {
        return;
    }
    for kind in USER_KINDS {
        if let Some(o) = &old {
            crate::hermes::remove_user(app, o, kind);
        }
        if let Some(a) = &account {
            crate::hermes::add_user(app, a, kind);
        }
    }
    super::watch::topics(app);
}

/// W3 The account's follows, A-Z, read again every 15 minutes.
async fn load_follows(app: &AppHandle, account: &str, f: &mut Follows) {
    let fresh = f.account == account && f.at.is_some_and(|t| t.elapsed() < FOLLOWS_AGE);
    if fresh {
        return;
    }
    match crate::browse::follows::follows(app, account, false).await {
        Ok(cards) => {
            let mut list: Vec<Named> = cards.into_iter().map(|c| (c.id, c.login, c.name)).collect();
            list.sort_by_key(|c| c.2.to_lowercase());
            *f = Follows {
                account: account.to_owned(),
                at: Some(Instant::now()),
                list,
            };
        }
        Err(e) => {
            eprintln!("miner: follows: {e}");
            log::add(app, LogKind::Error, None, "Couldn’t refresh the channel list.");
        }
    }
}

/// The two picked first, then the list or the follows (minus skipped ones).
fn wanted(s: &MinerSettings, follows: &[Named]) -> Vec<Named> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut push = |id: &str, login: &str, name: &str| {
        if seen.insert(id.to_owned()) {
            out.push((id.to_owned(), login.to_owned(), name.to_owned()));
        }
    };
    for c in &s.first {
        push(&c.id, &c.login, &c.name);
    }
    match s.source {
        MineSource::List => {
            for c in &s.channels {
                push(&c.id, &c.login, &c.name);
            }
        }
        MineSource::Follows => {
            let skip: HashSet<&str> = s.skip.iter().map(|c| c.id.as_str()).collect();
            for (id, login, name) in follows {
                if !skip.contains(id.as_str()) {
                    push(id, login, name);
                }
            }
        }
    }
    out
}

/// Puts the channels in order, keeping what's known about each. `found` is the live
/// check, when it worked.
fn set_channels(app: &AppHandle, wanted: &[Named], found: Option<&[MinerChannel]>) {
    let now = now_ms();
    let mut st = lock(app);
    let mut events: Vec<(String, &str)> = Vec::new();
    let mut old: Vec<Chan> = std::mem::take(&mut st.channels);
    let mut list = Vec::with_capacity(wanted.len());
    for (id, login, name) in wanted {
        let mut fresh = false;
        let mut c = match old.iter().position(|c| c.id == *id) {
            Some(i) => old.swap_remove(i),
            None => {
                fresh = true;
                Chan::new(id, login, name)
            }
        };
        let Some(found) = found else {
            if st.account.is_none() {
                c.stream = None;
                c.live_since = 0;
            }
            list.push(c);
            continue;
        };
        match found.iter().find(|f| f.id == *id) {
            Some(f) => {
                c.login = f.login.clone();
                c.name = f.name.clone();
                c.avatar = f.avatar.clone();
                match (&c.stream, &f.stream) {
                    (None, Some(s)) => {
                        if !fresh {
                            events.push((c.name.clone(), "Went live."));
                        }
                        c.streak.went_live();
                        c.live_since = live_since(&s.started_at, now);
                        c.context_at = 0;
                    }
                    (Some(_), None) => {
                        events.push((c.name.clone(), "Went offline."));
                        went_offline(&mut c, now);
                    }
                    (Some(a), Some(b)) if a.id != b.id => {
                        c.streak.went_live();
                        c.live_since = now;
                    }
                    _ => {}
                }
                c.stream = f.stream.clone();
            }
            None => {
                if c.stream.is_some() {
                    events.push((c.name.clone(), "Went offline."));
                    went_offline(&mut c, now);
                }
                c.stream = None;
            }
        }
        list.push(c);
    }
    st.channels = list;
    let live: HashSet<&str> = st
        .channels
        .iter()
        .filter(|c| c.stream.is_some())
        .map(|c| c.id.as_str())
        .collect();
    let slots: Vec<String> = st
        .slots
        .iter()
        .filter(|id| live.contains(id.as_str()))
        .cloned()
        .collect();
    st.slots = slots;
    drop(st);
    for (name, text) in events {
        log::add(app, LogKind::Channel, Some(&name), text);
    }
}

pub(super) fn went_offline(c: &mut Chan, now: i64) {
    c.streak.went_offline(now);
    c.live_since = 0;
    c.token = None;
    c.stream = None;
}

/// A stream that started a while ago counts right away; a new one after `SETTLE_MS`.
fn live_since(started_at: &str, now: i64) -> i64 {
    let started = time::OffsetDateTime::parse(started_at, &time::format_description::well_known::Rfc3339)
        .map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
        .unwrap_or(now);
    if now - started >= SETTLE_MS {
        now - SETTLE_MS
    } else {
        now
    }
}

/// Points contexts (balance, chest, multipliers, goals) of live channels that are new or
/// 30 minutes old, and drops of the first few.
async fn contexts(app: &AppHandle, account: &str, s: &MinerSettings) {
    let now = now_ms();
    let (due, drops_due): (Vec<(String, String)>, Vec<String>) = {
        let st = lock(app);
        let live = st.channels.iter().filter(|c| c.stream.is_some());
        let due = live
            .clone()
            .filter(|c| now - c.context_at >= CONTEXT_AGE)
            .map(|c| (c.id.clone(), c.login.clone()))
            .collect();
        let drops = live
            .take(DROPS_CHECKED)
            .filter(|c| s.claim_drops && now - c.drops_at >= DROPS_AGE)
            .map(|c| c.id.clone())
            .collect();
        (due, drops)
    };
    if due.is_empty() && drops_due.is_empty() {
        return;
    }
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let http = http(app);
    for (id, login) in due {
        match gm::points_context(&http, gql_auth(&token, &client_id), &login).await {
            Ok(Some(ctx)) => {
                {
                    let mut st = lock(app);
                    if let Some(c) = st.channels.iter_mut().find(|c| c.id == id) {
                        c.balance = Some(ctx.balance);
                        c.multiplier = ctx.multiplier;
                        c.context_at = now;
                    }
                }
                if let Some(claim) = &ctx.claim {
                    super::claims::bonus(app, account, &id, claim).await;
                }
                if s.goals {
                    super::goals::contribute(app, account, &ctx).await;
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("miner: points of {login}: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    for id in drops_due {
        let has = cyex_twitch::gql::drops::channel_campaigns(&http, gql_auth(&token, &client_id), &id)
            .await
            .map(|c| !c.is_empty());
        let mut st = lock(app);
        if let Some(c) = st.channels.iter_mut().find(|c| c.id == id) {
            c.drops_at = now;
            if let Ok(has) = has {
                c.drops = has;
            }
        }
    }
}

/// Hermes said a mined stream went up: look now instead of within the minute.
pub fn nudge(app: &AppHandle) {
    if let Some(m) = app.try_state::<Miner>() {
        m.wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::schema::TabChannel;

    fn ch(id: &str) -> TabChannel {
        TabChannel {
            id: id.into(),
            login: format!("c{id}"),
            name: format!("C{id}"),
        }
    }

    #[test]
    fn first_ones_lead_and_skips_drop_out() {
        let s = MinerSettings {
            first: vec![ch("3")],
            channels: vec![ch("1"), ch("3"), ch("2")],
            skip: vec![ch("5")],
            ..MinerSettings::default()
        };
        let ids = |v: Vec<Named>| v.into_iter().map(|c| c.0).collect::<Vec<_>>();
        assert_eq!(ids(wanted(&s, &[])), ["3", "1", "2"]);
        let follows: Vec<Named> = ["4", "5"]
            .iter()
            .map(|i| (i.to_string(), String::new(), String::new()))
            .collect();
        let f = MinerSettings {
            source: MineSource::Follows,
            ..s
        };
        assert_eq!(ids(wanted(&f, &follows)), ["3", "4"]);
    }

    #[test]
    fn old_streams_count_at_once() {
        let now = 1_791_630_000_000;
        assert_eq!(live_since("2026-10-10T09:00:00Z", now), now - SETTLE_MS);
        assert_eq!(live_since("not a date", now), now);
    }
}
