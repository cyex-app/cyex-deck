//! Links that open in the default browser.

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

type CmdResult<T> = Result<T, String>;

/// Hosts the UI may open. Everything else is refused, so a bad link in chat or a token
/// field can't launch arbitrary URLs.
/// Twitch, the emote providers' pages (1.41 emote cards) and the logs site (1.36).
const ALLOWED_HOSTS: [&str; 9] = [
    "twitch.tv",
    "www.twitch.tv",
    "dev.twitch.tv",
    "clips.twitch.tv",
    "7tv.app",
    "betterttv.com",
    "www.frankerfacez.com",
    "frankerfacez.com",
    "logs.cyex.app",
];

#[tauri::command]
pub fn system_open_link(app: AppHandle, url: String) -> CmdResult<()> {
    open_allowed(&app, url)
}

/// Opens one of the allowed sites in the browser.
pub fn open_allowed(app: &AppHandle, url: String) -> CmdResult<()> {
    if !is_allowed(&url) {
        return Err("That link can’t be opened from here.".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// A link someone posted in chat (1.8). Any web address, nothing else.
#[tauri::command]
pub fn system_open_chat_link(app: AppHandle, url: String) -> CmdResult<()> {
    if !is_web(&url) {
        return Err("That link can’t be opened.".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// V1051-03 Empties the picture cache (emotes, badges, paints).
#[tauri::command]
pub async fn system_clear_picture_cache(app: AppHandle) -> CmdResult<()> {
    use tauri::Manager;
    let main = app
        .get_webview_window(crate::windows::MAIN)
        .ok_or("The main window isn’t open.")?;
    crate::sys::clear_picture_cache(&main).await
}

/// V1052-02 Bytes the picture cache takes on disk.
#[tauri::command]
pub async fn system_picture_cache_size(app: AppHandle) -> u64 {
    use tauri::Manager;
    let Some(dir) = crate::paths::webview(&app).or_else(|| app.path().app_local_data_dir().ok()) else {
        return 0;
    };
    tauri::async_runtime::spawn_blocking(move || crate::sys::cache::picture_cache_size(&dir))
        .await
        .unwrap_or(0)
}

fn is_web(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
}

fn is_allowed(url: &str) -> bool {
    Url::parse(url)
        .is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some_and(|h| ALLOWED_HOSTS.contains(&h)))
}

#[cfg(test)]
mod tests {
    use super::{is_allowed, is_web};

    #[test]
    fn chat_links_are_web_only() {
        assert!(is_web("https://example.com/a?b"));
        assert!(is_web("http://x.y"));
        assert!(!is_web("file:///C:/Windows/system32/calc.exe"));
        assert!(!is_web("javascript:alert(1)"));
        assert!(!is_web("ms-settings:privacy"));
    }

    #[test]
    fn only_twitch_over_https() {
        assert!(is_allowed(
            "https://www.twitch.tv/activate?public=true&device-code=ABCD"
        ));
        assert!(is_allowed("https://dev.twitch.tv/console/apps"));
        assert!(is_allowed("https://7tv.app/emotes/01FG70GMPR00021TQRJX35KWP1"));
        assert!(!is_allowed("http://www.twitch.tv/activate"));
        assert!(!is_allowed("https://twitch.tv.evil.example/"));
        assert!(!is_allowed("file:///C:/Windows"));
    }
}
