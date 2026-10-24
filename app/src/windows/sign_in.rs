//! V108-02 A sign-in in the app: a private, signed-out window (nothing kept after it closes),
//! so another account can sign in than the one the browser knows. Twitch's pages and the
//! sign-in's own redirect (127.0.0.1 only) load here; other links go to the browser.

use cyex_core::color::Theme;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WindowEvent};
use tauri_plugin_opener::OpenerExt;

use crate::theme;

const LABEL: &str = "signin-twitch";

/// Shows `url` in the sign-in window, opening it when it isn't. Closing it cancels the sign-in.
pub fn open(app: &AppHandle, url: &str) -> Result<(), String> {
    let url: Url = url.parse().map_err(|_| "That isn’t Twitch’s sign-in page.")?;
    if let Some(w) = app.get_webview_window(LABEL) {
        w.navigate(url).map_err(|e| e.to_string())?;
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let look = theme::look(app);
    let nav_app = app.clone();
    let pop_app = app.clone();
    let window = super::environment(app, LABEL, WebviewUrl::External(url))
        .incognito(true)
        .title("Sign in to Twitch")
        .inner_size(520.0, 760.0)
        .min_inner_size(420.0, 560.0)
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
    let app = app.clone();
    window.on_window_event(move |e| {
        if matches!(e, WindowEvent::CloseRequested { .. }) {
            app.state::<crate::auth::Auth>().cancel_sign_in();
        }
    });
    Ok(())
}

/// The sign-in ended: the window goes (without cancelling anything).
pub fn close(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.destroy();
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
