//! The `X-Device-Id` playback tokens are asked with: random, kept for this install the
//! way a browser keeps it. Not a secret.

use std::fs;

use tauri::{AppHandle, Manager};

pub fn id(app: &AppHandle) -> String {
    let path = app.path().config_dir().map(|d| d.join("cyex-viewer").join("device-id"));
    if let Ok(p) = &path
        && let Ok(saved) = fs::read_to_string(p)
        && valid(saved.trim())
    {
        return saved.trim().to_owned();
    }
    let id = cyex_twitch::random::hex(16);
    if let Ok(p) = path {
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Err(e) = fs::write(&p, &id) {
            eprintln!("player: couldn't save the device id: {e}");
        }
    }
    id
}

fn valid(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_digit() || b.is_ascii_lowercase())
}
