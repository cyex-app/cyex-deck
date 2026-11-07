//! V109-C03 IPC for the clips manager: an account's own clips and its channel's clips.

use cyex_twitch::gql::dashboard::content::clips::{ClipFilter, ClipPage};
use tauri::AppHandle;

use crate::clips;
use crate::streammanager::bulk::Bulk;

#[tauri::command]
pub async fn clips_list(app: AppHandle, account: String, filter: ClipFilter) -> Result<ClipPage, String> {
    clips::list(&app, &account, filter).await
}

#[tauri::command]
pub async fn clips_delete(app: AppHandle, account: String, slugs: Vec<String>) -> Result<Bulk, String> {
    clips::delete(&app, &account, slugs).await
}

/// V109-C05 Renames one clip; the title Twitch kept.
#[tauri::command]
pub async fn clips_rename(app: AppHandle, account: String, slug: String, title: String) -> Result<String, String> {
    clips::rename(&app, &account, &slug, &title).await
}

/// True: Twitch's editor opened in the account's Twitch window; false: a mock run.
#[tauri::command]
pub async fn clips_edit(app: AppHandle, account: String, slug: String) -> Result<bool, String> {
    clips::edit(&app, &account, &slug).await
}
