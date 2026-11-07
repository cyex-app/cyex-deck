//! V107-R39 IPC for the stream manager's Clips and Videos panels (your own channel).

use cyex_twitch::gql::VideoKind;
use cyex_twitch::gql::dashboard::content::clips::{ClipFilter, ClipPage};
use cyex_twitch::gql::dashboard::content::videos::VideoPage;
use tauri::AppHandle;

use crate::streammanager::bulk::Bulk;
use crate::streammanager::content;

#[tauri::command]
pub async fn content_clips(app: AppHandle, filter: ClipFilter) -> Result<ClipPage, String> {
    content::clips(&app, filter).await
}

#[tauri::command]
pub async fn content_delete_clips(app: AppHandle, slugs: Vec<String>) -> Result<Bulk, String> {
    content::delete_clips(&app, slugs).await
}

#[tauri::command]
pub async fn content_videos(
    app: AppHandle,
    account: Option<String>,
    kind: Option<VideoKind>,
    cursor: Option<String>,
) -> Result<VideoPage, String> {
    content::videos(&app, account.as_deref(), kind, cursor).await
}

#[tauri::command]
pub async fn content_rename_video(app: AppHandle, account: Option<String>, id: String, title: String) -> Result<String, String> {
    content::rename_video(&app, account.as_deref(), id, title).await
}

#[tauri::command]
pub async fn content_publish_video(app: AppHandle, account: Option<String>, id: String, public: bool) -> Result<bool, String> {
    content::publish_video(&app, account.as_deref(), id, public).await
}

#[tauri::command]
pub async fn content_publish_videos(app: AppHandle, account: Option<String>, ids: Vec<String>, public: bool) -> Result<Bulk, String> {
    content::publish_videos(&app, account.as_deref(), ids, public).await
}

#[tauri::command]
pub async fn content_delete_videos(app: AppHandle, account: Option<String>, ids: Vec<String>) -> Result<Bulk, String> {
    content::delete_videos(&app, account.as_deref(), ids).await
}
