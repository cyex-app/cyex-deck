//! V109-B02 The cyex cosmetics area's commands (Settings → Accounts).

use tauri::AppHandle;

use crate::cyex::{self, CyexEquipped, CyexHistory, CyexView};

#[tauri::command]
pub async fn cyex_view(app: AppHandle, account: String) -> CyexView {
    cyex::view(&app, &account).await
}

/// Rejects with the message to show.
#[tauri::command]
pub async fn cyex_equip(app: AppHandle, account: String, main: Option<String>, custom: bool) -> Result<CyexEquipped, String> {
    cyex::equip(&app, &account, main, custom).await
}

/// What the code gave, as a line; rejects with the message to show.
#[tauri::command]
pub async fn cyex_redeem(app: AppHandle, account: String, code: String) -> Result<String, String> {
    cyex::redeem(&app, &account, &code).await
}

/// V109-B06 The Supporter+ uploads; rejects with the message to show.
#[tauri::command]
pub async fn cyex_badge_history(app: AppHandle, account: String) -> Result<CyexHistory, String> {
    cyex::history(&app, &account).await
}

/// V109-B06 Puts an earlier approved upload back live; rejects with the message to show.
#[tauri::command]
pub async fn cyex_badge_restore(app: AppHandle, account: String, id: u64) -> Result<(), String> {
    cyex::restore(&app, &account, id).await
}

/// V109-B07 An own upload's preview picture (PNG bytes); rejects with the message to show.
#[tauri::command]
pub async fn cyex_badge_preview(app: AppHandle, account: String, url: String) -> Result<tauri::ipc::Response, String> {
    cyex::preview(&app, &account, &url).await.map(tauri::ipc::Response::new)
}
