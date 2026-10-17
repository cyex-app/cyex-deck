//! V106-07 Feed tabs of their own kind: live notifications, AutoMod, whispers.

use cyex_settings::schema::FeedKind;
use tauri::AppHandle;
use tauri::ipc::Channel;

use crate::feeds::{self, FeedsOpened, KindEvent};

/// Streams one kind's entries to a feed tab.
#[tauri::command]
pub fn feeds_open(app: AppHandle, kind: FeedKind, on_event: Channel<Vec<KindEvent>>) -> FeedsOpened {
    feeds::open(&app, kind, on_event)
}

#[tauri::command]
pub fn feeds_close(app: AppHandle, view: u32) {
    feeds::close(&app, view);
}

/// Q4 The feed header's Clear.
#[tauri::command]
pub fn feeds_clear(app: AppHandle, kind: FeedKind) {
    feeds::clear(&app, kind);
}
