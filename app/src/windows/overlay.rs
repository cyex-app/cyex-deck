//! U7 Chat overlays: a tab's chat in a see-through window that stays on top, e.g. over a
//! game. Route `overlay/<tile id>`; position and size are kept like any pop-out. With
//! click-through on, the mouse goes to whatever is below; it's turned off again from the
//! tray or a global hotkey (S2).

use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};

use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder, Wry};

const PREFIX: &str = "overlay/";
const CLICK_THROUGH_EVENT: &str = "overlay:clickThrough";

/// Labels of the overlays that let clicks through.
#[derive(Default)]
pub struct ClickThrough(Mutex<HashSet<String>>);

fn lock(app: &AppHandle) -> MutexGuard<'_, HashSet<String>> {
    app.state::<ClickThrough>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

pub fn is_overlay(route: &str) -> bool {
    route.starts_with(PREFIX)
}

/// An overlay's window, by its label (`panels::label_for` of its route).
pub fn is_overlay_window(label: &str) -> bool {
    label.starts_with(&super::panels::label_for(PREFIX))
}

/// See-through, on top, no shadow and no taskbar button.
pub fn style<'a>(
    builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    builder
        .transparent(true)
        .background_color(Color(0, 0, 0, 0))
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(true)
}

fn overlay_labels(app: &AppHandle) -> Vec<String> {
    super::panels::open_panels_all(app)
        .into_iter()
        .filter(|(route, _)| is_overlay(route))
        .map(|(_, label)| label)
        .collect()
}

/// Turns click-through on or off for one overlay.
pub fn set_click_through(app: &AppHandle, label: &str, on: bool) -> Result<(), String> {
    let window = app.get_webview_window(label).ok_or("That overlay is closed.")?;
    window.set_ignore_cursor_events(on).map_err(|e| e.to_string())?;
    {
        let mut set = lock(app);
        if on {
            set.insert(label.to_owned());
        } else {
            set.remove(label);
        }
    }
    let _ = app.emit_to(label, CLICK_THROUGH_EVENT, on);
    sync_tray(app);
    Ok(())
}

/// The global hotkey and the tray: if any open overlay takes clicks, all let them
/// through; otherwise all take clicks again.
pub fn toggle_click_through(app: &AppHandle) {
    let labels = overlay_labels(app);
    let on = labels.iter().any(|l| !lock(app).contains(l));
    for label in labels {
        let _ = set_click_through(app, &label, on);
    }
}

pub fn is_click_through(app: &AppHandle, label: &str) -> bool {
    lock(app).contains(label)
}

pub fn forget(app: &AppHandle, label: &str) {
    if app.try_state::<ClickThrough>().is_some() {
        lock(app).remove(label);
        sync_tray(app);
    }
}

/// Opening or closing an overlay changes what the tray offers.
pub fn sync_tray(app: &AppHandle) {
    let labels = overlay_labels(app);
    let all = !labels.is_empty() && labels.iter().all(|l| lock(app).contains(l));
    crate::tray::set_overlays(app, !labels.is_empty(), all);
}

#[cfg(test)]
mod tests {
    use super::{is_overlay, is_overlay_window};
    use crate::windows::panels::label_for;

    #[test]
    fn routes() {
        assert!(is_overlay("overlay/abc"));
        assert!(!is_overlay("chat/abc"));
    }

    #[test]
    fn window_labels() {
        assert!(is_overlay_window(&label_for("overlay/abc")));
        assert!(!is_overlay_window(&label_for("chat/abc")));
        assert!(!is_overlay_window("main"));
    }
}
