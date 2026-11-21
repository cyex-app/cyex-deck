//! V112-10..11 cyex.tv cosmetics of the people a window draws: the catalog of paints and badges
//! (read when first needed, then every 5 minutes) and, in batches of at most 100, what each
//! chatter wears. Someone missing from an answer wears nothing. A person is asked about once a
//! session; the live socket (`apply_user_cosmetics`) keeps them current after that: it follows
//! `cosmetics` and the `user:twitch:<id>` of the last `MAX_FOLLOWED` people drawn (the oldest topic
//! goes when a newer person comes, so the count stays bounded), and none while the setting is off.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cyex_emotes::cyextv::api::MAX_LOOKUP;
use cyex_emotes::cyextv::cosmetics::{self as api, Wear};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use super::super::Emotes;
use crate::auth::Auth;
use crate::settings::Settings;

/// Wait for more people to gather before one lookup.
const GATHER: Duration = Duration::from_millis(400);
/// After a failed read nothing is asked for this long.
const COOL_OFF: Duration = Duration::from_secs(30);
/// People remembered as asked; past this the list starts over.
const MAX_ASKED: usize = 20_000;
/// People waiting for a lookup at most (the rest are asked when drawn again).
const MAX_QUEUE: usize = 2_000;
/// People whose live topic is held (the 500 a socket takes also hold channels and sets; more
/// sockets open past that, but this keeps the count small).
const MAX_FOLLOWED: usize = 300;

#[derive(Default)]
struct Queue {
    state: Mutex<State>,
    wake: Notify,
    /// The catalog should be read now (first need, or a person wears something it lacks).
    catalog: Notify,
}

#[derive(Default)]
struct State {
    asked: HashSet<String>,
    waiting: Vec<String>,
    cool_until: Option<Instant>,
    /// The catalog was read at least once.
    have_catalog: bool,
    /// V112-13 Whose `user:` topic is held, with when they were last drawn (a counter).
    following: HashMap<String, u64>,
    seq: u64,
    /// V112-13 The `cosmetics` topic is held.
    catalog_topic: bool,
}

pub fn spawn(app: AppHandle) {
    app.manage(Queue::default());
    let catalog_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let queue = catalog_app.state::<Queue>();
        loop {
            queue.catalog.notified().await;
            if !on(&catalog_app) {
                continue;
            }
            loop {
                read_catalog(&catalog_app, &queue).await;
                // Asked for again right away: not more often than this.
                tokio::time::sleep(Duration::from_secs(15)).await;
                // Again in 5 minutes, or sooner when asked to; switched off, back to waiting.
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(api::REFRESH_SECS)) => {}
                    () = queue.catalog.notified() => {}
                }
                if !on(&catalog_app) {
                    break;
                }
            }
        }
    });
    tauri::async_runtime::spawn(async move {
        loop {
            let queue = app.state::<Queue>();
            queue.wake.notified().await;
            tokio::time::sleep(GATHER).await;
            loop {
                let (batch, wait) = take(&queue);
                if let Some(wait) = wait {
                    tokio::time::sleep(wait).await;
                    continue;
                }
                if batch.is_empty() {
                    break;
                }
                look_up(&app, &queue, batch).await;
            }
        }
    });
}

fn on(app: &AppHandle) -> bool {
    wanted(&app.state::<Settings>().read::<ChatSettings>(Partition::Chat), cyex_core::CYEXTV_READY)
}

/// V112-23 The setting counts only while cyex.tv is ready: off, nothing is looked up or followed.
fn wanted(c: &ChatSettings, cyextv_ready: bool) -> bool {
    cyextv_ready && c.cyextv_cosmetics
}

fn take(queue: &Queue) -> (Vec<String>, Option<Duration>) {
    let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
    if s.waiting.is_empty() {
        return (Vec::new(), None);
    }
    if let Some(wait) = s.cool_until.and_then(|t| t.checked_duration_since(Instant::now())) {
        return (Vec::new(), Some(wait));
    }
    let n = s.waiting.len().min(MAX_LOOKUP);
    (s.waiting.drain(..n).collect(), None)
}

