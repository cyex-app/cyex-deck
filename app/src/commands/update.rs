//! V105-20 Update commands.

use cyex_core::version;
use cyex_settings::Partition;
use cyex_settings::schema::General;
use cyex_twitch::github::Update;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::settings::Settings;
use crate::update::install::{self, UpdateProgress};
use crate::update::{self, CheckResult};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn update_state(app: AppHandle) -> Option<Update> {
    update::available(&app)
}

/// "Check now": ignores the skipped version.
#[tauri::command]
pub async fn update_check(app: AppHandle) -> CheckResult {
    update::check(&app, true).await
}

/// "Skip this version": the badge goes and the timed check stays quiet about it.
#[tauri::command]
pub fn update_skip(app: AppHandle, settings: State<Settings>, version: String) -> CmdResult<()> {
    let v = version::parse(&version).ok_or("Not a version.")?;
    settings.update::<General>(&app, Partition::General, |g| {
        g.updates.skip = v.to_string();
        true
    })?;
    update::set_available(&app, None);
    Ok(())
}

#[tauri::command]
pub async fn update_install(app: AppHandle, on_event: Channel<UpdateProgress>) -> CmdResult<()> {
    install::run(app, on_event).await
}

/// "What's new" / "Open release page": the release's own page on github.com, nothing else.
#[tauri::command]
pub fn update_open_notes(app: AppHandle) -> CmdResult<()> {
    let update = update::available(&app).ok_or("No update.")?;
    if !is_release_page(&update.notes_url) {
        return Err("That link can't be opened from here.".into());
    }
    app.opener()
        .open_url(update.notes_url, None::<&str>)
        .map_err(|e| e.to_string())
}

fn is_release_page(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| u.scheme() == "https" && u.host_str() == Some("github.com"))
}

#[cfg(test)]
mod tests {
    use super::is_release_page;

    #[test]
    fn only_github_over_https() {
        assert!(is_release_page("https://github.com/cyex-app/cyex-deck/releases/tag/v0.1.5"));
        assert!(!is_release_page("http://github.com/x"));
        assert!(!is_release_page("https://github.com.evil.test/x"));
        assert!(!is_release_page("file:///C:/Windows/system32/calc.exe"));
    }
}
