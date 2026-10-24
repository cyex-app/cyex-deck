//! V108-02 A sign-in in the app: a private, signed-out window (nothing kept after it closes),
//! so another account can sign in than the one the browser knows. Twitch's pages and the
//! sign-in's own redirect (127.0.0.1 only) load here; other links go to the browser.
//! V108-CX It runs in its own browser process and closes blank-first (`private`).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use cyex_core::color::Theme;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WindowEvent};
use tauri_plugin_opener::OpenerExt;

use crate::theme;

const PREFIX: &str = "signin-twitch";

/// V108-CX Each window gets a new label: one that is still closing (`private::close`) never
/// holds up, or gets reused by, the next sign-in.
static SEQ: AtomicU32 = AtomicU32::new(0);
static CURRENT: Mutex<Option<String>> = Mutex::new(None);

fn current() -> MutexGuard<'static, Option<String>> {
    CURRENT.lock().unwrap_or_else(|p| p.into_inner())
}

/// Shows `url` in the sign-in window, opening it when it isn't. Closing it cancels the sign-in.
pub fn open(app: &AppHandle, url: &str) -> Result<(), String> {
    let url: Url = url.parse().map_err(|_| "That isn’t Twitch’s sign-in page.")?;
    let open_one = current().clone().and_then(|l| app.get_webview_window(&l));
    if let Some(w) = open_one {
        w.navigate(url).map_err(|e| e.to_string())?;
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let label = format!("{PREFIX}-{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let look = theme::look(app);
    let nav_app = app.clone();
    let pop_app = app.clone();
    let window = super::private::builder(app, &label, WebviewUrl::External(url))
        .title("Sign in to Twitch")
        .inner_size(520.0, 760.0)
        .min_inner_size(360.0, 300.0)
        .initialization_script(super::twitch::skin(&look))
        .center()
        .theme(Some(match look.theme {
            Theme::Dark => tauri::Theme::Dark,
            Theme::Light => tauri::Theme::Light,
        }))
        .background_color(theme::background(app))
        .on_navigation(move |u| {
            let ok = allowed(u);
            if !ok && u.scheme() == "https" {
                let _ = nav_app.opener().open_url(u.as_str(), None::<&str>);
            }
            ok
        })
        .on_new_window(move |u, features| super::twitch::popup(&pop_app, u, features))
        .build()
        .map_err(|e| e.to_string())?;
    *current() = Some(label.clone());
    super::login::follow(app, label.clone(), (520.0, 760.0), std::time::Duration::from_secs(3600));
    let app = app.clone();
    window.on_window_event(move |e| {
        if let WindowEvent::CloseRequested { api, .. } = e {
            // The window goes our way (blank page first), not WebView2's.
            api.prevent_close();
            app.state::<crate::auth::Auth>().cancel_sign_in();
            forget(&label);
            if let Some(w) = app.get_webview_window(&label) {
                tauri::async_runtime::spawn(super::private::close(w));
            }
        }
    });
    Ok(())
}

/// V108-K1 The sign-in here went through for `account`: its Twitch login goes to the
/// account's Twitch pages profile (`twitch::adopt`) before the window closes, so 7TV's
/// sign-in and the Twitch pages don't ask again.
pub async fn keep_login(app: &AppHandle, account: &str) {
    let Some(w) = current().clone().and_then(|l| app.get_webview_window(&l)) else {
        return;
    };
    let Some(all) = crate::sys::devtools::call(&w, "Network.getAllCookies", "{}".to_owned()).await else {
        eprintln!("sign-in: couldn't read the window's Twitch login");
        return;
    };
    if !super::twitch::adopt(app, account, &all).await {
        eprintln!("sign-in: Twitch pages: login not kept");
    }
}

/// The sign-in ended: the window goes (without cancelling anything).
pub fn close(app: &AppHandle) {
    let Some(label) = current().take() else { return };
    if let Some(w) = app.get_webview_window(&label) {
        tauri::async_runtime::spawn(super::private::close(w));
    }
}

fn forget(label: &str) {
    let mut c = current();
    if c.as_deref() == Some(label) {
        *c = None;
    }
}

fn allowed(u: &Url) -> bool {
    let twitch = u.scheme() == "https" && u.host_str().is_some_and(|h| h == "twitch.tv" || h.ends_with(".twitch.tv"));
    twitch || is_redirect(u) || u.scheme() == "about"
}

/// The sign-in's redirect: `http://localhost:17563/…` or `http://127.0.0.1:17563/…`.
fn is_redirect(u: &Url) -> bool {
    u.scheme() == "http"
        && matches!(u.host_str(), Some("localhost" | "127.0.0.1"))
        && u.port() == Some(cyex_auth::browser::PORT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn twitch_and_the_redirect_only() {
        assert!(allowed(&u("https://id.twitch.tv/oauth2/authorize?x")));
        assert!(allowed(&u("https://www.twitch.tv/activate?device-code=A")));
        assert!(allowed(&u("http://localhost:17563/auth#access_token=a")));
        assert!(allowed(&u("http://127.0.0.1:17563/token")));
        assert!(!allowed(&u("http://localhost:17564/auth")));
        assert!(!allowed(&u("http://evil.example:17563/auth")));
        assert!(!allowed(&u("http://www.twitch.tv/activate")));
        assert!(!allowed(&u("https://twitch.tv.evil.example/")));
    }
}
