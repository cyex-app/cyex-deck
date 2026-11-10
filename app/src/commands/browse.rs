//! Home and discover commands (1.6, 1.7).

use cyex_twitch::gql::{self, Category, CategoryStreams, ChannelCard, Page, SearchResults, StreamFilter};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::auth::Auth;
use crate::browse::schedule::{self, ChannelSchedule, NextStream};
use crate::browse::{discover, follows};

use super::CmdResult;

/// 1.6 Channels the account follows. `force` fetches the whole list again.
#[tauri::command]
pub async fn home_follows(app: AppHandle, account: String, force: bool) -> CmdResult<Vec<ChannelCard>> {
    follows::follows(&app, &account, force).await
}

/// H2 Channels by user id, e.g. favorites. Public.
#[tauri::command]
pub async fn home_channels(app: AppHandle, ids: Vec<String>) -> CmdResult<Vec<ChannelCard>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let http = app.state::<Auth>().http.clone();
    gql::cards(&http, &ids).await.map_err(follows::error_text)
}

#[tauri::command]
pub async fn discover_streams(
    app: AppHandle,
    filter: StreamFilter,
    after: Option<String>,
) -> CmdResult<Page<ChannelCard>> {
    discover::streams(&app, &filter, after.as_deref()).await
}

#[tauri::command]
pub async fn discover_categories(app: AppHandle, after: Option<String>) -> CmdResult<Page<Category>> {
    discover::categories(&app, after.as_deref()).await
}

#[tauri::command]
pub async fn discover_category(
    app: AppHandle,
    slug: String,
    filter: StreamFilter,
    after: Option<String>,
) -> CmdResult<Option<CategoryStreams>> {
    discover::category(&app, &slug, &filter, after.as_deref()).await
}

#[tauri::command]
pub async fn discover_search(app: AppHandle, text: String) -> CmdResult<SearchResults> {
    discover::search(&app, text.trim()).await
}

#[tauri::command]
pub async fn discover_search_more(
    app: AppHandle,
    text: String,
    after: String,
) -> CmdResult<Page<ChannelCard>> {
    discover::search_more(&app, text.trim(), &after).await
}

/// H4
#[tauri::command]
pub async fn discover_followed_categories(app: AppHandle, account: String) -> CmdResult<Vec<Category>> {
    discover::followed_categories(&app, &account).await
}

/// V108-N03 A channel's upcoming streams and vacation. Public.
#[tauri::command]
pub async fn schedule_channel(app: AppHandle, login: String) -> CmdResult<ChannelSchedule> {
    schedule::channel(&app, &login).await
}

/// V108-N03 The next planned stream of each channel that has one, by user id. Never fails:
/// what couldn't be read is left out.
#[tauri::command]
pub async fn schedule_next(app: AppHandle, ids: Vec<String>) -> Vec<NextStream> {
    schedule::next(&app, ids).await
}

/// V108-N03 Saves a channel's schedule as an .ics file through a save dialog. `None`:
/// cancelled, or nothing planned; else the path.
#[tauri::command]
pub async fn schedule_save_ics(app: AppHandle, login: String, name: String) -> CmdResult<Option<String>> {
    let Some(text) = schedule::ics_text(&app, &login, &name).await? else {
        return Err("Nothing planned to save.".into());
    };
    let file = format!("{}-schedule.ics", login.trim().to_ascii_lowercase());
    let asked = app.clone();
    let picked = tokio::task::spawn_blocking(move || {
        asked
            .dialog()
            .file()
            .set_title("Save schedule")
            .set_file_name(file)
            .add_filter("Calendar", &["ics"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(target) = picked else { return Ok(None) };
    let path = target.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}
