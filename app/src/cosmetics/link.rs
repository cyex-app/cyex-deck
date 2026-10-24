//! V108-7L "Send to cyex deck": the bookmark clicked on 7tv.app hands its session to the
//! listener on 127.0.0.1:17563, bound only while the user waits for it on an account card.
//! Same checks and vault entry as "Paste a 7TV token". The nonce is per install, in the
//! local data folder (never in exports or backups).

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use cyex_auth::AuthError;
use cyex_auth::browser::{self, link};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use super::paste::{Refused, check_token, kept};
use crate::auth::{self, Auth};

const FILE: &str = "seventv-link.txt";
const WAIT_FOR: Duration = Duration::from_secs(5 * 60);
pub const DONE: &str = "Connected to 7TV. You can close this tab.";

/// The one wait running: its cancel switch.
#[derive(Default)]
struct Waiting(Mutex<Option<watch::Sender<bool>>>);

fn waiting(app: &AppHandle) -> tauri::State<'_, Waiting> {
    app.manage(Waiting::default());
    app.state::<Waiting>()
}

/// Reads the install's nonce, or makes and saves one.
pub(super) fn nonce_in(dir: &Path) -> Result<String, String> {
    let path = dir.join(FILE);
    if let Ok(n) = std::fs::read_to_string(&path) {
        let n = n.trim();
        if n.len() == 32 && n.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(n.to_owned());
        }
    }
    let n = browser::new_state();
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, &n).map_err(|e| e.to_string())?;
    Ok(n)
}

fn nonce(app: &AppHandle) -> Result<String, String> {
    nonce_in(&crate::paths::local(app).map_err(|e| e.to_string())?)
}

/// The bookmark's address for this install.
pub fn bookmarklet(app: &AppHandle) -> Result<String, String> {
    Ok(link::bookmarklet(&nonce(app)?))
}

/// Waits (up to 5 minutes) for the bookmark, checks what it sent and keeps it. A new wait
/// stops the one before. Rejects with the message to show.
pub async fn wait(app: &AppHandle, account: &str) -> Result<(), String> {
    if !super::has_consent(app, account) {
        return Err(Refused::NoConsent.text().into());
    }
    let nonce = nonce(app)?;
    stop(app);
    // The wait before lets the port go within its 100 ms poll.
    let mut bound = browser::Listener::bind();
    for _ in 0..5 {
        if !matches!(bound, Err(AuthError::PortBusy(_))) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        bound = browser::Listener::bind();
    }
    let listener = bound.map_err(|e| e.to_string())?;
    let (tx, rx) = watch::channel(false);
    *waiting(app).0.lock().unwrap_or_else(|p| p.into_inner()) = Some(tx);
    let sent = tauri::async_runtime::spawn_blocking(move || listener.wait_7tv(&nonce, WAIT_FOR, || *rx.borrow()))
        .await
        .map_err(|e| e.to_string())?
        // Stopped: nothing to say.
        .map_err(|e| if matches!(e, AuthError::Cancelled) { String::new() } else { e.to_string() })?;
    let api = super::api(app);
    let checked = check_token(&api, &app.state::<Auth>().seventv, account, sent.token.clone(), auth::unix_now())
        .await
        .and_then(|id| kept(app, account, id));
    let text = match &checked {
        Ok(()) => DONE,
        Err(r) => r.text(),
    };
    tauri::async_runtime::spawn_blocking(move || sent.answer(text));
    checked.map_err(|r| r.text().to_owned())
}

/// Stops the wait, if one runs; the port closes with it.
pub fn stop(app: &AppHandle) {
    if let Some(tx) = waiting(app).0.lock().unwrap_or_else(|p| p.into_inner()).take() {
        let _ = tx.send(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_nonce_per_install() {
        let dir = std::env::temp_dir().join(format!("cyex-7tv-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = nonce_in(&dir).unwrap();
        assert_eq!(a.len(), 32);
        assert_eq!(nonce_in(&dir).unwrap(), a, "kept");
        std::fs::write(dir.join(FILE), "short").unwrap();
        let b = nonce_in(&dir).unwrap();
        assert!(b.len() == 32 && b != "short", "a damaged one is replaced");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
