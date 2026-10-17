//! Windows pop-ups for live alerts (2.2), whispers, Twitch's bell (1.26) and mentions
//! (V104-01). A click brings the main window up and tells it what to open. Toasts are
//! silent; the app plays its own sound when that's on (V1051-01). Windows only shows local
//! pictures, so avatars are cached in the local app data folder.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_winrt_notification::{IconCrop, Toast};
use ts_rs::TS;

use crate::windows::{self, MAIN};

/// The installer gives the Start menu shortcut this id (tauri.conf `identifier`), which
/// Windows needs to show an app's toasts.
const APP_ID: &str = cyex_core::app::ID;
const OPEN_EVENT: &str = "notify:open";
const AVATAR_MAX: usize = 512 * 1024;

/// What a click opens in the main window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export, rename = "NotifyOpen")]
pub enum Open {
    Channel {
        login: String,
    },
    Whisper {
        login: String,
    },
    /// V104-02 A global mention: that channel's `#channel` window (V102-05), read
    /// anonymously.
    Room {
        login: String,
    },
    /// Twitch's bell: the window decides between a channel, an in-app Twitch page and
    /// the browser.
    Link {
        url: String,
    },
}

pub struct Pop {
    pub title: String,
    pub text: String,
    /// Picture URL, shown round.
    pub avatar: Option<String>,
    pub open: Open,
}

/// Shows a toast. Pictures are fetched first, so this returns before it shows. Streamer
/// mode (U4) can keep them off the screen.
pub fn show(app: &AppHandle, pop: Pop) {
    if crate::streamer::hides(app, |s| s.no_notifications) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let avatar = match &pop.avatar {
            Some(url) => cached_avatar(&app, url).await,
            None => None,
        };
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let click_app = app.clone();
            let open = pop.open.clone();
            let mut toast = Toast::new(app_id())
                .title(&pop.title)
                .sound(None)
                .on_activated(move |_| {
                    windows::show_main(&click_app);
                    let _ = click_app.emit_to(MAIN, OPEN_EVENT, open.clone());
                    Ok(())
                });
            if !pop.text.is_empty() {
                toast = toast.text1(&pop.text);
            }
            if let Some(path) = &avatar {
                toast = toast.icon(path, IconCrop::Circular, "");
            }
            if let Err(e) = toast.show() {
                eprintln!("toasts: {e}");
            }
        })
        .await;
    });
}

/// V106-35 Whether the main window's taskbar button is flashing.
static FLASHING: AtomicBool = AtomicBool::new(false);

/// V106-35 (start, now flashing). In front: nothing (Q8). Already flashing: nothing, no restart.
#[cfg(test)]
fn on_request(flashing: bool, in_front: bool) -> (bool, bool) {
    if in_front || flashing {
        (false, flashing)
    } else {
        (true, true)
    }
}

/// (stop, now flashing). Any cyex window counts (Q7).
fn on_focus(flashing: bool) -> (bool, bool) {
    (flashing, false)
}

/// V106-35 Flashes the taskbar button until any cyex window comes to the front.
pub fn flash(app: &AppHandle) {
    if app_in_front(app) {
        return;
    }
    let Some(w) = app.get_webview_window(MAIN) else {
        return;
    };
    if FLASHING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }
    eprintln!("flash: start");
    crate::sys::flash::start(&w);
}

/// V106-35 A cyex window came to the front: stops the flash.
pub fn focused(app: &AppHandle) {
    let (stop, _) = on_focus(FLASHING.swap(false, Ordering::SeqCst));
    if !stop {
        return;
    }
    eprintln!("flash: stop");
    if let Some(w) = app.get_webview_window(MAIN) {
        crate::sys::flash::stop(&w);
    }
}

/// Whether the user is looking at the app right now.
pub fn app_in_front(app: &AppHandle) -> bool {
    app.webview_windows()
        .values()
        .any(|w| w.is_focused().unwrap_or(false))
}

/// A build run from `target\` has no Start menu shortcut, so Windows wouldn't know the
/// app's id; borrow PowerShell's, as Tauri's own notification plugin does.
fn app_id() -> &'static str {
    let dev = std::env::current_exe()
        .ok()
        .is_some_and(|p| p.components().any(|c| c.as_os_str() == "target"));
    if dev { Toast::POWERSHELL_APP_ID } else { APP_ID }
}

async fn cached_avatar(app: &AppHandle, url: &str) -> Option<PathBuf> {
    let dir = crate::paths::local(app).ok()?.join("toast-avatars");
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    let ext = if url.ends_with(".jpg") || url.ends_with(".jpeg") {
        "jpg"
    } else {
        "png"
    };
    let path = dir.join(format!("{:016x}.{ext}", h.finish()));
    if path.exists() {
        return Some(path);
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let bytes = http.get_bytes(url, AVATAR_MAX).await.ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::write(&path, bytes).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_starts_only_from_idle_and_out_of_front() {
        assert_eq!(on_request(false, true), (false, false), "in front: no flash (Q8)");
        assert_eq!(on_request(false, false), (true, true));
        assert_eq!(
            on_request(true, false),
            (false, true),
            "already flashing: no restart"
        );
        assert_eq!(on_request(true, true), (false, true));
    }

    #[test]
    fn focus_stops_a_flash_once() {
        assert_eq!(on_focus(true), (true, false));
        assert_eq!(on_focus(false), (false, false));
    }
}