/// These people were drawn: those not asked about yet are looked up soon.
pub fn want(app: &AppHandle, users: &[String]) {
    let Some(queue) = app.try_state::<Queue>() else {
        return;
    };
    if !on(app) {
        return;
    }
    follow(app, &queue, users);
    let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
    let before = s.waiting.len();
    for u in users.iter().take(500) {
        if u.is_empty() || !u.bytes().all(|b| b.is_ascii_digit()) || s.waiting.len() >= MAX_QUEUE {
            continue;
        }
        if s.asked.len() >= MAX_ASKED {
            s.asked.clear();
        }
        if s.asked.insert(u.clone()) {
            s.waiting.push(u.clone());
        }
    }
    let first = !s.have_catalog;
    if s.waiting.len() > before {
        queue.wake.notify_one();
    }
    drop(s);
    if first {
        queue.catalog.notify_one();
    }
}

/// V112-13 These people are drawn: their live topics are held (the oldest go past the bound).
fn follow(app: &AppHandle, queue: &Queue, users: &[String]) {
    let (added, dropped, first) = {
        let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
        let mut added = Vec::new();
        for u in users.iter().take(MAX_FOLLOWED).filter(|u| numeric(u)) {
            s.seq += 1;
            let seq = s.seq;
            if s.following.insert(u.clone(), seq).is_none() {
                added.push(u.clone());
            }
        }
        let mut dropped = Vec::new();
        while s.following.len() > MAX_FOLLOWED {
            let Some(old) = s.following.iter().min_by_key(|(_, seq)| **seq).map(|(u, _)| u.clone()) else {
                break;
            };
            s.following.remove(&old);
            dropped.push(old);
        }
        let first = !s.catalog_topic;
        s.catalog_topic = true;
        (added, dropped, first)
    };
    let live = &app.state::<Emotes>().cyextv_live;
    if first {
        live.follow_catalog(true);
    }
    for u in &added {
        live.follow_user(u, true);
    }
    for u in &dropped {
        live.follow_user(u, false);
    }
}

fn numeric(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
}

/// V112-13 The setting changed: switched off, every live topic of ours goes. (Switched on, they
/// come back with the next people drawn.)
pub fn sync(app: &AppHandle) {
    if on(app) {
        return;
    }
    let Some(queue) = app.try_state::<Queue>() else {
        return;
    };
    let (users, catalog) = {
        let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
        let users: Vec<String> = s.following.drain().map(|(u, _)| u).collect();
        (users, std::mem::take(&mut s.catalog_topic))
    };
    let live = &app.state::<Emotes>().cyextv_live;
    if catalog {
        live.follow_catalog(false);
    }
    for u in &users {
        live.follow_user(u, false);
    }
}

/// V112-13 The server says anything may have changed: the catalog and everyone followed are read
/// again.
pub fn resync(app: &AppHandle) {
    if !on(app) {
        return;
    }
    let Some(queue) = app.try_state::<Queue>() else {
        return;
    };
    {
        let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
        let users: Vec<String> = s.following.keys().cloned().collect();
        for u in users {
            if s.waiting.len() < MAX_QUEUE && !s.waiting.contains(&u) {
                s.asked.insert(u.clone());
                s.waiting.push(u);
            }
        }
    }
    queue.wake.notify_one();
    refresh_catalog(app);
}

/// V112-11 Read the catalog now (a socket event named a paint or badge it doesn't have, or the
/// socket was down); it is read again every 5 minutes after that.
pub fn refresh_catalog(app: &AppHandle) {
    if !on(app) {
        return;
    }
    if let Some(queue) = app.try_state::<Queue>() {
        queue.catalog.notify_one();
    }
}

