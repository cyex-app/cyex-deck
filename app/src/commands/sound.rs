//! V1051-01 Settings → Sound and the app's own sounds.

use tauri::AppHandle;
use tauri::ipc::InvokeBody;

use crate::sound::{self, SoundDevices};

type CmdResult<T> = Result<T, String>;

/// Async, so listing the devices never holds the window's thread.
#[tauri::command]
pub async fn sound_devices() -> SoundDevices {
    // The enumeration waits on a COM thread: not on a runtime worker.
    tauri::async_runtime::spawn_blocking(sound::devices).await.unwrap_or_default()
}

/// V1051-01 The page asks before it picks a speaker (`on`) and tells right after (`off`):
/// WebView2 lets it use the microphone only in between. Answers once WebView2 has it.
#[tauri::command]
pub async fn sound_permit(app: AppHandle, on: bool) -> CmdResult<()> {
    sound::permit(&app, on).await
}

/// A sound file from your settings, or the chime (no path): the Play and test buttons.
#[tauri::command]
pub fn sound_play(app: AppHandle, path: Option<String>) -> CmdResult<()> {
    if let Some(p) = &path
        && !sound::named(&app, p)
    {
        return Err("Not a sound from your settings.".into());
    }
    sound::preview(&app, path.as_deref());
    Ok(())
}

/// The page's decoded sound: raw f32 body, the request's id in `x-id`. Empty: it wouldn't decode.
// V1051-01 final review: off the UI thread (up to 11 MB of samples or a 5 MB file).
#[tauri::command(async)]
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
