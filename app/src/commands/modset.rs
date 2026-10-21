//! V107-R38 IPC for your channel's moderation settings (the stream manager's panel).

use cyex_twitch::gql::dashboard::modset::{Change, ModSettings};
use tauri::AppHandle;

use crate::streammanager::modset;

#[tauri::command]
pub async fn modset_read(app: AppHandle) -> Result<ModSettings, String> {
    modset::read(&app).await
}

#[tauri::command]
pub async fn modset_set(app: AppHandle, change: Change) -> Result<ModSettings, String> {
    modset::set(&app, change).await
}

#[tauri::command]
pub async fn modset_add_term(app: AppHandle, permitted: bool, text: String) -> Result<ModSettings, String> {
    modset::add_term(&app, permitted, &text).await
}

#[tauri::command]
pub async fn modset_remove_term(app: AppHandle, permitted: bool, id: String) -> Result<ModSettings, String> {
    modset::remove_term(&app, permitted, &id).await
}
