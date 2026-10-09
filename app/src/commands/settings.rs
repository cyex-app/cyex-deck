//! Settings commands (2.8, 2.9, X6).

use std::collections::BTreeMap;
use std::fs;

use cyex_settings::{Bundle, Clock, Partition, SystemClock, suggested_file_name};
use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use time::Date;
use time::macros::format_description;

use crate::settings::Settings;

type CmdResult<T> = Result<T, String>;

const UI_DATE: &[time::format_description::BorrowedFormatItem<'static>] =
    format_description!("[year]/[month]/[day]");
const FILE_FILTER: (&str, &[&str]) = ("cyex settings", &["json"]);

#[tauri::command]
pub fn settings_all(settings: State<Settings>) -> BTreeMap<Partition, Value> {
    settings.with(|s| s.all().clone())
}

#[tauri::command]
pub fn settings_patch(
    app: AppHandle,
    settings: State<Settings>,
    partition: Partition,
    patch: Value,
) -> CmdResult<Value> {
    settings.patch(&app, partition, &patch, true)
}

#[tauri::command]
pub fn settings_reset(app: AppHandle, settings: State<Settings>, partition: Partition) -> Value {
    settings.reset(&app, partition)
}

/// Opens a save dialog. Returns false if the user cancelled.
#[tauri::command]
pub async fn settings_export_file(
    app: AppHandle,
    settings: State<'_, Settings>,
    partitions: Vec<Partition>,
) -> CmdResult<bool> {
    if partitions.is_empty() {
        return Err("Pick at least one section".into());
    }
    let bundle = settings.with(|s| s.export(&partitions));
    let name = suggested_file_name(&partitions, SystemClock.now());
    let Some(target) = app
        .dialog()
        .file()
        .set_title("Export settings")
        .set_file_name(name)
        .add_filter(FILE_FILTER.0, FILE_FILTER.1)
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = target.into_path().map_err(|e| e.to_string())?;
    fs::write(path, bundle.to_json()).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Opens a file picker and imports the file. Empty result = cancelled.
#[tauri::command]
pub async fn settings_import_file(
    app: AppHandle,
    settings: State<'_, Settings>,
) -> CmdResult<Vec<Partition>> {
    let Some(source) = app
        .dialog()
        .file()
        .set_title("Import settings")
        .add_filter(FILE_FILTER.0, FILE_FILTER.1)
        .blocking_pick_file()
    else {
        return Ok(Vec::new());
    };
    let path = source.into_path().map_err(|e| e.to_string())?;
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let bundle = Bundle::from_json(&text).map_err(|e| e.to_string())?;
    settings.import(&app, bundle)
}

#[tauri::command]
pub fn settings_share_code(settings: State<Settings>, partitions: Vec<Partition>) -> CmdResult<String> {
    if partitions.is_empty() {
        return Err("Pick at least one section".into());
    }
    Ok(settings.with(|s| s.export(&partitions).to_share_code()))
}

#[tauri::command]
pub fn settings_import_code(
    app: AppHandle,
    settings: State<Settings>,
    code: String,
) -> CmdResult<Vec<Partition>> {
    let bundle = Bundle::from_share_code(&code).map_err(|e| e.to_string())?;
    settings.import(&app, bundle)
}

/// Backup dates as `yyyy/mm/dd`, newest first.
#[tauri::command]
pub fn settings_backups(settings: State<Settings>) -> CmdResult<Vec<String>> {
    let dates = settings.with(|s| s.backups()).map_err(|e| e.to_string())?;
    Ok(dates
        .into_iter()
        .map(|d| d.format(UI_DATE).expect("date formats"))
        .collect())
}

#[tauri::command]
pub fn settings_restore_backup(
    app: AppHandle,
    settings: State<Settings>,
    date: String,
) -> CmdResult<Vec<Partition>> {
    let date = Date::parse(&date, UI_DATE).map_err(|_| format!("\"{date}\" isn't a backup date"))?;
    settings.restore_backup(&app, date)
}

#[tauri::command]
pub fn settings_open_folder(app: AppHandle, settings: State<Settings>) -> CmdResult<()> {
    let dir = settings.with(|s| s.dir().to_path_buf());
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Partitions reset at startup because their file was unreadable. Returned once.
#[tauri::command]
pub fn settings_take_load_report(settings: State<Settings>) -> Vec<Partition> {
    settings.take_load_report()
}
