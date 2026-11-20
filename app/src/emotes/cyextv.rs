//! V112-04 cyex.tv personal sets: the people a window draws are looked up in batches of at most
//! 100, and their sets go in as personal ones (the same slots 7TV's and FFZ's use), so their
//! lines are read again with the new emotes. A person with no set is asked about once a session.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::cyextv::api;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use super::Emotes;
use crate::auth::Auth;

/// Wait for more people to gather before one lookup.
const GATHER: Duration = Duration::from_millis(400);
/// After a failed lookup nobody is asked for this long.
const COOL_OFF: Duration = Duration::from_secs(30);
/// People remembered as asked; past this the list starts over.
const MAX_ASKED: usize = 20_000;
/// People waiting for a lookup at most (newest dropped past it, asked again when drawn again).
const MAX_QUEUE: usize = 2_000;

#[derive(Default)]
struct Queue {
    state: Mutex<State>,
    wake: Notify,
}

#[derive(Default)]
struct State {
    asked: HashSet<String>,
    waiting: Vec<String>,
    /// No lookup before then.
    cool_until: Option<Instant>,
}

pub fn spawn(app: AppHandle) {
    app.manage(Queue::default());
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

/// Up to [`api::MAX_LOOKUP`] people to ask about, or how long to wait first.
fn take(queue: &Queue) -> (Vec<String>, Option<Duration>) {
    let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
    if s.waiting.is_empty() {
        return (Vec::new(), None);
    }
    if let Some(wait) = s
        .cool_until
        .and_then(|t| t.checked_duration_since(Instant::now()))
    {
        return (Vec::new(), Some(wait));
    }
    let n = s.waiting.len().min(api::MAX_LOOKUP);
    (s.waiting.drain(..n).collect(), None)
}

/// These people were drawn: those not asked about yet are looked up soon.
pub fn want(app: &AppHandle, users: &[String]) {
    let Some(queue) = app.try_state::<Queue>() else {
        return;
    };
    if super::providers::current(app).sets_off.cyextv {
        return;
    }
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
    if s.waiting.len() > before {
        queue.wake.notify_one();
    }
}

/// V112-06 These people are asked about again, e.g. after the live socket was down.
pub fn again(app: &AppHandle, users: &[String]) {
    if let Some(queue) = app.try_state::<Queue>() {
        let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
        for u in users {
            s.asked.remove(u);
        }
    }
    want(app, users);
}

async fn look_up(app: &AppHandle, queue: &Queue, batch: Vec<String>) {
    let http = app.state::<Auth>().http.clone();
    let got = api::personal_raw(&http, &batch, Lane::Back)
        .await
        .map_err(|e| e.describe("cyex.tv"))
        .and_then(|v| api::parse_personal(&v).map_err(|e| format!("unexpected answer ({e})")));
    match got {
        Ok(sets) => put(app, sets),
        Err(e) => {
            eprintln!("emotes: no cyex.tv personal sets: {e}");
            let mut s = queue.state.lock().unwrap_or_else(|p| p.into_inner());
            // Asked again when drawn again, after the pause.
            for u in &batch {
                s.asked.remove(u);
            }
            s.cool_until = Some(Instant::now() + COOL_OFF);
        }
    }
}

/// Each user's set in, and their lines drawn again.
fn put(app: &AppHandle, sets: Vec<(String, cyex_emotes::EmoteSet)>) {
    if sets.is_empty() {
        return;
    }
    let users: Vec<String> = sets.iter().map(|(u, _)| u.clone()).collect();
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        for (user, set) in sets {
            debug_assert_eq!(set.provider, Provider::Cyextv);
            let id = set.id.clone();
            inner.personal_sets.insert(id.clone(), Arc::new(set));
            let list = inner.user_sets.get_or_default(user);
            if !list.contains(&id) {
                list.push(id);
            }
        }
        // As the other personal sets: every user stamped, then rebuilt, then one trim.
        for u in &users {
            inner.rebuild_personal(u);
        }
        inner.trim_personal();
    }
    crate::chat::retokenize_users(app, &users);
}
