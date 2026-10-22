//! V106-07 Feed tabs of their own kind: live notifications, AutoMod, whispers.

use cyex_settings::schema::FeedKind;
use tauri::{AppHandle, WebviewWindow};
use tauri::ipc::Channel;

use crate::feeds::{self, FeedsOpened, KindEvent};

/// Streams one kind's entries to a feed tab.
#[tauri::command]
pub fn feeds_open(app: AppHandle, window: WebviewWindow, kind: FeedKind, on_event: Channel<Vec<KindEvent>>) -> FeedsOpened {
    feeds::open(&app, kind, window.label(), on_event)
}

#[tauri::command]
pub fn feeds_close(app: AppHandle, view: u32) {
    feeds::close(&app, view);
}

/// Q4 The feed header's Clear. V107-N8a With `keys`: only the rows the tab hid, not what arrived since.
#[tauri::command]
pub fn feeds_clear(app: AppHandle, kind: FeedKind, keys: Option<Vec<String>>) {
    match keys {
        Some(keys) => feeds::remove(&app, kind, |e| keys.iter().any(|k| k == e.key())),
        None => feeds::clear(&app, kind),
    }
}
