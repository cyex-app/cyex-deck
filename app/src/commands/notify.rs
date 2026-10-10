//! IPC for the bell (1.26) and Twitch profile and notification settings (2.2).

use cyex_twitch::gql::notifications::List;
use cyex_twitch::gql::profile::Profile;
use tauri::AppHandle;

use crate::bell::{self, BellPage, BellState};
use crate::profile::{self, TwitchNotify};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn bell_state(app: AppHandle) -> BellState {
    bell::state(&app)
}

#[tauri::command]
pub async fn bell_list(app: AppHandle, list: List, cursor: Option<String>) -> CmdResult<BellPage> {
    bell::list(&app, list, cursor).await
}

#[tauri::command]
pub async fn bell_seen(app: AppHandle) -> CmdResult<()> {
    bell::seen(&app).await
}

#[tauri::command]
pub async fn bell_read(app: AppHandle, ids: Vec<String>) -> CmdResult<()> {
    bell::read(&app, ids).await
}

#[tauri::command]
pub async fn bell_read_all(app: AppHandle, list: List) -> CmdResult<()> {
    bell::read_all(&app, list).await
}

#[tauri::command]
pub async fn bell_delete(app: AppHandle, id: String) -> CmdResult<()> {
    bell::delete(&app, id).await
}

#[tauri::command]
pub async fn profile_get(app: AppHandle, account: String) -> CmdResult<Profile> {
    profile::get(&app, &account).await
}

#[tauri::command]
pub async fn profile_save(
    app: AppHandle,
    account: String,
    display_name: String,
    bio: String,
) -> CmdResult<()> {
    profile::save(&app, &account, &display_name, &bio).await
}

#[tauri::command]
pub async fn twitch_notify_get(
    app: AppHandle,
    account: String,
    cursor: Option<String>,
) -> CmdResult<TwitchNotify> {
    profile::notify_settings(&app, &account, cursor).await
}

#[tauri::command]
pub async fn twitch_notify_switch(
    app: AppHandle,
    account: String,
    category: String,
    platform: String,
    on: bool,
) -> CmdResult<()> {
    profile::set_switch(&app, &account, &category, &platform, on).await
}

#[tauri::command]
pub async fn twitch_notify_channel(
    app: AppHandle,
    account: String,
    channel_id: String,
    state: String,
) -> CmdResult<()> {
    profile::set_channel(&app, &account, &channel_id, &state).await
}
