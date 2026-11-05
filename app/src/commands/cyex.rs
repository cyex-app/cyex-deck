//! V109-B02 The cyex cosmetics area's commands (Settings → Accounts).

use tauri::AppHandle;

use crate::cyex::{self, CyexEquipped, CyexView};

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
