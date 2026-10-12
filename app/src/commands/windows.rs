//! Window commands (1.51, 3.9, U2).

use serde::Serialize;
use tauri::{AppHandle, WebviewWindow};
use ts_rs::TS;

use crate::windows::{self, MAIN};

type CmdResult<T> = Result<T, String>;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WindowInfo {
    pub is_main: bool,
    /// Panel route, for pop-outs.
    pub route: Option<String>,
    pub pinned: bool,
}

#[tauri::command]
pub fn window_info(app: AppHandle, window: WebviewWindow) -> WindowInfo {
    let label = window.label();
    WindowInfo {
        is_main: label == MAIN,
        route: windows::route_of(&app, label),
        pinned: windows::is_pinned(&app, label),
    }
}

/// Pins or unpins the calling window (always on top).
#[tauri::command]
pub fn window_set_pinned(app: AppHandle, window: WebviewWindow, pinned: bool) -> CmdResult<bool> {
    windows::set_pinned(&app, &window.as_ref().window(), pinned)?;
    Ok(pinned)
}

/// The calling window's title, as the taskbar shows it.
#[tauri::command]
pub fn window_set_title(window: WebviewWindow, title: String) -> CmdResult<()> {
    window.set_title(&title).map_err(|e| e.to_string())
}

/// Routes of the open pop-outs. Changes arrive as `panels:changed`.
#[tauri::command]
pub fn window_panels(app: AppHandle) -> Vec<String> {
    windows::panel_routes(&app)
}

/// 3.14 Closes a pop-out; its panel docks back into the main window.
#[tauri::command]
pub fn window_close_panel(app: AppHandle, route: String) -> CmdResult<()> {
    windows::close_panel(&app, &route).map_err(|e| e.to_string())
}

/// Async on purpose: creating a window from a sync command deadlocks WebView2 on Windows.
#[tauri::command]
pub async fn window_open_panel(
    app: AppHandle,
    route: String,
    title: String,
    width: Option<f64>,
    height: Option<f64>,
) -> CmdResult<()> {
    let size = width.zip(height);
    windows::open_panel(&app, &route, &title, size).map_err(|e| e.to_string())
}

/// 1.36 Async for the same reason as `window_open_panel`.
#[tauri::command]
pub async fn window_open_usercard(app: AppHandle, channel: String, login: String) -> CmdResult<()> {
    let valid = |s: &str| (1..=25).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    if !valid(&channel) || !valid(&login) {
        return Err("That isn't a Twitch name.".into());
    }
    windows::open_usercard(&app, &channel.to_ascii_lowercase(), &login.to_ascii_lowercase()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn layout_save(app: AppHandle, name: String) -> CmdResult<()> {
    windows::save_layout(&app, &name)
}

/// Async because it may open windows (see `window_open_panel`).
#[tauri::command]
pub async fn layout_apply(app: AppHandle, name: String) -> CmdResult<()> {
    windows::apply_layout(&app, &name)
}

#[tauri::command]
pub fn layout_delete(app: AppHandle, name: String) -> CmdResult<()> {
    windows::delete_layout(&app, &name)
}
