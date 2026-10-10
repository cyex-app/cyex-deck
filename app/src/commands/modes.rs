//! Streamer mode (U4), chat overlays (U7) and global hotkeys (S2).

use tauri::{AppHandle, WebviewWindow};

use crate::streamer::{self, StreamerState};
use crate::windows::overlay;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn streamer_state(app: AppHandle) -> StreamerState {
    streamer::state(&app)
}

/// Flips streamer mode until the streaming app starts or stops.
#[tauri::command]
pub fn streamer_toggle(app: AppHandle) -> StreamerState {
    streamer::toggle(&app);
    streamer::state(&app)
}

/// Whether the calling overlay lets clicks through.
#[tauri::command]
pub fn overlay_click_through(app: AppHandle, window: WebviewWindow) -> bool {
    overlay::is_click_through(&app, window.label())
}

#[tauri::command]
pub fn overlay_set_click_through(app: AppHandle, window: WebviewWindow, on: bool) -> CmdResult<()> {
    overlay::set_click_through(&app, window.label(), on)
}

/// Global combos Windows refused because another app holds them.
#[tauri::command]
pub fn hotkeys_failed(app: AppHandle) -> Vec<String> {
    crate::hotkeys::failed(&app)
}
