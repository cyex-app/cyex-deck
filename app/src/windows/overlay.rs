//! U7 Chat overlays: a tab's chat in a see-through window that stays on top, e.g. over a
//! game. Route `overlay/<tile id>`; position and size are kept like any pop-out. With
//! click-through on, the mouse goes to whatever is below, except on the overlay's top bar
//! (V105-02); it's turned off again from that bar, the tray or a global hotkey (S2).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder, Wry};

const PREFIX: &str = "overlay/";
const CLICK_THROUGH_EVENT: &str = "overlay:clickThrough";
/// V105-02 Height of the overlay's top bar (OverlayView.svelte, .bar), logical px.
const BAR_HEIGHT: f64 = 30.0;
const WATCH_EVERY: Duration = Duration::from_millis(50);
const BAR_EVENT: &str = "overlay:bar";

#[derive(Default)]
pub struct ClickThrough {
    /// Overlays that let clicks through.
    on: Mutex<HashSet<String>>,
    /// V105-02 Of those, the ones whose bar is under the cursor right now (they take the mouse).
    bar: Mutex<HashSet<String>>,
    /// The cursor watch is running.
    watching: AtomicBool,
}

fn lock(app: &AppHandle) -> MutexGuard<'_, HashSet<String>> {
    app.state::<ClickThrough>()
        .inner()
        .on
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

fn bars(app: &AppHandle) -> MutexGuard<'_, HashSet<String>> {
    app.state::<ClickThrough>()
        .inner()
        .bar
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// Whether the cursor (physical px) is on the bar of a window at `pos` with `width` (physical px).
fn on_bar(cursor: (f64, f64), pos: (i32, i32), width: u32, scale: f64) -> bool {
    let (x, y) = cursor;
    let (left, top) = (f64::from(pos.0), f64::from(pos.1));
    x >= left && x < left + f64::from(width) && y >= top && y < top + BAR_HEIGHT * scale
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

/// Turns click-through on or off for one overlay. Runs on the main thread, so the cursor
/// watch and the bar, tray and hotkey can't race (V105-02).
pub fn set_click_through(app: &AppHandle, label: &str, on: bool) -> Result<(), String> {
    app.get_webview_window(label).ok_or("That overlay is closed.")?;
    let (a, l) = (app.clone(), label.to_owned());
    app.run_on_main_thread(move || apply(&a, &l, on))
        .map_err(|e| e.to_string())
}

/// Main thread only.
fn apply(app: &AppHandle, label: &str, on: bool) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    if window.set_ignore_cursor_events(on).is_err() {
        return;
    }
    {
        let mut set = lock(app);
        if on {
            set.insert(label.to_owned());
        } else {
            set.remove(label);
        }
    }
    bars(app).remove(label);
    let _ = app.emit_to(label, CLICK_THROUGH_EVENT, on);
    let _ = app.emit_to(label, BAR_EVENT, false);
    sync_tray(app);
    if on {
        watch(app);
    }
}

/// V105-02 While an overlay lets clicks through, its top bar takes the mouse when the
/// cursor is over it. Runs only while at least one overlay is click-through.
fn watch(app: &AppHandle) {
    if app.state::<ClickThrough>().watching.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(WATCH_EVERY).await;
            if !app.state::<ClickThrough>().watching.load(Ordering::SeqCst) {
                break;
            }
            let a = app.clone();
            if app.run_on_main_thread(move || tick(&a)).is_err() {
                break;
            }
        }
    });
}

/// Main thread only.
fn tick(app: &AppHandle) {
    let labels = lock(app).clone();
    if labels.is_empty() {
        bars(app).clear();
        app.state::<ClickThrough>()
            .watching
            .store(false, Ordering::SeqCst);
        return;
    }
    let Ok(cursor) = app.cursor_position() else {
        return;
    };
    for label in labels {
        let Some(window) = app.get_webview_window(&label) else {
            continue;
        };
        let (Ok(pos), Ok(size), Ok(scale)) = (
            window.inner_position(),
            window.inner_size(),
            window.scale_factor(),
        ) else {
            continue;
        };
        let over = on_bar((cursor.x, cursor.y), (pos.x, pos.y), size.width, scale);
        let had = bars(app).contains(&label);
        if over != had {
            let _ = window.set_ignore_cursor_events(!over);
            if over {
                bars(app).insert(label.clone());
            } else {
                bars(app).remove(&label);
            }
            let _ = app.emit_to(&label, BAR_EVENT, over);
        }
    }
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
        bars(app).remove(label);
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
    use super::{is_overlay, is_overlay_window, on_bar};
    use crate::windows::panels::label_for;

    #[test]
    fn routes() {
        assert!(is_overlay("overlay/abc"));
        assert!(!is_overlay("chat/abc"));
    }

    #[test]
    fn bar_hit() {
        for scale in [1.0, 1.5] {
            let h = 30.0 * scale;
            let (pos, w) = ((100, 50), 400);
            assert!(on_bar((100.0, 50.0), pos, w, scale));
            assert!(on_bar((300.0, 50.0 + h - 1.0), pos, w, scale));
            assert!(!on_bar((300.0, 50.0 + h), pos, w, scale));
            assert!(!on_bar((500.0, 60.0), pos, w, scale));
            assert!(on_bar((499.0, 60.0), pos, w, scale));
            assert!(!on_bar((99.0, 60.0), pos, w, scale));
            assert!(!on_bar((300.0, 49.0), pos, w, scale));
        }
    }

    #[test]
    fn window_labels() {
        assert!(is_overlay_window(&label_for("overlay/abc")));
        assert!(!is_overlay_window(&label_for("chat/abc")));
        assert!(!is_overlay_window("main"));
    }
}
