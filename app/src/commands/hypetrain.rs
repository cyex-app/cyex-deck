//! V108-N04 IPC for the hype train bar over chat.

use cyex_twitch::gql::hypetrain::Train;
use tauri::AppHandle;

use crate::chat::hypetrain;

/// The channel's running hype train, if any.
#[tauri::command]
pub async fn chat_hype_train(app: AppHandle, login: String) -> Result<Option<Train>, String> {
    hypetrain::read(&app, &login).await
}

/// Follows the train's topic while a chat of a live channel shows the bar.
#[tauri::command]
pub fn chat_hype_watch(app: AppHandle, window: tauri::WebviewWindow, channel_id: String, on: bool) {
    hypetrain::watch(&app, window.label(), &channel_id, on);
}
