//! V107-R19 Profile picture, banner and offline image.

use tauri::AppHandle;
use tauri::ipc::InvokeBody;

use crate::profile_images::{self as images, ImageError, ImageErrorKind, ProfileImageKind, ProfileImagesView};

type ImageResult<T> = Result<T, ImageError>;

#[tauri::command]
pub async fn profile_images_get(app: AppHandle, account: String) -> ImageResult<ProfileImagesView> {
    images::get(&app, &account).await
}

/// The file as the raw body; `x-account` and `x-kind` (`avatar`, `banner`, `offline`) headers.
#[tauri::command]
pub async fn profile_images_change(app: AppHandle, request: tauri::ipc::Request<'_>) -> ImageResult<ProfileImagesView> {
    let header = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).unwrap_or("").to_owned();
    let kind: ProfileImageKind = serde_json::from_value(serde_json::Value::String(header("x-kind")))
        .map_err(|_| ImageError::new(ImageErrorKind::Failed))?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(ImageError::new(ImageErrorKind::Format));
    };
    images::change(&app, &header("x-account"), kind, bytes.clone()).await
}

#[tauri::command]
pub async fn profile_images_restore(app: AppHandle, account: String, kind: ProfileImageKind) -> ImageResult<ProfileImagesView> {
    images::restore(&app, &account, kind).await
}

/// Whether this run talks to the local mock (mocks builds on test data only).
#[tauri::command]
pub fn profile_images_mocked(app: AppHandle) -> bool {
    images::mock_base(&app).is_some()
}
