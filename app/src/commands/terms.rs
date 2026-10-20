//! IPC for the mod view's blocked terms (1.48, V107-R31).

use cyex_twitch::helix::moderation::BlockedTerm;
use tauri::AppHandle;

use crate::moderation::terms;

#[tauri::command]
pub async fn mod_blocked_terms(app: AppHandle, account: String, channel: String) -> Result<Vec<BlockedTerm>, String> {
    terms::list(&app, &account, &channel).await
}

#[tauri::command]
pub async fn mod_add_blocked_term(app: AppHandle, account: String, channel: String, text: String) -> Result<BlockedTerm, String> {
    terms::add(&app, &account, &channel, &text).await
}

#[tauri::command]
pub async fn mod_remove_blocked_term(app: AppHandle, account: String, channel: String, id: String) -> Result<(), String> {
    terms::remove(&app, &account, &channel, &id).await
}
