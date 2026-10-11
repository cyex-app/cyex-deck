//! V102-02 Where the app keeps its files: `%APPDATA%\cyex-deck` (settings, plugins,
//! recordings and the rest) and `%LOCALAPPDATA%\cyex-deck` (the Twitch window's profiles,
//! toast pictures). Both are copied once from the old name's folders.

use std::path::PathBuf;

use cyex_core::app::{OLD_SLUG, SLUG};
use cyex_settings::migrate::{self, Copied};
use tauri::{AppHandle, Manager};

/// Browser caches in the Twitch window's profiles: they refill on their own, so they
/// aren't copied. Sign-ins and cookies are.
const CACHES: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "GraphiteDawnCache",
    "GrShaderCache",
    "ShaderCache",
    "Crashpad",
];

/// `%APPDATA%\cyex-deck`.
pub fn config(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().config_dir()?.join(SLUG))
}

/// `%LOCALAPPDATA%\cyex-deck`.
pub fn local(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().local_data_dir()?.join(SLUG))
}

/// Runs first on every start; does something only on the first start under the new name.
/// The main window's own browser storage (Tauri's folder named after the bundle id) isn't
/// copied: it's mostly cache and holds only the theme it starts with and the emote menu's
/// last page, which come back on their own.
pub fn migrate(app: &AppHandle) {
    let path = app.path();
    for (base, skip) in [(path.config_dir(), &[][..]), (path.local_data_dir(), CACHES)] {
        let Ok(base) = base else { continue };
        match migrate::copy_once(&base.join(OLD_SLUG), &base.join(SLUG), skip) {
            Ok(Copied::Yes) => eprintln!("paths: copied {} from the old folder", base.join(SLUG).display()),
            Ok(_) => {}
            Err(e) => eprintln!(
                "paths: couldn't copy the old folder into {}: {e}",
                base.join(SLUG).display()
            ),
        }
    }
}

/// Runs on every start, after `migrate` (which may have just copied it): deletes
/// `%APPDATA%\cyex-deck\.env`, unused since the client ID is built in (V103-01). Only that
/// file; `%APPDATA%\cyex-viewer` is never touched.
pub fn remove_old_env(app: &AppHandle) {
    let Ok(dir) = config(app) else { return };
    match migrate::remove_old_env(&dir) {
        Ok(true) => eprintln!("paths: deleted the old .env"),
        Ok(false) => {}
        Err(e) => eprintln!("paths: couldn't delete the old .env: {e}"),
    }
}
