//! V110-U43 IPC for the raid banner over chat.

use cyex_twitch::gql::dashboard::raids::RaidStatus;
use tauri::AppHandle;

use crate::chat::raid;

/// The raid counting down from the channel, if any.
#[tauri::command]
pub fn chat_raid(channel_id: String) -> Option<RaidStatus> {
    raid::current(&channel_id)
}

/// Follows the channel's raid topic while a chat of it shows.
#[tauri::command]
pub fn chat_raid_watch(app: AppHandle, window: tauri::WebviewWindow, channel_id: String, on: bool) {
    raid::watch(&app, window.label(), &channel_id, on);
}

/// Joins the raid as the chat's account. Only from the banner's tick, never by itself.
#[tauri::command]
pub async fn chat_raid_join(app: AppHandle, account: String, raid_id: String) -> Result<(), String> {
    raid::join(&app, &account, &raid_id).await
}

/// Join unticked before the raid went: only the pop-up is dropped.
#[tauri::command]
pub fn chat_raid_leave(app: AppHandle, raid_id: String) {
    raid::leave(&app, &raid_id);
}

/// Raid ids with Join ticked, for a chat that opens mid-raid.
#[tauri::command]
pub fn chat_raid_ticked() -> Vec<String> {
    raid::ticked()
}
