//! V112-14/16 cyex.tv sign-in (Settings › Accounts) and the picker's cyex.tv side.

use cyex_emotes::cyextv::gql::Owned;
use tauri::AppHandle;

use crate::chat::run::CyextvMenuOptions;
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

/// V112-17 What the emote menu offers for a cyex.tv emote in this chat's channel.
#[tauri::command]
pub async fn cyextv_emote_options(app: AppHandle, account: String, login: String, emote_id: String) -> CyextvMenuOptions {
    crate::chat::run::cyextv_menu_options(&app, &account, &login, &emote_id).await
}

/// V112-17 Adds the emote to this chat's channel under its own name. Rejects with the line to show.
#[tauri::command]
pub async fn cyextv_emote_add(app: AppHandle, account: String, login: String, emote_id: String) -> Result<String, String> {
    crate::chat::run::cyextv_menu_add(&app, &account, &login, &emote_id).await
}
