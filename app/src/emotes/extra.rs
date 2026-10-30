//! V108-N06 Extra badge sets (FFZ:AP, BadgeHQ, Chatsen, DankChat, Homies). A set is fetched once
//! its switch is on (a few seconds after start so startup stays quiet), kept in `Cosmetics`, and
//! fetched again every few hours. Turning it off drops the list at once. One task per set; nothing
//! runs per message.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use cyex_emotes::badges::{self, BadgeSet};
use cyex_settings::Partition;
use cyex_settings::schema::{BadgeSets, Emotes as EmoteSettings};
use tauri::{AppHandle, Emitter, Manager};

use super::Emotes;

/// The UI asks everyone it draws again.
pub(super) const EVENT: &str = "emotes:badge-sets";
/// Startup settles first.
const FIRST: Duration = Duration::from_secs(4);
/// After a failed fetch: first wait, doubled each time up to the refresh time.
const RETRY: Duration = Duration::from_secs(120);

#[derive(Default)]
struct Tasks {
    /// The task each switched-on set runs; a newer number replaces an older task.
    live: HashMap<BadgeSet, u64>,
    next: u64,
}

#[derive(Default)]
struct Extra(Mutex<Tasks>);

impl Extra {
    fn lock(&self) -> std::sync::MutexGuard<'_, Tasks> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn current(&self, set: BadgeSet, run: u64) -> bool {
        self.lock().live.get(&set) == Some(&run)
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Extra::default());
    sync(app);
}

fn wanted(s: &BadgeSets) -> HashSet<BadgeSet> {
    [
        (s.ffzap, BadgeSet::FfzAp),
        (s.badgehq, BadgeSet::BadgeHq),
        (s.chatsen, BadgeSet::Chatsen),
        (s.dankchat, BadgeSet::DankChat),
        (s.homies, BadgeSet::Homies),
    ]
    .into_iter()
    .filter_map(|(on, set)| on.then_some(set))
    .collect()
}

/// The switches changed (or the app started): starts the sets now on, drops the ones now off.
pub fn sync(app: &AppHandle) {
    let Some(extra) = app.try_state::<Extra>() else {
        return;
    };
    let want = wanted(&app.state::<crate::settings::Settings>().read::<EmoteSettings>(Partition::Emotes).badge_sets);
    let mut started = Vec::new();
    let mut off = Vec::new();
    {
        let mut t = extra.lock();
        for set in BadgeSet::ALL {
            match (want.contains(&set), t.live.contains_key(&set)) {
                (true, false) => {
                    t.next += 1;
                    let run = t.next;
                    t.live.insert(set, run);
                    started.push((set, run));
                }
                (false, true) => {
                    t.live.remove(&set);
                    off.push(set);
                }
                _ => {}
            }
        }
    }
    if !off.is_empty() {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        let had = off.iter().any(|s| inner.cosmetics.extra_loaded(*s));
        for set in off {
            inner.cosmetics.clear_extra(set);
        }
        drop(inner);
        if had {
            let _ = app.emit(EVENT, ());
        }
    }
    for (set, run) in started {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { keep(app, set, run).await });
    }
}

/// Fetches a set, then again after the refresh time, until its switch goes off.
async fn keep(app: AppHandle, set: BadgeSet, run: u64) {
    let mut wait = FIRST;
    loop {
        tokio::time::sleep(wait).await;
        let Some(extra) = app.try_state::<Extra>() else {
            return;
        };
        if !extra.current(set, run) {
            return;
        }
        let http = app.state::<crate::auth::Auth>().http.clone();
        match badges::fetch(&http, set).await {
            Ok(loaded) => {
                // Switched off while it loaded: nothing to keep.
                if !extra.current(set, run) {
                    return;
                }
                let (badges, people) = (loaded.defs.len(), loaded.users.len());
                app.state::<Emotes>().lock().cosmetics.set_extra(set, loaded);
                eprintln!("emotes: {} badges: {badges} badges for {people} people", set.prefix());
                let _ = app.emit(EVENT, ());
                // V108-C10 Pictures that failed to load for comparing get another go.
                super::art::retry(&app);
                wait = badges::REFRESH;
            }
            Err(e) => {
                eprintln!("emotes: no {} badges: {}", set.prefix(), e.describe(set.prefix()));
                wait = if wait < badges::REFRESH { (wait * 2).max(RETRY).min(badges::REFRESH) } else { RETRY };
            }
        }
    }
}
