//! Theme commands (3.3, 3.4, U8).

use cyex_core::color::{AccentTokens, Theme, accent_tokens};
use cyex_settings::Partition;
use cyex_settings::schema::{Appearance, CustomTheme, preset_themes};
use cyex_settings::theme_code;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::settings::Settings;
use crate::theme::{self, ThemeLook};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn theme_accent_tokens(accent: String, theme: Theme) -> AccentTokens {
    accent_tokens(&accent, theme)
}

/// What the window paints: palette, accent and wallpaper of the theme in use.
#[tauri::command]
pub fn theme_look(app: AppHandle) -> ThemeLook {
    theme::look(&app)
}

/// U8 Themes to start from in the theme editor.
#[tauri::command]
pub fn theme_presets() -> Vec<CustomTheme> {
    preset_themes()
}

/// U8 Adds a theme (a preset, a copy or a new one) with a fresh id and switches to it.
#[tauri::command]
pub fn theme_add(app: AppHandle, settings: State<Settings>, theme: CustomTheme) -> CmdResult<String> {
    let theme = theme::fresh(theme);
    let id = theme.id.clone();
    settings.update::<Appearance>(&app, Partition::Appearance, |a| {
        a.themes.push(theme);
        a.theme_id = id.clone();
        true
    })?;
    Ok(id)
}

/// U8 The share code of one of your themes.
#[tauri::command]
pub fn theme_code(settings: State<Settings>, id: String) -> CmdResult<String> {
    let a: Appearance = settings.read(Partition::Appearance);
    let t = a
        .themes
        .iter()
        .find(|t| t.id == id)
        .ok_or("That theme is gone.")?;
    Ok(theme_code::to_code(t))
}

/// U8 Adds the theme in a share code and switches to it.
#[tauri::command]
pub fn theme_import(app: AppHandle, settings: State<Settings>, code: String) -> CmdResult<String> {
    let theme = theme_code::from_code(&code).map_err(|_| "That isn’t a theme code.".to_owned())?;
    theme_add(app, settings, theme)
}

/// U8 Picks a picture for a theme's wallpaper; returns its name in the app's folder,
/// or nothing when cancelled.
#[tauri::command]
pub async fn theme_pick_wallpaper(app: AppHandle) -> CmdResult<Option<String>> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Pick a wallpaper")
        .add_filter("Pictures", &["png", "jpg", "jpeg", "webp", "gif"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    theme::add_wallpaper(&app, &path).map(Some)
}
