//! Settings commands (2.8, 2.9, X6, V1041-02).

use std::collections::BTreeMap;
use std::fs;

use cyex_settings::chatterino::ImportReport;
use cyex_settings::schema::{Commands, Highlights};
use cyex_settings::{Bundle, Clock, Partition, SystemClock, suggested_file_name};
use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use time::Date;
use time::macros::format_description;

use crate::auth;
use crate::settings::Settings;
use crate::tabs::import as chatterino;

type CmdResult<T> = Result<T, String>;

const UI_DATE: &[time::format_description::BorrowedFormatItem<'static>] =
    format_description!("[year]/[month]/[day]");
const FILE_FILTER: (&str, &[&str]) = ("cyex settings", &["json"]);
/// V1041-02 `window-layout.json` and the 9 backups Chatterino keeps next to it.
const LAYOUT_FILTER: (&str, &[&str]) = (
    "Chatterino layout",
    &["json", "bkp-1", "bkp-2", "bkp-3", "bkp-4", "bkp-5", "bkp-6", "bkp-7", "bkp-8", "bkp-9"],
);

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
    rust_owned(partition)?;
    settings.patch(&app, partition, &patch, true)
}

/// V105-11 The uploader's headers come from the credential store; writing them goes through
/// `settings_patch` (`headers`), which keeps them out of the settings files.
#[tauri::command]
pub fn uploader_headers(settings: State<Settings>) -> String {
    settings.uploader_headers()
}

/// V105-12 The settings page was left: empty rules go. Returns the partitions it looks at.
#[tauri::command]
pub fn settings_prune_blank(app: AppHandle, settings: State<Settings>) -> CmdResult<BTreeMap<Partition, Value>> {
    settings.update::<Highlights>(&app, Partition::Highlights, Highlights::prune_blank)?;
    settings.update::<Commands>(&app, Partition::Commands, Commands::prune_blank)?;
    Ok(settings.with(|s| {
        [Partition::Highlights, Partition::Commands]
            .into_iter()
            .map(|p| (p, s.get(p).clone()))
            .collect()
    }))
}

#[tauri::command]
pub fn settings_reset(app: AppHandle, settings: State<Settings>, partition: Partition) -> CmdResult<Value> {
    rust_owned(partition)?;
    Ok(settings.reset(&app, partition))
}

/// The accounts list points at stored tokens, so only the account manager changes it.
fn rust_owned(partition: Partition) -> CmdResult<()> {
    if partition == Partition::Accounts {
        return Err("Accounts are changed in the account manager.".into());
    }
    Ok(())
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

/// V1041-02 Picks a Chatterino layout and looks its channels up. `None` = cancelled.
#[tauri::command]
pub async fn settings_chatterino_preview(app: AppHandle) -> CmdResult<Option<ImportReport>> {
    // Said before the picker opens, not after.
    chatterino::account(&app)?;
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Import from Chatterino")
        .add_filter(LAYOUT_FILTER.0, LAYOUT_FILTER.1)
        .add_filter("All files", &["*"]);
    if let Some(dir) = chatterino::folder(&app) {
        dialog = dialog.set_directory(dir);
    }
    let Some(source) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let path = source.into_path().map_err(|e| e.to_string())?;
    chatterino::preview(&app, &path).await.map(Some)
}

/// V1041-02 Adds the previewed tabs: chat only, or chat and the stream (`player`).
#[tauri::command]
pub fn settings_chatterino_import(app: AppHandle, player: bool) -> CmdResult<ImportReport> {
    chatterino::confirm(&app, player)
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

/// Backup dates as `yyyy/mm/dd`, newest first, then the `before-restore-…` snapshots.
#[tauri::command]
pub fn settings_backups(settings: State<Settings>) -> CmdResult<Vec<String>> {
    let dates = settings.with(|s| s.backups()).map_err(|e| e.to_string())?;
    let snaps = settings.with(|s| s.snapshots()).map_err(|e| e.to_string())?;
    Ok(dates
        .into_iter()
        .map(|d| d.format(UI_DATE).expect("date formats"))
        .chain(snaps)
        .collect())
}

/// V1052-03 Writes the current settings to a `before-restore-…` folder (no secrets, last 3 kept).
#[tauri::command]
pub fn settings_snapshot_now(settings: State<Settings>) -> CmdResult<Option<String>> {
    settings.snapshot_now()
}

#[tauri::command]
pub fn settings_restore_backup(
    app: AppHandle,
    settings: State<Settings>,
    date: String,
) -> CmdResult<Vec<Partition>> {
    let restored = if date.starts_with("before-restore-") {
        settings.restore_snapshot(&app, &date)?
    } else {
        let date = Date::parse(&date, UI_DATE).map_err(|_| format!("\"{date}\" isn’t a backup date"))?;
        settings.restore_backup(&app, date)?
    };
    if restored.contains(&Partition::Accounts) {
        auth::reload(&app);
    }
    Ok(restored)
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
