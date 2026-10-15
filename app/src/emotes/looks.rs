//! Who wears what, for the UI. Windows ask for the people they draw; changes go to every
//! window a quarter second later, batched.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use cyex_emotes::Looks;
use cyex_emotes::cosmetics::SevenChange;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use super::Emotes;

const EVENT: &str = "emotes:looks";
const BATCH: Duration = Duration::from_millis(250);

#[derive(Default)]
struct Pending {
    users: Mutex<HashSet<String>>,
    wake: Notify,
}

pub fn spawn(app: AppHandle) {
    app.manage(Pending::default());
    tauri::async_runtime::spawn(async move {
        loop {
            let pending = app.state::<Pending>();
            pending.wake.notified().await;
            tokio::time::sleep(BATCH).await;
            let users: Vec<String> =
                std::mem::take(&mut *pending.users.lock().unwrap_or_else(|p| p.into_inner()))
                    .into_iter()
                    .collect();
            if users.is_empty() {
                continue;
            }
            let looks = looks_for(&app, &users);
            let _ = app.emit(EVENT, looks);
        }
    });
}

/// These users' looks changed.
pub fn changed(app: &AppHandle, users: impl IntoIterator<Item = String>) {
    let Some(pending) = app.try_state::<Pending>() else {
        return;
    };
    let mut set = pending.users.lock().unwrap_or_else(|p| p.into_inner());
    let before = set.len();
    set.extend(users);
    if set.len() > before {
        pending.wake.notify_one();
    }
}

pub fn looks_for(app: &AppHandle, users: &[String]) -> Looks {
    let state = app.state::<Emotes>();
    let inner = state.lock();
    inner.cosmetics.looks(users.iter().take(500).map(String::as_str))
}

/// V1051-03 What an account now wears.
pub enum Worn {
    Paint(Option<cyex_emotes::seventv::paint::PaintStyle>),
    Badge(Option<cyex_emotes::cosmetics::BadgeDef>),
}

/// V1051-03 The account's own new paint or badge, shown at once (before 7TV's events arrive).
pub fn wear(app: &AppHandle, twitch_id: &str, change: Worn) {
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        match &change {
            Worn::Paint(p) => {
                if let Some(p) = p {
                    inner.cosmetics.add_paint(p.clone());
                }
                let id = p.as_ref().map(|p| p.id.as_str());
                inner.cosmetics.set_seventv(twitch_id, SevenChange::Paint(id));
            }
            Worn::Badge(b) => {
                if let Some(b) = b {
                    inner.cosmetics.add_badge(b.clone());
                }
                let id = b.as_ref().map(|b| b.id.as_str());
                inner.cosmetics.set_seventv(twitch_id, SevenChange::Badge(id));
            }
        }
    }
    changed(app, [twitch_id.to_owned()]);
}
