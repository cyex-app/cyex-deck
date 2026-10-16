//! App zoom (`appearance.zoom`, percent): WebView2's own page zoom on the main window and
//! every pop-out. Overlays keep their own size setting (U7) and the Twitch and 7TV
//! sign-in pages are not ours to scale, so neither is touched.

use cyex_settings::Partition;
use cyex_settings::schema::Appearance;
use tauri::{AppHandle, Manager, Webview};

use crate::settings::Settings;

/// Windows that follow the zoom: the main window and pop-outs, not overlays.
fn follows(label: &str) -> bool {
    label == super::MAIN || (label.starts_with("panel-") && !super::overlay::is_overlay_window(label))
}

fn factor(app: &AppHandle) -> f64 {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    f64::from(a.zoom) / 100.0
}

/// A page loading in `webview` starts at the current zoom.
pub fn apply(webview: &Webview) {
    if follows(webview.label()) {
        let _ = webview.set_zoom(factor(webview.app_handle()));
    }
}

/// The setting changed: every open window follows at once.
pub fn apply_all(app: &AppHandle) {
    let f = factor(app);
    for (label, window) in app.webview_windows() {
        if follows(&label) {
            let _ = window.set_zoom(f);
        }
    }
}
