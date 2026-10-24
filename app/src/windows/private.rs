//! V108-CX Private (signed-out) windows: the Twitch sign-in and 7TV's private sign-in.
//! Twitch's login page asks for passkeys as it loads. Closing an InPrivate page while that
//! request is open crashes its whole WebView2 browser process (seen 2026/10/24, with and
//! without the debug port). So these windows run in a WebView2 folder of their own (their
//! own browser process, never the main window's), and close by going to `about:blank` first.

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry};

/// The private windows' WebView2 folder, beside the app's data. InPrivate leaves nothing in
/// it but the browser's own housekeeping.
const FOLDER: &str = "webview-private";
/// Time for `about:blank` to replace the page (and drop its passkey request).
const BLANK_FOR: Duration = Duration::from_millis(500);

static CLOSING: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn closing() -> MutexGuard<'static, Vec<String>> {
    CLOSING.lock().unwrap_or_else(|p| p.into_inner())
}

/// An InPrivate window in the private browser process, with the app's usual flags.
pub fn builder<'a>(
    app: &'a AppHandle,
    label: &'a str,
    url: WebviewUrl,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    match crate::paths::local(app) {
        Ok(dir) => WebviewWindowBuilder::new(app, label, url)
            .additional_browser_args(super::BROWSER_ARGS)
            .data_directory(dir.join(FOLDER))
            .incognito(true),
        // No folder of its own: still private, in the app's browser.
        Err(_) => super::environment(app, label, url).incognito(true),
    }
}

/// Whether `label` is on its way out (`close` is running for it).
pub fn is_closing(label: &str) -> bool {
    closing().iter().any(|l| l == label)
}

/// Hides the window at once, blanks its page, then destroys it. Never kills a process.
pub async fn close(w: WebviewWindow) {
    let label = w.label().to_owned();
    if is_closing(&label) {
        return;
    }
    closing().push(label.clone());
    let _ = w.hide();
    if w.navigate("about:blank".parse().expect("url")).is_ok() {
        tokio::time::sleep(BLANK_FOR).await;
    }
    let _ = w.destroy();
    closing().retain(|l| l != &label);
}

/// Waits (up to 3 s) for a closing window `label` to be gone, so its label is free again.
pub async fn wait_gone(app: &AppHandle, label: &str) {
    use tauri::Manager;
    for _ in 0..60 {
        if !is_closing(label) && app.get_webview_window(label).is_none() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_labels_are_tracked() {
        assert!(!is_closing("t-a"));
        closing().push("t-a".into());
        assert!(is_closing("t-a") && !is_closing("t-b"));
        closing().retain(|l| l != "t-a");
        assert!(!is_closing("t-a"));
    }
}
