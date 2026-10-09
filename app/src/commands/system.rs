//! Links that open in the default browser.

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

type CmdResult<T> = Result<T, String>;

/// Hosts the UI may open. Everything else is refused, so a bad link in chat or a token
/// field can't launch arbitrary URLs.
const ALLOWED_HOSTS: [&str; 3] = ["twitch.tv", "www.twitch.tv", "dev.twitch.tv"];

#[tauri::command]
pub fn system_open_link(app: AppHandle, url: String) -> CmdResult<()> {
    if !is_allowed(&url) {
        return Err("That link can't be opened from here.".into());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

fn is_allowed(url: &str) -> bool {
    Url::parse(url)
        .is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some_and(|h| ALLOWED_HOSTS.contains(&h)))
}

#[cfg(test)]
mod tests {
    use super::is_allowed;

    #[test]
    fn only_twitch_over_https() {
        assert!(is_allowed(
            "https://www.twitch.tv/activate?public=true&device-code=ABCD"
        ));
        assert!(is_allowed("https://dev.twitch.tv/console/apps"));
        assert!(!is_allowed("http://www.twitch.tv/activate"));
        assert!(!is_allowed("https://twitch.tv.evil.example/"));
        assert!(!is_allowed("file:///C:/Windows"));
    }
}
