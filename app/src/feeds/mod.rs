//! V106-07 Feed tabs of their own kind: live notifications, AutoMod, whispers. Each keeps
//! this session's entries in memory (capped, Q4) and streams changes to the tabs showing it.
//! Content is per kind, not per tab: two tabs of one kind show the same list.

pub mod automod;
pub mod live;
mod ring;
pub mod whispers;

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use cyex_settings::schema::FeedKind;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use ring::Ring;

/// Q4 Entries each feed keeps.
pub const LIVE: usize = 300;
pub const HELD: usize = 200;
pub const WHISPERS: usize = 200;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum FeedEntry {
    Live(live::LiveEntry),
    Held(automod::HeldEntry),
    Whisper(whispers::WhisperEntry),
}

impl FeedEntry {
    pub fn key(&self) -> &str {
        match self {
            Self::Live(e) => &e.key,
            Self::Held(e) => &e.key,
            Self::Whisper(e) => &e.key,
        }
    }
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum KindEvent {
    Add { entry: FeedEntry },
    Update { entry: FeedEntry },
    Remove { key: String },
    Clear,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FeedsOpened {
    pub view: u32,
    pub entries: Vec<FeedEntry>,
}

type View = Channel<Vec<KindEvent>>;

/// Lock order: `rings`, released, then `views`; never both at once.
#[derive(Default)]
struct Store {
    rings: Mutex<HashMap<FeedKind, Ring<FeedEntry>>>,
    views: Mutex<Vec<(u32, FeedKind, View)>>,
    next: AtomicU32,
}

pub fn init(app: &AppHandle) {
    app.manage(Store::default());
    app.manage(live::Last::default());
}

fn cap(kind: FeedKind) -> usize {
    match kind {
        FeedKind::Live => LIVE,
        FeedKind::Automod => HELD,
        FeedKind::Whispers => WHISPERS,
        FeedKind::Rules => 0,
    }
}

/// Changes one kind's ring and returns what `f` gives back; the lock is released after.
fn with_ring<R>(app: &AppHandle, kind: FeedKind, f: impl FnOnce(&mut Ring<FeedEntry>) -> R) -> Option<R> {
    let store = app.try_state::<Store>()?;
    let mut rings = store.rings.lock().unwrap_or_else(|p| p.into_inner());
    Some(f(rings.entry(kind).or_insert_with(|| Ring::new(cap(kind)))))
}

fn send(app: &AppHandle, kind: FeedKind, events: Vec<KindEvent>) {
    if events.is_empty() {
        return;
    }
    let Some(store) = app.try_state::<Store>() else {
        return;
    };
    let mut views = store.views.lock().unwrap_or_else(|p| p.into_inner());
    views.retain(|v| v.1 != kind || v.2.send(events.clone()).is_ok());
}

/// Adds an entry. Past the cap the oldest one `keep` doesn't protect goes.
pub(crate) fn push(app: &AppHandle, kind: FeedKind, entry: FeedEntry, keep: impl Fn(&FeedEntry) -> bool) {
    let Some(dropped) = with_ring(app, kind, |r| r.push(entry.clone(), keep)) else {
        return;
    };
    let mut events = vec![KindEvent::Add { entry }];
    events.extend(dropped.map(|d| KindEvent::Remove {
        key: d.key().to_owned(),
    }));
    send(app, kind, events);
}

/// Changes the first entry `hit` finds. Returns whether one was found.
pub(crate) fn update(
    app: &AppHandle,
    kind: FeedKind,
    hit: impl Fn(&FeedEntry) -> bool,
    f: impl FnOnce(&mut FeedEntry),
) -> bool {
    let Some(Some(entry)) = with_ring(app, kind, |r| r.update(hit, f)) else {
        return false;
    };
    send(app, kind, vec![KindEvent::Update { entry }]);
    true
}

/// Drops what `gone` names.
pub(crate) fn remove(app: &AppHandle, kind: FeedKind, gone: impl Fn(&FeedEntry) -> bool) {
    let dropped = with_ring(app, kind, |r| r.remove(gone)).unwrap_or_default();
    let events = dropped
        .into_iter()
        .map(|d| KindEvent::Remove {
            key: d.key().to_owned(),
        })
        .collect();
    send(app, kind, events);
}

pub(crate) fn clear(app: &AppHandle, kind: FeedKind) {
    if with_ring(app, kind, Ring::clear).is_some() {
        send(app, kind, vec![KindEvent::Clear]);
    }
}

/// Starts streaming a kind's changes. Returns the view id and the entries it has.
pub fn open(app: &AppHandle, kind: FeedKind, channel: View) -> FeedsOpened {
    let store = app.state::<Store>();
    let view = store.next.fetch_add(1, Ordering::Relaxed) + 1;
    let entries = with_ring(app, kind, |r| r.items()).unwrap_or_default();
    store
        .views
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push((view, kind, channel));
    FeedsOpened { view, entries }
}

pub fn close(app: &AppHandle, view: u32) {
    let Some(store) = app.try_state::<Store>() else {
        return;
    };
    store
        .views
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .retain(|v| v.0 != view);
}
