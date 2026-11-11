//! Theme commands (3.3, 3.4, U8).

use cyex_core::color::{AccentTokens, Theme, accent_tokens};
use cyex_settings::Partition;
use cyex_settings::schema::{Appearance, CustomTheme, MAX_THEMES, builtin_themes};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::settings::Settings;
use crate::theme::{self, ThemeLook};

use super::CmdResult;

#[tauri::command]
pub fn theme_accent_tokens(accent: String, theme: Theme) -> AccentTokens {
    accent_tokens(&accent, theme)
}

/// What the window paints: palette, accent and wallpaper of the theme in use.
#[tauri::command]
pub fn theme_look(app: AppHandle) -> ThemeLook {
    theme::look(&app)
}

/// V106-02 Midnight, Forest, Dusk, Ocean, Ember.
#[tauri::command]
pub fn theme_builtins() -> Vec<CustomTheme> {
    builtin_themes().to_vec()
}

/// U8 Adds a theme (a copy or a new one) with a fresh id and switches to it.
#[tauri::command]
pub fn theme_add(app: AppHandle, settings: State<Settings>, theme: CustomTheme) -> CmdResult<String> {
    add(&app, &settings, theme::fresh(theme))
}

/// Adds a theme that already has its fresh id (`theme::fresh`) and switches to it.
pub(super) fn add(app: &AppHandle, settings: &Settings, theme: CustomTheme) -> CmdResult<String> {
    let id = theme.id.clone();
    let mut full = false;
    settings.update::<Appearance>(app, Partition::Appearance, |a| {
        full = !a.add_theme(theme);
        !full
    })?;
    if full {
        return Err(format!("You have {MAX_THEMES} themes. Delete one to add another."));
    }
    Ok(id)
}

/// U8 The share code of one of your themes. V110-G2 A wallpaper file goes up to cyex.app first,
/// but only with `with_wallpaper` (the user said yes); without it the code has no wallpaper file.
#[tauri::command]
pub async fn theme_code(app: AppHandle, id: String, with_wallpaper: bool) -> CmdResult<crate::theme_share::CodeCopy> {
    crate::theme_share::code(&app, &id, with_wallpaper).await
}

/// U8 Picks a picture for a theme's wallpaper; returns its name in the app's folder,
/// or nothing when cancelled.
#[tauri::command]
pub async fn theme_pick_wallpaper(app: AppHandle) -> CmdResult<Option<String>> {
    // V110-F85 The picker blocks: off the runtime's workers.
    let asked = app.clone();
    let picked = tokio::task::spawn_blocking(move || {
        asked
            .dialog()
            .file()
            .set_title("Pick a wallpaper")
            .add_filter("Pictures", &["png", "jpg", "jpeg", "webp", "gif"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    theme::add_wallpaper(&app, &path).map(Some)
}