async fn read_catalog(app: &AppHandle, queue: &Queue) {
    let http = app.state::<Auth>().http.clone();
    let got = api::catalog_raw(&http, Lane::Back)
        .await
        .map_err(|e| e.describe("cyex.tv"))
        .and_then(|v| api::parse_catalog(&v).map_err(|e| format!("unexpected answer ({e})")));
    match got {
        Ok(c) => {
            let users = {
                let state = app.state::<Emotes>();
                let mut inner = state.lock();
                inner.cosmetics.replace_cyextv_catalog(c.paints, c.badges)
            };
            queue.state.lock().unwrap_or_else(|p| p.into_inner()).have_catalog = true;
            if !users.is_empty() {
                super::changed(app, users);
            }
        }
        Err(e) => eprintln!("emotes: no cyex.tv cosmetics: {e}"),
    }
}

async fn look_up(app: &AppHandle, queue: &Queue, batch: Vec<String>) {
    let http = app.state::<Auth>().http.clone();
    let got = api::users_raw(&http, &batch, Lane::Back)
        .await
        .map_err(|e| e.describe("cyex.tv"))
        .and_then(|v| api::parse_users(&v).map_err(|e| format!("unexpected answer ({e})")));
    match got {
        Ok(wears) => put(app, &batch, wears),
        Err(e) => {
            eprintln!("emotes: no cyex.tv cosmetics for chatters: {e}");
            let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
            for u in &batch {
                s.asked.remove(u);
            }
            s.cool_until = Some(Instant::now() + COOL_OFF);
        }
    }
}

/// The answer in: everyone asked about is set, those not listed to nothing.
fn put(app: &AppHandle, asked: &[String], wears: Vec<Wear>) {
    let mut changed = Vec::new();
    let mut missing_catalog = false;
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        let listed: HashSet<&str> = wears.iter().map(|w| w.user.as_str()).collect();
        for u in asked.iter().filter(|u| !listed.contains(u.as_str())) {
            if inner.cosmetics.set_cyextv_user(u, None, None) {
                changed.push(u.clone());
            }
        }
        for w in &wears {
            if inner.cosmetics.set_cyextv_user(&w.user, w.badge.as_deref(), w.paint.as_deref()) {
                changed.push(w.user.clone());
            }
            missing_catalog |= inner.cosmetics.look(&w.user).is_none();
        }
    }
    if !changed.is_empty() {
        super::changed(app, changed);
    }
    // Someone wears what the catalog lacks: it is read again (the new definitions redraw them).
    if missing_catalog {
        refresh_catalog(app);
    }
}

/// V112-11 A live change: what this Twitch user now wears (catalog ids; `None` takes it off).
/// For the cyex.tv socket's `user.cosmetics_updated`.
pub fn apply_user_cosmetics(app: &AppHandle, user_id: &str, badge_id: Option<&str>, paint_id: Option<&str>) {
    if !on(app) {
        return;
    }
    let (changed, unknown) = {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        let changed = inner.cosmetics.set_cyextv_user(user_id, badge_id, paint_id);
        let unknown = (badge_id.is_some() || paint_id.is_some()) && inner.cosmetics.look(user_id).is_none();
        (changed, unknown)
    };
    if changed {
        super::changed(app, [user_id.to_owned()]);
    }
    if unknown {
        refresh_catalog(app);
    }
}

#[cfg(test)]
mod tests {
    use cyex_settings::schema::Chat;

    use super::wanted;

    /// V112-23 cyex.tv coming soon: the saved setting (on by default) starts no lookups, catalog
    /// reads or live topics until the switch flips.
    #[test]
    fn v112_23_cosmetics_wait_for_the_switch() {
        let c = Chat::default();
        assert!(c.cyextv_cosmetics, "the saved default stays on");
        assert!(!wanted(&c, false));
        assert!(wanted(&c, true));
        assert!(!wanted(&Chat { cyextv_cosmetics: false, ..Chat::default() }, true));
        assert_eq!(wanted(&c, cyex_core::CYEXTV_READY), cyex_core::CYEXTV_READY);
    }
}
