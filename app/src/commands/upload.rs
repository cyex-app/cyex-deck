//! V105-11 Image uploads from the chat box.

use tauri::ipc::{Channel, InvokeBody};
use tauri::{AppHandle, State};

use crate::upload::{self, UploadEvent, UploadItem, Uploads};

type CmdResult<T> = Result<T, String>;

/// Pasted image data (raw body, extension in the `x-ext` header). Returns an id for `upload_images`.
#[tauri::command]
pub fn upload_stage(state: State<Uploads>, request: tauri::ipc::Request<'_>) -> CmdResult<String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("Only PNG, JPG, GIF and WebP images.".into());
    };
    let ext = request
        .headers()
        .get("x-ext")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    upload::stage(&state, bytes.clone(), ext)
}

#[tauri::command]
pub async fn upload_images(
    app: AppHandle,
    channel: String,
    items: Vec<UploadItem>,
    on_event: Channel<UploadEvent>,
) {
    upload::run(app, channel, items, on_event).await;
}
