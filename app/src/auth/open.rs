//! V108-02/03 Where a sign-in page opens: the browser used last (else the default one), or
//! a fresh, signed-out app window. Only Twitch's sign-in page and its code page open here.

use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;
use url::Url;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Place {
    #[default]
    Browser,
    /// A new signed-out profile, so another account can sign in.
    App,
}

/// Twitch's sign-in page for this app, or twitch.tv/activate (with the code filled in).
pub fn allowed(url: &str) -> bool {
    cyex_auth::browser::is_authorize_url(url) || is_activation(url)
}

pub fn is_activation(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && matches!(u.host_str(), Some("www.twitch.tv" | "twitch.tv"))
            && u.port().is_none()
            && u.path().trim_end_matches('/') == "/activate"
    })
}

pub fn open(app: &AppHandle, place: Place, url: &str) -> Result<(), String> {
    if !allowed(url) {
        return Err("That isn’t Twitch’s sign-in page.".into());
    }
    match place {
        Place::App => crate::windows::sign_in::open(app, url),
        Place::Browser => {
            if crate::sys::browsers::open_in_last_browser(url) {
                return Ok(());
            }
            app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_twitch_sign_in_pages() {
        assert!(allowed("https://www.twitch.tv/activate?device-code=ABCDEFGH"));
        assert!(allowed("https://twitch.tv/activate/"));
        assert!(allowed(&cyex_auth::browser::authorize_url("abc", "s")));
        assert!(!allowed("http://www.twitch.tv/activate?device-code=A"));
        assert!(!allowed("https://www.twitch.tv/settings"));
        assert!(!allowed("https://www.twitch.tv:444/activate"));
        assert!(!allowed("https://twitch.tv.evil.example/activate"));
        assert!(!allowed("https://evil.example/activate"));
        assert!(!allowed("file:///C:/Windows/notepad.exe"));
    }
}
