//! V112-14/16 cyex.tv sign-in (Settings › Accounts) and the picker's cyex.tv side.

use cyex_emotes::cyextv::gql::Owned;
use tauri::AppHandle;

use crate::cyextv::{self, TvCosmetics, TvKind};

/// Resolves with the cyex.tv name; rejects with the line to show ('' when cancelled).
#[tauri::command]
pub async fn cyextv_sign_in(app: AppHandle, account: String) -> Result<String, String> {
    cyextv::sign_in(&app, &account).await
}

#[tauri::command]
pub fn cyextv_sign_in_cancel(app: AppHandle) {
    cyextv::cancel(&app);
}

#[tauri::command]
pub async fn cyextv_sign_out(app: AppHandle, account: String) {
    cyextv::sign_out(&app, &account).await;
}

#[tauri::command]
pub async fn cyextv_cosmetics(app: AppHandle, account: String) -> TvCosmetics {
    cyextv::view(&app, &account).await
}

/// Rejects with the line to show.
#[tauri::command]
pub async fn cyextv_select(app: AppHandle, account: String, kind: TvKind, id: Option<String>) -> Result<Owned, String> {
    cyextv::select(&app, &account, kind, id).await
}
