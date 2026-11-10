//! V109-T01 Theme links on cyex.app. Logic in `crate::theme_share`.

use tauri::{AppHandle, State};

use crate::settings::Settings;
use crate::theme_share::{self, LinkUpdate, ThemePreview};

use super::CmdResult;

/// Shares a theme and returns its link (the same link when it's shared already).
#[tauri::command]
pub async fn theme_share(app: AppHandle, id: String) -> CmdResult<String> {
    theme_share::share(&app, &id).await
}

/// "Update shared theme": returns the new version.
#[tauri::command]
pub async fn theme_share_update(app: AppHandle, id: String) -> CmdResult<u32> {
    theme_share::update_share(&app, &id).await
}

#[tauri::command]
pub async fn theme_share_stop(app: AppHandle, id: String) -> CmdResult<()> {
    theme_share::stop_share(&app, &id).await
}

/// True when this PC has the edit key of the theme's link.
#[tauri::command]
pub fn theme_share_can_edit(app: AppHandle, id: String) -> bool {
    theme_share::can_edit(&app, &id)
}

/// A pasted link or theme code, to show before adding.
#[tauri::command]
pub async fn theme_preview(app: AppHandle, text: String) -> CmdResult<ThemePreview> {
    theme_share::preview(&app, &text).await
}

/// Adds the previewed theme and switches to it; returns its id.
#[tauri::command]
pub fn theme_add_preview(app: AppHandle, settings: State<Settings>, preview: ThemePreview) -> CmdResult<String> {
    super::theme::add(&app, &settings, theme_share::from_preview(preview))
}

#[tauri::command]
pub async fn theme_report(app: AppHandle, source_id: String, reason: String) -> CmdResult<()> {
    theme_share::report(&app, &source_id, &reason).await
}

/// "Update" a theme added from a link. `replace` after the user said yes to losing their changes.
#[tauri::command]
pub async fn theme_link_update(app: AppHandle, id: String, replace: bool) -> CmdResult<LinkUpdate> {
    theme_share::update_from_link(&app, &id, replace).await
}

/// Go back to an earlier version (`index` in the theme's history).
#[tauri::command]
pub fn theme_link_restore(app: AppHandle, id: String, index: usize) -> CmdResult<()> {
    theme_share::restore(&app, &id, index)
}

#[tauri::command]
pub fn theme_link_ignore(app: AppHandle, id: String) -> CmdResult<()> {
    theme_share::ignore(&app, &id)
}
