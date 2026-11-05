//! V109-B02 cyex badges in chat. The snapshot of everyone (`/badges/v1/users`) is asked a few
//! seconds after start and every 30 minutes with its ETag, kept as one map (Twitch id → badges)
//! and handed to `Cosmetics` as the `cyex` set. The switch (Chat → Emotes and badges) off: nothing
//! is asked and the set goes. An account's own equip change shows in its chat at once.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_emotes::badges::BadgeSet;
use cyex_emotes::badges::cyex::{self, Held, Kinds, Tagged};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use tauri::{AppHandle, Emitter, Manager};

use super::Emotes;
use super::extra::EVENT;

/// Startup settles first.
const FIRST: Duration = Duration::from_secs(4);
/// After a failed ask (the spec asks for 5 minutes at least).
const RETRY: Duration = Duration::from_secs(5 * 60);
/// An own change wins over the snapshot until the snapshot agrees, or this long.
const OWN_KEEP: Duration = Duration::from_secs(60 * 60);

#[derive(Default)]
struct Book {
    /// The running task; a newer number replaces an older one.
    run: Option<u64>,
    next: u64,
    tag: Tagged,
    kinds: Option<Kinds>,
    users: HashMap<String, Vec<Held>>,
    /// Own accounts' badges after an equip change or a `/me` read.
    own: HashMap<String, (Instant, Vec<Held>)>,
}

#[derive(Default)]
struct State(Mutex<Book>);

impl State {
    fn lock(&self) -> MutexGuard<'_, Book> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

pub(super) fn init(app: &AppHandle) {
    app.manage(State::default());
    sync(app);
}

fn wanted(app: &AppHandle) -> bool {
    app.state::<crate::settings::Settings>()
        .read::<ChatSettings>(Partition::Chat)
        .badges
        .cyex
}

/// The switch changed (or the app started).
pub(super) fn sync(app: &AppHandle) {
    let Some(st) = app.try_state::<State>() else {
        return;
    };
    let want = wanted(app);
    let (start, stop) = {
        let mut b = st.lock();
        match (want, b.run) {
            (true, None) => {
                b.next += 1;
                b.run = Some(b.next);
                (b.run, false)
            }
            (false, Some(_)) => {
                b.run = None;
                b.tag.reset();
                b.users.clear();
                b.kinds = None;
                (None, true)
            }
            _ => (None, false),
        }
    };
    if stop {
        let had = {
            let state = app.state::<Emotes>();
            let mut inner = state.lock();
            let had = inner.cosmetics.extra_loaded(BadgeSet::Cyex);
            inner.cosmetics.clear_extra(BadgeSet::Cyex);
            had
        };
        if had {
            let _ = app.emit(EVENT, ());
        }
    }
    if let Some(run) = start {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { keep(app, run).await });
    }
}

fn current(app: &AppHandle, run: u64) -> bool {
    app.try_state::<State>().is_some_and(|s| s.lock().run == Some(run))
}

async fn keep(app: AppHandle, run: u64) {
    let mut wait = FIRST;
    loop {
        tokio::time::sleep(wait).await;
        if !current(&app, run) {
            return;
        }
        let http = app.state::<crate::auth::Auth>().http.clone();
        let base = crate::cyex::base(&app);
        let st = app.state::<State>();
        let need_kinds = st.lock().kinds.is_none();
        let kinds = if need_kinds { cyex::fetch_kinds(&http, &base).await.ok() } else { None };
        let mut tag = std::mem::take(&mut st.lock().tag);
        let got = cyex::fetch_users(&http, &base, &mut tag).await;
        let new_kinds = {
            let mut b = st.lock();
            if b.run != Some(run) {
                return;
            }
            b.tag = tag;
            let new = kinds.is_some();
            if kinds.is_some() {
                b.kinds = kinds;
            }
            new
        };
        match got {
            Ok(Some(snap)) => {
                let people = snap.users.len();
                {
                    let mut b = st.lock();
                    b.users = snap.users;
                    let users = &b.users;
                    let now = Instant::now();
                    let agreed: Vec<String> = b
                        .own
                        .iter()
                        .filter(|(id, (at, list))| now.duration_since(*at) > OWN_KEEP || same(users.get(*id), list))
                        .map(|(id, _)| id.clone())
                        .collect();
                    for id in agreed {
                        b.own.remove(&id);
                    }
                }
                eprintln!("emotes: cyex badges for {people} people");
                apply(&app);
                wait = cyex::REFRESH;
            }
            Ok(None) => {
                if new_kinds {
                    apply(&app);
                }
                wait = cyex::REFRESH;
            }
            Err(e) => {
                eprintln!("emotes: no cyex badges: {}", e.describe("cyex"));
                wait = RETRY;
            }
        }
    }
}

/// Whether the snapshot shows what the own change shows.
fn same(snap: Option<&Vec<Held>>, own: &[Held]) -> bool {
    let a: Vec<&Held> = snap.map(|l| cyex::shown(l)).unwrap_or_default();
    a == cyex::shown(own)
}

/// Hands everyone's badges (own changes on top) to chat.
fn apply(app: &AppHandle) {
    let Some(st) = app.try_state::<State>() else {
        return;
    };
    let base = crate::cyex::base(app);
    let loaded = {
        let b = st.lock();
        if b.run.is_none() {
            return;
        }
        let mut all: HashMap<&String, &Vec<Held>> = b.users.iter().collect();
        for (id, (_, list)) in &b.own {
            all.insert(id, list);
        }
        cyex::build(b.kinds.as_ref(), &base, all)
    };
    app.state::<Emotes>().lock().cosmetics.set_extra(BadgeSet::Cyex, loaded);
    let _ = app.emit(EVENT, ());
}

/// The pictures of a built-in badge (`founder`, `supporter`), for the cosmetics preview.
pub fn pictures(app: &AppHandle, kind: &str) -> cyex::Images {
    let base = crate::cyex::base(app);
    match app.try_state::<State>() {
        Some(st) => cyex::pictures(st.lock().kinds.as_ref(), &base, kind),
        None => cyex::pictures(None, &base, kind),
    }
}

/// An own account's badges as cyex says now (`None`: the account is gone).
pub fn set_own(app: &AppHandle, account: &str, list: Option<Vec<Held>>) {
    let Some(st) = app.try_state::<State>() else {
        return;
    };
    let changed = {
        let mut b = st.lock();
        match list {
            Some(l) => {
                let old = b.own.get(account).map(|(_, o)| o.clone());
                let shown_now = b.users.get(account);
                let differs = old.as_ref() != Some(&l) && !(old.is_none() && same(shown_now, &l));
                if differs {
                    b.own.insert(account.to_owned(), (Instant::now(), l));
                }
                differs
            }
            None => b.own.remove(account).is_some(),
        }
    };
    if changed {
        apply(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(id: &str) -> Held {
        Held {
            id: id.into(),
            number: (id == "founder").then_some(1),
            images: None,
        }
    }

    #[test]
    fn own_changes_match_the_snapshot_by_what_shows() {
        let snap = vec![held("founder"), held("supporter")];
        assert!(same(Some(&snap), &[held("founder")]), "the rule shows founder only");
        assert!(!same(Some(&snap), &[held("supporter")]));
        assert!(same(None, &[]), "nothing on both sides");
        assert!(!same(None, &[held("supporter")]));
    }
}
