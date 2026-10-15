//! V1051-02 7TV's own Twitch sign-in in a small private window. The app types nothing: the user
//! signs in on Twitch's page; when 7TV's page has its session we copy it out and close.
//! The window shares the app's WebView2 environment but is InPrivate and has no app IPC.

use std::time::Duration;

use cyex_auth::SevenTvToken;
use cyex_core::color::Theme;
use serde::Serialize;
use tauri::webview::{NewWindowResponse, WebviewWindow};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WindowEvent};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::mpsc;
use ts_rs::TS;

use crate::theme;

const PREFIX: &str = "seventv-";
const HOSTS: [&str; 6] = [
    "api.7tv.app",
    "7tv.app",
    "id.twitch.tv",
    "www.twitch.tv",
    "twitch.tv",
    "passport.twitch.tv",
];
/// How long we look for the session after 7TV's callback.
const POLL_FOR: Duration = Duration::from_secs(15);
const POLL_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Serialize, TS, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum SignIn {
    Done,
    Cancelled,
    OtherAccount,
    Failed,
}

#[derive(Debug, PartialEq, Eq)]
enum Callback {
    Code,
    Error,
}

enum Event {
    Callback(Callback),
    Closed,
}

/// `host:port` of a local mock (`http://127.0.0.1:<port>`), the only plain-http place allowed.
fn mock_of(login_url: &str) -> Option<String> {
    let u = Url::parse(login_url).ok()?;
    if u.scheme() != "http" || u.host_str() != Some("127.0.0.1") {
        return None;
    }
    Some(format!("127.0.0.1:{}", u.port()?))
}

fn on_mock(url: &Url, mock: Option<&str>) -> bool {
    let Some(mock) = mock else { return false };
    url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port().is_some_and(|p| format!("127.0.0.1:{p}") == mock)
}

/// Whether the window may show `url`.
fn allowed(url: &Url, mock: Option<&str>) -> bool {
    url.scheme() == "about"
        || on_mock(url, mock)
        || (url.scheme() == "https" && url.host_str().is_some_and(|h| HOSTS.contains(&h)))
}

/// 7TV's landing page after Twitch: with a code the session follows; with an error it's over.
fn callback(url: &Url) -> Option<Callback> {
    if url.path() != "/login/callback" {
        return None;
    }
    let has = |k: &str| url.query_pairs().any(|(n, _)| n == k);
    if has("error") {
        Some(Callback::Error)
    } else if has("code") {
        Some(Callback::Code)
    } else {
        None
    }
}

fn on_7tv(w: &WebviewWindow, mock: Option<&str>) -> bool {
    w.url()
        .is_ok_and(|u| u.host_str() == Some("7tv.app") || on_mock(&u, mock))
}

fn label_of(account: &str) -> String {
    let safe: String = account.chars().filter(char::is_ascii_alphanumeric).collect();
    format!("{PREFIX}{safe}")
}

/// Opens the sign-in window and returns 7TV's session once the user is through, or why not.
pub async fn capture(app: &AppHandle, account: &str, login_url: String) -> Result<SevenTvToken, SignIn> {
    let label = label_of(account);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Err(SignIn::Cancelled);
    }
    let url = Url::parse(&login_url).map_err(|_| SignIn::Failed)?;
    let mock = mock_of(&login_url);
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let nav_tx = tx.clone();
    let nav_app = app.clone();
    let nav_mock = mock.clone();
    let pop_app = app.clone();
    let look = theme::look(app);
    let window = super::environment(app, &label, WebviewUrl::External(url))
        .title("Sign in to 7TV")
        .inner_size(480.0, 720.0)
        .min_inner_size(420.0, 560.0)
        .center()
        .theme(Some(match look.theme {
            Theme::Dark => tauri::Theme::Dark,
            Theme::Light => tauri::Theme::Light,
        }))
        .background_color(theme::background(app))
        .incognito(true)
        .on_navigation(move |u| {
            let ok = allowed(u, nav_mock.as_deref());
            if !ok && u.scheme() == "https" {
                let _ = nav_app.opener().open_url(u.as_str(), None::<&str>);
            }
            if ok && let Some(c) = callback(u) {
                let _ = nav_tx.send(Event::Callback(c));
            }
            ok
        })
        .on_new_window(move |u, _| {
            if u.scheme() == "https" {
                let _ = pop_app.opener().open_url(u.as_str(), None::<&str>);
            }
            NewWindowResponse::Deny
        })
        .build()
        .map_err(|e| {
            eprintln!("seventv: sign-in window: {e}");
            SignIn::Failed
        })?;
    window.on_window_event(move |e| {
        if matches!(e, WindowEvent::Destroyed) {
            let _ = tx.send(Event::Closed);
        }
    });

    let result = wait(&window, &mut rx, mock.as_deref()).await;
    let _ = window.close();
    result
}

async fn wait(
    window: &WebviewWindow,
    rx: &mut mpsc::UnboundedReceiver<Event>,
    mock: Option<&str>,
) -> Result<SevenTvToken, SignIn> {
    if !matches!(rx.recv().await, Some(Event::Callback(Callback::Code))) {
        return Err(SignIn::Cancelled);
    }
    let start = tokio::time::Instant::now();
    while start.elapsed() < POLL_FOR {
        if matches!(rx.try_recv(), Ok(Event::Closed)) {
            return Err(SignIn::Cancelled);
        }
        if on_7tv(window, mock)
            && let Some(raw) = crate::sys::run_script(window, "localStorage.getItem('7tv-token')").await
            && let Ok(Some(jwt)) = serde_json::from_str::<Option<String>>(&raw)
        {
            let _ = crate::sys::run_script(window, "localStorage.removeItem('7tv-token')").await;
            return SevenTvToken::parse(&jwt).ok_or(SignIn::Failed);
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
    Err(SignIn::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn callback_with_code_or_error() {
        assert_eq!(
            callback(&u("https://7tv.app/login/callback?platform=twitch&code=x")),
            Some(Callback::Code)
        );
        assert_eq!(
            callback(&u(
                "https://7tv.app/login/callback?platform=twitch&error=access_denied"
            )),
            Some(Callback::Error)
        );
        assert_eq!(
            callback(&u("https://7tv.app/login/callback?platform=twitch")),
            None
        );
        assert_eq!(callback(&u("https://7tv.app/?code=x")), None);
    }

    #[test]
    fn allowlist() {
        assert!(allowed(&u("https://7tv.app/login/callback?code=x"), None));
        assert!(allowed(
            &u("https://id.twitch.tv/oauth2/authorize?client_id=a"),
            None
        ));
        assert!(allowed(&u("about:blank"), None));
        assert!(!allowed(&u("http://7tv.app/login/callback"), None));
        assert!(!allowed(&u("https://evil.example/"), None));
        assert!(!allowed(&u("https://7tv.app.evil.example/"), None));
        assert!(!allowed(&u("http://127.0.0.1:5199/"), None));
    }

    #[test]
    fn mock_is_only_its_own_port() {
        let mock = mock_of("http://127.0.0.1:5199/v4/auth/login?platform=twitch");
        assert_eq!(mock.as_deref(), Some("127.0.0.1:5199"));
        assert!(allowed(
            &u("http://127.0.0.1:5199/login/callback?code=x"),
            mock.as_deref()
        ));
        assert!(!allowed(&u("http://127.0.0.1:5200/"), mock.as_deref()));
        assert_eq!(mock_of("https://api.7tv.app/v4/auth/login"), None);
    }
}
