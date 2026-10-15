//! V1051-01 Settings → Sound and the app's own sounds.

use cyex_settings::schema::OutputDevice;
use tauri::AppHandle;
use tauri::ipc::InvokeBody;

use crate::sound;

type CmdResult<T> = Result<T, String>;

/// Async, so listing the devices never holds the window's thread.
#[tauri::command]
pub async fn sound_devices() -> Vec<OutputDevice> {
    sound::devices()
}

/// A sound file from your settings, or the chime (no path): the Play and test buttons.
#[tauri::command]
pub fn sound_play(app: AppHandle, path: Option<String>) -> CmdResult<()> {
    if let Some(p) = &path
        && !sound::named(&app, p)
    {
        return Err("Not a sound from your settings.".into());
    }
    sound::alert(&app, path.as_deref());
    Ok(())
}

/// The page's decoded sound: raw f32 body, the request's id in `x-id`. Empty: it wouldn't decode.
#[tauri::command]
pub fn sound_decoded(app: AppHandle, request: tauri::ipc::Request<'_>) -> CmdResult<()> {
    let id = request
        .headers()
        .get("x-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .ok_or("No sound id.")?;
    let bytes: &[u8] = match request.body() {
        InvokeBody::Raw(b) => b,
        _ => &[],
    };
    sound::decoded(&app, id, bytes)
}
