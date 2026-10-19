//! Links that open in the default browser.

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

type CmdResult<T> = Result<T, String>;

/// Hosts the UI may open. Everything else is refused, so a bad link in chat or a token
/// field can't launch arbitrary URLs.
/// Twitch, the emote providers' pages (1.41 emote cards) the logs site (1.36), the docs (V106-12) and Chat Vault
/// (emote pages for Twitch emotes, 2026/10/18).
const ALLOWED_HOSTS: [&str; 11] = [
    "twitch.tv",
    "www.twitch.tv",
    "dev.twitch.tv",
    "clips.twitch.tv",
    "7tv.app",
    "betterttv.com",
    "www.frankerfacez.com",
    "frankerfacez.com",
    "logs.cyex.app",
    "cyex.app",
    "chatvau.lt",
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

/// L2 Least time between two collections, whichever window asks (they share one page process).
const TRIM_GAP: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// L2 One full garbage collection in the app's page process. Chat churns the page's DOM
/// heap; its freed space only goes back with a full collection, which a busy shown page
/// rarely gets (soak: +0,5-2 MB/min, all of it given back by one, ~20 ms). The UI asks
/// every 10 min once the user is idle; hidden windows already give memory back (D28).
#[tauri::command]
pub async fn system_trim_memory(app: AppHandle) -> CmdResult<()> {
    use std::sync::Mutex;
    use std::time::Instant;
    use tauri::Manager;
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    {
        let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();
        if !trim_due(*last, now) {
            return Ok(());
        }
        *last = Some(now);
    }
    let main = app
        .get_webview_window(crate::windows::MAIN)
        .ok_or("The main window isn’t open.")?;
    // Best effort: an older WebView2 or a failed call just leaves memory as it is.
    let _ = crate::sys::devtools::call(&main, "HeapProfiler.collectGarbage", "{}".into()).await;
    Ok(())
}

fn trim_due(last: Option<std::time::Instant>, now: std::time::Instant) -> bool {
    last.is_none_or(|l| now.saturating_duration_since(l) >= TRIM_GAP)
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
    use super::{TRIM_GAP, is_allowed, is_web, trim_due};

    #[test]
    fn trims_at_most_every_five_minutes() {
        let t = std::time::Instant::now();
        assert!(trim_due(None, t));
        assert!(!trim_due(Some(t), t));
        assert!(!trim_due(Some(t), t + TRIM_GAP - std::time::Duration::from_secs(1)));
        assert!(trim_due(Some(t), t + TRIM_GAP));
    }

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
        assert!(is_allowed("https://chatvau.lt/emote/twitch/25"));
        assert!(!is_allowed("http://www.twitch.tv/activate"));
        assert!(!is_allowed("https://twitch.tv.evil.example/"));
        assert!(is_allowed("https://cyex.app/deck/docs/#commands"));
        assert!(!is_allowed("http://cyex.app/deck/docs/"));
        assert!(!is_allowed("https://cyex.app.evil.example/"));
        assert!(!is_allowed("file:///C:/Windows"));
    }
}
