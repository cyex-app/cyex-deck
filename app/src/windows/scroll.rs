//! V103-11 Smooth scrolling for every scrollable area in the app's windows. WebView2 animates
//! wheel and key scrolling itself, off the page's main thread, so no page code is involved.
//! It's a browser flag and every window of one profile must start with the same flags, so
//! the setting is read once per run and applies from the next start.

use std::sync::OnceLock;

use cyex_settings::Partition;
use cyex_settings::schema::General;
use tauri::{AppHandle, Manager};

use crate::settings::Settings;

/// `base` plus the smooth-scrolling flag the setting asks for, the same for the whole run.
pub fn browser_args(app: &AppHandle, base: &str) -> &'static str {
    static ARGS: OnceLock<String> = OnceLock::new();
    ARGS.get_or_init(|| {
        let smooth = app
            .try_state::<Settings>()
            .is_none_or(|s| s.read::<General>(Partition::General).smooth_scroll);
        with_scrolling(base, smooth)
    })
}

/// Forced either way, so Windows' own animation switch doesn't decide it.
fn with_scrolling(base: &str, smooth: bool) -> String {
    let flag = if smooth {
        "--enable-smooth-scrolling"
    } else {
        "--disable-smooth-scrolling"
    };
    format!("{base} {flag}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_follows_the_setting() {
        assert_eq!(with_scrolling("--a", true), "--a --enable-smooth-scrolling");
        assert_eq!(with_scrolling("--a", false), "--a --disable-smooth-scrolling");
    }
}
