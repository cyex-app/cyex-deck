//! V1051-02 7TV's own Twitch sign-in in a small private window. The app types nothing: the user
//! signs in on Twitch's page; when 7TV's page has its session we copy it out and close.
//! The window shares the app's WebView2 environment but is InPrivate and has no app IPC.
//! For "both" (Twitch and 7TV in one go) it borrows the account's Twitch web profile instead,
//! so Twitch is signed in already, and every 7TV trace is cleared from it before it closes.

use std::time::Duration;

use cyex_auth::SevenTvToken;
use cyex_core::color::Theme;
use serde::Serialize;
use tauri::webview::{NewWindowResponse, WebviewWindow};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WindowEvent};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::mpsc::{self, error::TryRecvError};
use ts_rs::TS;

use super::twitch;
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
#[cfg(feature = "mocks")]
fn mock_of(login_url: &str) -> Option<String> {
    let origin = crate::mock_url::loopback_origin(&Url::parse(login_url).ok()?)?;
    origin.strip_prefix("http://").map(str::to_owned)
}

/// R10 No mock outside a mocks build.
#[cfg(not(feature = "mocks"))]
fn mock_of(_: &str) -> Option<String> {
    None
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
fn callback(url: &Url, mock: Option<&str>) -> Option<Callback> {
    let host_ok = (url.scheme() == "https" && url.host_str() == Some("7tv.app")) || on_mock(url, mock);
    if !host_ok || url.path() != "/login/callback" {
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

/// Where 7TV's own data lives in the page: what the clear covers.
fn origins(mock: Option<&str>) -> Vec<String> {
    match mock {
        Some(m) => vec![format!("http://{m}")],
        None => vec!["https://7tv.app".into(), "https://api.7tv.app".into()],
    }
}

/// A 7TV cookie: on 7tv.app or below it (or the mock's host).
fn is_7tv_domain(domain: &str, mock: Option<&str>) -> bool {
    let d = domain.trim_start_matches('.');
    d == "7tv.app" || d.ends_with(".7tv.app") || mock.is_some_and(|m| m.split(':').next() == Some(d))
}

/// The 7TV cookies left in `Storage.getCookies`' answer as (name, domain, path), or `None`
/// when the answer can't be read.
fn leftover(json: &str, mock: Option<&str>) -> Option<Vec<(String, String, String)>> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let field = |c: &serde_json::Value, k: &str| c.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_owned();
    Some(
        v.get("cookies")?
            .as_array()?
            .iter()
            .filter(|c| is_7tv_domain(&field(c, "domain"), mock))
            .map(|c| (field(c, "name"), field(c, "domain"), field(c, "path")))
            .collect(),
    )
}

fn label_of(account: &str) -> String {
    let safe: String = account.chars().filter(char::is_ascii_alphanumeric).collect();
    format!("{PREFIX}{safe}")
}

/// Opens the sign-in window and returns 7TV's session once the user is through, or why not.
/// `shared`: in the account's Twitch web profile (one Twitch login for both), else private.
pub async fn capture(app: &AppHandle, account: &str, login_url: String, shared: bool) -> Result<SevenTvToken, SignIn> {
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
    let nav_mock = mock.clone();
    let pop_app = app.clone();
    let look = theme::look(app);
    let profile = if shared { twitch::profile_dir(app, account) } else { None };
    let builder = match &profile {
        // Same flags as the Twitch window (none), so both can share its folder at once.
        Some(dir) => tauri::WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url)).data_directory(dir.clone()),
        None => super::environment(app, &label, WebviewUrl::External(url)).incognito(true),
    };
    let window = builder
        .title("Sign in to 7TV")
        .inner_size(480.0, 720.0)
        .min_inner_size(420.0, 560.0)
        .center()
        .theme(Some(match look.theme {
            Theme::Dark => tauri::Theme::Dark,
            Theme::Light => tauri::Theme::Light,
        }))
        .background_color(theme::background(app))
        .on_navigation(move |u| {
            let ok = allowed(u, nav_mock.as_deref());
            if ok && let Some(c) = callback(u, nav_mock.as_deref()) {
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
    // A page that calls `window.close()` loses its webview, but the window stays and sends
    // no `Destroyed`: WebView2's close request says so instead.
    let closing = tx.clone();
    crate::sys::on_page_close(&window, move || {
        let _ = closing.send(Event::Closed);
    });
    window.on_window_event(move |e| {
        if matches!(e, WindowEvent::Destroyed) {
            let _ = tx.send(Event::Closed);
        }
    });

    let result = wait(&window, &mut rx, mock.as_deref()).await;
    if profile.is_some() && !clean(&window, mock.as_deref()).await {
        // Not sure 7TV is gone: the whole profile goes, Twitch's login with it.
        eprintln!("seventv: couldn't clear 7TV from the Twitch profile; clearing all of it");
        let _ = window.clear_all_browsing_data();
    }
    // `destroy`, not `close`: it frees the label even when the page is already gone.
    let _ = window.destroy();
    result
}

/// Waits for 7TV's callback, then looks for the session for up to `POLL_FOR`. Closing the
/// window, or the page closing itself, is a cancel.
async fn wait(
    window: &WebviewWindow,
    rx: &mut mpsc::UnboundedReceiver<Event>,
    mock: Option<&str>,
) -> Result<SevenTvToken, SignIn> {
    let mut until: Option<tokio::time::Instant> = None;
    loop {
        loop {
            match rx.try_recv() {
                Ok(Event::Callback(Callback::Code)) => {
                    until.get_or_insert_with(|| tokio::time::Instant::now() + POLL_FOR);
                }
                Ok(Event::Callback(Callback::Error) | Event::Closed) | Err(TryRecvError::Disconnected) => {
                    return Err(SignIn::Cancelled);
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        if let Some(until) = until {
            if tokio::time::Instant::now() >= until {
                return Err(SignIn::Failed);
            }
            if on_7tv(window, mock)
                && let Some(raw) = crate::sys::run_script(window, "localStorage.getItem('7tv-token')").await
                && let Ok(Some(jwt)) = serde_json::from_str::<Option<String>>(&raw)
            {
                let _ = crate::sys::run_script(window, "localStorage.removeItem('7tv-token')").await;
                return SevenTvToken::parse(&jwt).ok_or(SignIn::Failed);
            }
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
}

/// Clears 7TV's origins (all storage types, cookies too) in a shared profile and checks that
/// no 7TV cookie or localStorage key is left. `false`: it couldn't be done or checked.
async fn clean(window: &WebviewWindow, mock: Option<&str>) -> bool {
    for o in origins(mock) {
        let params = serde_json::json!({ "origin": o, "storageTypes": "all" }).to_string();
        if crate::sys::devtools::call(window, "Storage.clearDataForOrigin", params).await.is_none() {
            return false;
        }
    }
    for _ in 0..2 {
        let Some(left) = crate::sys::devtools::call(window, "Storage.getCookies", "{}".into()).await.and_then(|j| leftover(&j, mock)) else {
            return false;
        };
        // The page may still be on 7TV and write again; its own view of localStorage must be empty.
        let storage_empty = !on_7tv(window, mock)
            || crate::sys::run_script(window, "localStorage.length").await.as_deref() == Some("0");
        if left.is_empty() && storage_empty {
            return true;
        }
        for (name, domain, path) in left {
            let params = serde_json::json!({ "name": name, "domain": domain, "path": path }).to_string();
            let _ = crate::sys::devtools::call(window, "Network.deleteCookies", params).await;
        }
        if on_7tv(window, mock) {
            let _ = crate::sys::run_script(window, "localStorage.clear()").await;
        }
    }
    false
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
            callback(&u("https://7tv.app/login/callback?platform=twitch&code=x"), None),
            Some(Callback::Code)
        );
        assert_eq!(
            callback(&u(
                "https://7tv.app/login/callback?platform=twitch&error=access_denied"
            ), None),
            Some(Callback::Error)
        );
        assert_eq!(
            callback(&u("https://7tv.app/login/callback?platform=twitch"), None),
            None
        );
        assert_eq!(callback(&u("https://7tv.app/?code=x"), None), None);
        // Another host with the same path is not 7TV's callback.
        assert_eq!(callback(&u("https://evil.example/login/callback?code=x"), None), None);
        assert_eq!(callback(&u("https://id.twitch.tv/login/callback?code=x"), None), None);
        let mock = Some("127.0.0.1:5199");
        assert_eq!(callback(&u("http://127.0.0.1:5199/login/callback?code=x"), mock), Some(Callback::Code));
        assert_eq!(callback(&u("http://127.0.0.1:5200/login/callback?code=x"), mock), None);
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
    fn the_clear_covers_7tv_only() {
        assert_eq!(origins(None), ["https://7tv.app", "https://api.7tv.app"]);
        assert_eq!(origins(Some("127.0.0.1:5199")), ["http://127.0.0.1:5199"]);
        let json = r#"{"cookies":[
            {"name":"seventv-verifier","domain":"api.7tv.app","path":"/"},
            {"name":"x","domain":".7tv.app","path":"/"},
            {"name":"auth-token","domain":".twitch.tv","path":"/"},
            {"name":"y","domain":"evil7tv.app","path":"/"}
        ]}"#;
        let left = leftover(json, None).unwrap();
        assert_eq!(left.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(), ["seventv-verifier", "x"]);
        // Twitch's login stays; nothing of 7TV's left reads as clean.
        assert_eq!(leftover(r#"{"cookies":[{"name":"auth-token","domain":".twitch.tv","path":"/"}]}"#, None), Some(vec![]));
        assert_eq!(leftover("not json", None), None);
        assert_eq!(leftover(r#"{"other":1}"#, None), None);
        let mock = r#"{"cookies":[{"name":"m","domain":"127.0.0.1","path":"/"}]}"#;
        assert_eq!(leftover(mock, Some("127.0.0.1:5199")).map(|l| l.len()), Some(1));
        assert_eq!(leftover(mock, None).map(|l| l.len()), Some(0));
    }

    #[cfg(feature = "mocks")]
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
        assert_eq!(mock_of("http://127.0.0.1:5199@evil.example/login"), None);
    }
}
