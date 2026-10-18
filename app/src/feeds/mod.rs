//! V106-07 Feed tabs of their own kind: live notifications, AutoMod, whispers. Each keeps
//! this session's entries in memory (capped, Q4) and streams changes to the tabs showing it.
//! Content is per kind, not per tab: two tabs of one kind show the same list.

pub mod automod;
pub mod live;
pub(crate) mod ring;
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

/// Lock order: `views` then `rings`, always (R3).
#[derive(Default)]
struct Store {
    rings: Mutex<HashMap<FeedKind, Ring<FeedEntry>>>,
    views: Mutex<Vec<(u32, FeedKind, View)>>,
    next: AtomicU32,
}

impl Store {
    /// R3 Changes one kind's ring and sends what `events` makes of the result, holding `views`
    /// the whole time, so a view opening meanwhile never gets an entry its snapshot has.
    fn change<R>(
        &self,
        kind: FeedKind,
        f: impl FnOnce(&mut Ring<FeedEntry>) -> R,
        events: impl FnOnce(&R) -> Vec<KindEvent>,
    ) -> R {
        let mut views = self.views.lock().unwrap_or_else(|p| p.into_inner());
        let out = {
            let mut rings = self.rings.lock().unwrap_or_else(|p| p.into_inner());
            f(rings.entry(kind).or_insert_with(|| Ring::new(cap(kind))))
        };
        let events = events(&out);
        if !events.is_empty() {
            views.retain(|v| v.1 != kind || v.2.send(events.clone()).is_ok());
        }
        out
    }

    fn open(&self, kind: FeedKind, channel: View) -> FeedsOpened {
        let view = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let mut views = self.views.lock().unwrap_or_else(|p| p.into_inner());
        let entries = {
            let mut rings = self.rings.lock().unwrap_or_else(|p| p.into_inner());
            rings.entry(kind).or_insert_with(|| Ring::new(cap(kind))).items()
        };
        views.push((view, kind, channel));
        FeedsOpened { view, entries }
    }
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

/// `Store::change` on the app's store; `None` before `init`.
fn change<R>(
    app: &AppHandle,
    kind: FeedKind,
    f: impl FnOnce(&mut Ring<FeedEntry>) -> R,
    events: impl FnOnce(&R) -> Vec<KindEvent>,
) -> Option<R> {
    Some(app.try_state::<Store>()?.change(kind, f, events))
}

fn removed(dropped: Option<&FeedEntry>) -> Option<KindEvent> {
    dropped.map(|d| KindEvent::Remove {
        key: d.key().to_owned(),
    })
}

/// Adds an entry. Past the cap the oldest one `keep` doesn't protect goes.
pub(crate) fn push(app: &AppHandle, kind: FeedKind, entry: FeedEntry, keep: impl Fn(&FeedEntry) -> bool) {
    change(
        app,
        kind,
        |r| r.push(entry.clone(), keep),
        |dropped| {
            let mut events = vec![KindEvent::Add { entry: entry.clone() }];
            events.extend(removed(dropped.as_ref()));
            events
        },
    );
}

/// Changes the first entry `hit` finds. Returns whether one was found.
pub(crate) fn update(
    app: &AppHandle,
    kind: FeedKind,
    hit: impl Fn(&FeedEntry) -> bool,
    f: impl FnOnce(&mut FeedEntry),
) -> bool {
    change(
        app,
        kind,
        |r| r.update(hit, f),
        |changed| {
            changed
                .iter()
                .map(|entry| KindEvent::Update { entry: entry.clone() })
                .collect()
        },
    )
    .flatten()
    .is_some()
}

/// Drops what `gone` names.
pub(crate) fn remove(app: &AppHandle, kind: FeedKind, gone: impl Fn(&FeedEntry) -> bool) {
    change(
        app,
        kind,
        |r| r.remove(gone),
        |dropped| dropped.iter().filter_map(|d| removed(Some(d))).collect(),
    );
}

pub(crate) fn clear(app: &AppHandle, kind: FeedKind) {
    change(app, kind, Ring::clear, |()| vec![KindEvent::Clear]);
}

/// Starts streaming a kind's changes. Returns the view id and the entries it has.
pub fn open(app: &AppHandle, kind: FeedKind, channel: View) -> FeedsOpened {
    app.state::<Store>().open(kind, channel)
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tauri::ipc::InvokeResponseBody;

    use super::*;

    fn entry(key: &str) -> FeedEntry {
        FeedEntry::Whisper(whispers::WhisperEntry {
            key: key.into(),
            ts: 0,
            thread_id: String::new(),
            login: String::new(),
            name: String::new(),
            avatar: None,
            text: String::new(),
        })
    }

    fn view(got: &Arc<Mutex<Vec<String>>>) -> View {
        let got = got.clone();
        Channel::new(move |body| {
            if let InvokeResponseBody::Json(s) = body {
                got.lock().unwrap().push(s);
            }
            Ok(())
        })
    }

    #[test]
    fn an_opened_view_gets_no_add_for_its_snapshot() {
        let store = Store::default();
        let kind = FeedKind::Whispers;
        let push = |e: FeedEntry| {
            store.change(kind, |r| r.push(e.clone(), |_| false), |_| vec![KindEvent::Add { entry: e.clone() }]);
        };
        push(entry("a"));
        let got = Arc::new(Mutex::new(Vec::new()));
        let opened = store.open(kind, view(&got));
        assert_eq!(opened.entries.iter().map(FeedEntry::key).collect::<Vec<_>>(), ["a"]);
        assert!(got.lock().unwrap().is_empty());
        push(entry("b"));
        let got = got.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert!(got[0].contains("\"b\"") && !got[0].contains("\"a\""));
    }
}
