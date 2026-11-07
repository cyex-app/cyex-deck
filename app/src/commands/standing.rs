//! V108-C40 IPC for the locked chat box: the account's standing in a channel, asking for an
//! unban and acknowledging a warning.

use tauri::AppHandle;

use crate::chat::known::Known;
use crate::chat::standing;

/// The chat account's ban or timeout in the channel, with the unban request rules, folded
/// into what the app knew (V109-F79).
#[tauri::command]
pub async fn chat_standing(app: AppHandle, account: String, channel_id: String) -> Result<Known, String> {
    standing::read(&app, &account, &channel_id).await
}

/// V109-F79 What the app knows of it without asking Twitch.
#[tauri::command]
pub fn chat_standing_known(account: String, channel_id: String) -> Known {
    standing::known(&account, &channel_id)
}

/// Only the Send button of the unban request form calls this.
#[tauri::command]
pub async fn chat_request_unban(app: AppHandle, account: String, channel_id: String, message: String) -> Result<(), String> {
    standing::request_unban(&app, &account, &channel_id, &message).await
}

/// Only the Acknowledge button calls this.
#[tauri::command]
pub async fn chat_ack_warning(app: AppHandle, account: String, channel_id: String) -> Result<(), String> {
    standing::acknowledge(&app, &account, &channel_id).await
}

/// Follows the account's own moderation topic while one of its chat boxes is open.
#[tauri::command]
pub fn chat_standing_watch(app: AppHandle, window: tauri::WebviewWindow, account: String, on: bool) {
    standing::watch(&app, window.label(), &account, on);
}

/// Test builds only: a made-up standing for one chat (see `standing::mock::set`).
#[cfg(feature = "mocks")]
#[tauri::command]
pub fn dev_standing(app: AppHandle, account: String, channel_id: String, kind: String) -> Result<(), String> {
    standing::mock::set(&app, &account, &channel_id, &kind)
}
