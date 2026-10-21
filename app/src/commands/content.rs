//! V107-R39 IPC for the stream manager's Clips and Videos panels (your own channel).

use cyex_twitch::gql::VideoKind;
use cyex_twitch::gql::dashboard::content::clips::{ClipFilter, ClipPage};
use cyex_twitch::gql::dashboard::content::videos::VideoPage;
use tauri::AppHandle;

use crate::streammanager::content::{self, Deleted};

#[tauri::command]
pub async fn content_clips(app: AppHandle, filter: ClipFilter) -> Result<ClipPage, String> {
    content::clips(&app, filter).await
}

#[tauri::command]
pub async fn content_delete_clips(app: AppHandle, slugs: Vec<String>) -> Result<Deleted, String> {
    content::delete_clips(&app, slugs).await
}

#[tauri::command]
pub async fn content_videos(app: AppHandle, kind: Option<VideoKind>, cursor: Option<String>) -> Result<VideoPage, String> {
    content::videos(&app, kind, cursor).await
}

#[tauri::command]
pub async fn content_rename_video(app: AppHandle, id: String, title: String) -> Result<String, String> {
    content::rename_video(&app, id, title).await
}

#[tauri::command]
pub async fn content_publish_video(app: AppHandle, id: String, public: bool) -> Result<bool, String> {
    content::publish_video(&app, id, public).await
}

#[tauri::command]
pub async fn content_publish_videos(app: AppHandle, ids: Vec<String>, public: bool) -> Result<Deleted, String> {
    content::publish_videos(&app, ids, public).await
}

#[tauri::command]
pub async fn content_delete_videos(app: AppHandle, ids: Vec<String>) -> Result<Deleted, String> {
    content::delete_videos(&app, ids).await
}
