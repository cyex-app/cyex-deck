//! Chat font files the user adds (V101-10).

use cyex_settings::Partition;
use cyex_settings::schema::{CustomFont, FONT_TYPES, General};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::fonts::{self, FontChoice};
use crate::settings::Settings;

type CmdResult<T> = Result<T, String>;

/// Picks a font file, keeps a copy and makes it the chat font. Nothing when cancelled.
#[tauri::command]
pub async fn fonts_add(app: AppHandle, settings: State<'_, Settings>) -> CmdResult<Option<CustomFont>> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Pick a font file")
        .add_filter("Fonts", &FONT_TYPES)
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let font = fonts::add(&app, &path)?;
    settings.update::<General>(&app, Partition::General, |g| {
        g.chat_font_family = font.name.clone();
        g.fonts.push(font.clone());
        true
    })?;
    Ok(Some(font))
}

/// Forgets an added font and deletes its file; chat goes back to the app font if it used it.
#[tauri::command]
pub fn fonts_remove(app: AppHandle, settings: State<Settings>, name: String) -> CmdResult<()> {
    let g: General = settings.read(Partition::General);
    let Some(font) = g.fonts.iter().find(|f| f.name == name).cloned() else {
        return Ok(());
    };
    settings.update::<General>(&app, Partition::General, |g| {
        g.fonts.retain(|f| f.name != name);
        if g.chat_font_family == name {
            g.chat_font_family.clear();
        }
        true
    })?;
    fonts::remove_file(&app, &font);
    Ok(())
}

/// V105-13 The chat font list: bundled, added and installed fonts.
#[tauri::command]
pub fn fonts_list(settings: State<Settings>) -> Vec<FontChoice> {
    let g: General = settings.read(Partition::General);
    let added: Vec<String> = g.fonts.iter().map(|f| f.name.clone()).collect();
    fonts::choices(&added, &crate::sys::installed_fonts())
}
