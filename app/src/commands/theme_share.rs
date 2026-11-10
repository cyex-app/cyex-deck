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

/// V110-A7 A theme added from a preview: its id, and why it came without its wallpaper if it did.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedTheme {
    id: String,
    note: Option<String>,
}

/// Adds the previewed theme and switches to it.
#[tauri::command]
pub async fn theme_add_preview(app: AppHandle, settings: State<'_, Settings>, preview: ThemePreview) -> CmdResult<AddedTheme> {
    let from_code = preview.source_id.is_empty();
    let mut theme = theme_share::from_preview(preview);
    let note = theme_share::localize_wallpaper(&app, &mut theme, from_code).await;
    let id = super::theme::add(&app, &settings, theme)?;
    Ok(AddedTheme { id, note })
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
