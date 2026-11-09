//! V110-U68 The running drop campaigns and their watch-only badges: the dashboard's ACTIVE
//! ids, then their details 20 at a time, every 30 minutes (skipped while hidden and idle
//! unless badges are picked). A refused read keeps the badges known so far and says so in
//! the log, so it never reads as "nothing to earn".

use std::time::{Duration, Instant};

use cyex_settings::Partition;
use cyex_settings::schema::Miner as MinerSettings;
use cyex_twitch::gql::drops::both_refused;
use tauri::{AppHandle, Manager};

use super::log::{self, LogKind};
use super::{Miner, gql_auth, http, lock, now_ms};
use crate::settings::Settings;

/// After a failed read, the next try.
const RETRY: Duration = Duration::from_secs(5 * 60);
/// Longest nap before looking at the cadence again.
const NAP: Duration = Duration::from_secs(5 * 60);

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Let the first live check run first.
        tokio::time::sleep(Duration::from_secs(25)).await;
        let idle_wake = app.state::<Miner>().idle_wake.clone();
        // (account, when) of the last load, and whether it worked.
        let mut last: Option<(String, Instant, bool)> = None;
        loop {
            let s = super::settings(&app);
            let account = lock(&app).account.clone();
            let every = super::idle::cadence(&app).campaigns_every(!s.badges.picked.is_empty());
            if let (Some(account), Some(every)) = (account.filter(|_| s.enabled), every) {
                let gap = match &last {
                    Some((a, _, ok)) if *a == account => {
                        if *ok {
                            every
                        } else {
                            RETRY
                        }
                    }
                    _ => Duration::ZERO,
                };
                if last
                    .as_ref()
                    .is_none_or(|(a, t, _)| *a != account || t.elapsed() >= gap)
                {
                    let ok = load(&app, &account, &s).await;
                    last = Some((account, Instant::now(), ok));
                }
            }
            tokio::select! {
                () = tokio::time::sleep(NAP) => {}
                () = idle_wake.notified() => {}
            }
        }
    });
}

/// One read. False when it failed.
async fn load(app: &AppHandle, account: &str, s: &MinerSettings) -> bool {
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return false;
    };
    let http = http(app);
    let auth = gql_auth(&token, &client_id);
    let mut ids = match super::source::dashboard(app, &http, auth).await {
        Ok(ids) => ids,
        Err(e) => return failed(app, &e),
    };
    // Some accounts get no dashboard list: use the ids the channel reads already found,
    // and the inventory's (one extra read, only here).
    if ids.is_empty() {
        let seen: Vec<Vec<String>> = lock(app).channels.iter().map(|c| c.campaigns.clone()).collect();
        let inv: Vec<String> = match super::source::inventory(app, &http, auth).await {
            Ok(i) => i.campaigns.into_iter().map(|c| c.id).collect(),
            Err(_) => Vec::new(),
        };
        ids = merge_ids(&seen, &inv);
    }
    let campaigns = if ids.is_empty() {
        Vec::new()
    } else {
        match super::source::details(app, &http, auth, account, &ids).await {
            Ok(c) => c,
            Err(e) => return failed(app, &e),
        }
    };
    let now = now_ms() / 1000;
    let (before, after) = {
        let mut st = lock(app);
        let before: std::collections::HashSet<String> = cyex_miner::badges::open_campaigns(&st.badges, now);
        st.badges = cyex_miner::badges::merge(&st.badges, &campaigns, now);
        (before, cyex_miner::badges::open_campaigns(&st.badges, now))
    };
    let new: Vec<String> = {
        let st = lock(app);
        after
            .iter()
            .filter(|c| !before.contains(*c))
            .map(|c| super::badges::name_of(&st, c))
            .collect()
    };
    match new.len() {
        0 => {}
        1..=3 => {
            for name in &new {
                log::add(app, LogKind::Badge, None, format!("New badge to earn: {name}."));
            }
        }
        n => log::add(
            app,
            LogKind::Badge,
            None,
            format!("{n} badges can be earned by watching."),
        ),
    }
    // Picks of campaigns that ended go.
    let running: std::collections::HashSet<String> = ids.into_iter().collect();
    if s.badges.picked.iter().any(|p| !running.contains(p)) {
        let _ = app
            .state::<Settings>()
            .update::<MinerSettings>(app, Partition::Miner, |m| m.badges.retain_running(&running));
    }
    note_counts(app, running.len(), after.len());
    super::badges::prune(app, &super::settings(app));
    super::emit_status(app);
    super::channels::nudge(app);
    true
}

fn failed(app: &AppHandle, e: &cyex_twitch::Error) -> bool {
    let text = if both_refused(e) {
        "Couldn’t read the badge campaigns: Twitch turned both ways of asking down."
    } else {
        "Couldn’t read the badge campaigns."
    };
    log::add(app, LogKind::Error, None, text);
    false
}

/// Campaign ids from the channels' reads, then the inventory's, each once, in first-seen order.
fn merge_ids(channels: &[Vec<String>], inventory: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    channels
        .iter()
        .flatten()
        .chain(inventory)
        .filter(|id| seen.insert(id.as_str()))
        .cloned()
        .collect()
}

/// One neutral line per read, only when the numbers changed.
fn note_counts(app: &AppHandle, running: usize, earnable: usize) {
    static LAST: std::sync::Mutex<Option<(usize, usize)>> = std::sync::Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if *last != Some((running, earnable)) {
        *last = Some((running, earnable));
        log::add(
            app,
            LogKind::Badge,
            None,
            format!("Badge campaigns: {running} running, {earnable} you can earn."),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::merge_ids;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn v110_u68_channel_and_inventory_ids_merge_once() {
        let ch = vec![v(&["a", "b"]), v(&[]), v(&["b", "c"])];
        assert_eq!(merge_ids(&ch, &v(&["c", "d"])), v(&["a", "b", "c", "d"]));
        assert!(merge_ids(&[], &[]).is_empty());
    }
}
