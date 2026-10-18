//! App zoom (`appearance.zoom`, percent): WebView2's own page zoom on the main window and
//! every pop-out. Overlays keep their own size setting (U7) and the Twitch and 7TV
//! sign-in pages are not ours to scale, so neither is touched.
//! V106-01 Only a zoom change touches windows; the minimum is answered by our subclass
//! (`sys::frame`).

use std::sync::atomic::{AtomicU16, Ordering};

use cyex_settings::Partition;
use cyex_settings::schema::Appearance;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Webview, WebviewWindow};

use crate::settings::Settings;

/// Windows that follow the zoom: the main window and pop-outs, not overlays.
pub(super) fn follows(label: &str) -> bool {
    label == super::MAIN || (label.starts_with("panel-") && !super::overlay::is_overlay_window(label))
}

/// `appearance.zoom` in percent.
pub(super) fn zoom_percent(app: &AppHandle) -> u16 {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    a.zoom
}

/// V106-01 The zoom the windows were last fitted to. Other appearance changes (theme,
/// tab size, …) leave windows alone.
static FITTED: AtomicU16 = AtomicU16::new(0);

/// True when `now` differs from the zoom last fitted to (and remembers it).
fn zoom_changed(last: &AtomicU16, now: u16) -> bool {
    last.swap(now, Ordering::AcqRel) != now
}

/// Start: the windows open at the saved zoom.
pub fn init(app: &AppHandle) {
    FITTED.store(zoom_percent(app), Ordering::Release);
}

/// A page loading in `webview` starts at the current zoom.
pub fn apply(webview: &Webview) {
    if follows(webview.label()) {
        let _ = webview.set_zoom(f64::from(zoom_percent(webview.app_handle())) / 100.0);
    }
}

/// The setting changed: every open window follows at once.
pub fn apply_all(app: &AppHandle) {
    let zoom = zoom_percent(app);
    if !zoom_changed(&FITTED, zoom) {
        return;
    }
    let f = f64::from(zoom) / 100.0;
    for (label, window) in app.webview_windows() {
        if follows(&label) {
            let _ = window.set_zoom(f);
            fit(app, &window, zoom);
        }
    }
}

const MAIN_MIN: (f64, f64) = (720.0, 420.0);

/// Smallest window (logical px) that keeps the page at its base minimum in CSS px: the base
/// times the zoom, but never more than the work area (`work`, logical px) can hold.
pub fn min_size(base: (f64, f64), zoom: u16, work: Option<(f64, f64)>) -> (f64, f64) {
    let f = f64::from(zoom) / 100.0;
    let (w, h) = (base.0 * f, base.1 * f);
    work.map_or((w, h), |(ww, wh)| (w.min(ww), h.min(wh)))
}

/// The window's own minimum (`None` for pop-ups that keep a fixed size).
fn base_min(app: &AppHandle, window: &WebviewWindow) -> Option<(f64, f64)> {
    if window.label() == super::MAIN {
        return Some(MAIN_MIN);
    }
    let popup = super::route_of(app, window.label()).is_some_and(|r| super::usercard::is_usercard(&r));
    (!popup).then_some(super::panels::MIN_SIZE)
}

/// K2 The size `size` grows to so both sides reach `min`; `None` when it already fits.
fn grown(size: (u32, u32), min: (u32, u32)) -> Option<(u32, u32)> {
    (size.0 < min.0 || size.1 < min.1).then(|| (size.0.max(min.0), size.1.max(min.1)))
}

/// Sets the window's minimum for `zoom` (percent) and grows the window to it, keeping it
/// on its monitor.
pub fn fit(app: &AppHandle, window: &WebviewWindow, zoom: u16) {
    use crate::sys::frame;

    let Some(base) = base_min(app, window) else { return };
    let Ok(scale) = window.scale_factor() else { return };
    let area = window.current_monitor().ok().flatten().map(|m| *m.work_area());
    let work = area.map(|r| (f64::from(r.size.width) / scale, f64::from(r.size.height) / scale));
    let dpi = (scale * 96.0).round() as u32;
    let (mut pw, mut ph) = frame::min_track(frame::centi(min_size(base, zoom, work)), dpi);
    // In physical px too, so rounding never asks for more than the work area.
    if let Some(r) = area {
        (pw, ph) = (pw.min(r.size.width), ph.min(r.size.height));
    }
    frame::set_min(window, frame::from_physical((pw, ph), dpi));
    if window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false) {
        return;
    }
    let Ok(size) = window.inner_size() else { return };
    let Some((w, h)) = grown((size.width, size.height), (pw, ph)) else { return };
    let _ = window.set_size(PhysicalSize::new(w, h));
    // Growing from the top left may push the edge off the monitor: move it back in.
    if let (Some(r), Ok(pos), Ok(outer)) = (area, window.outer_position(), window.outer_size()) {
        let end = |o: i32, len: u32, size: u32| o + i32::try_from(len).unwrap_or(0) - i32::try_from(size).unwrap_or(0);
        let x = pos.x.min(end(r.position.x, r.size.width, outer.width)).max(r.position.x);
        let y = pos.y.min(end(r.position.y, r.size.height, outer.height)).max(r.position.y);
        if (x, y) != (pos.x, pos.y) {
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_scales_with_zoom_and_stops_at_the_work_area() {
        assert_eq!(min_size((720.0, 420.0), 100, None), (720.0, 420.0));
        assert_eq!(min_size((720.0, 420.0), 200, None), (1440.0, 840.0));
        assert_eq!(min_size((320.0, 240.0), 150, None), (480.0, 360.0));
        assert_eq!(min_size((720.0, 420.0), 200, Some((1920.0, 1040.0))), (1440.0, 840.0));
        assert_eq!(min_size((720.0, 420.0), 200, Some((1366.0, 728.0))), (1366.0, 728.0));
    }

    #[test]
    fn grows_only_the_sides_below_the_minimum() {
        assert_eq!(grown((720, 420), (1440, 840)), Some((1440, 840)));
        assert_eq!(grown((1500, 900), (1440, 840)), None);
        assert_eq!(grown((1500, 400), (1440, 840)), Some((1500, 840)));
    }

    #[test]
    fn only_a_new_zoom_counts_as_a_change() {
        let last = AtomicU16::new(100);
        assert!(!zoom_changed(&last, 100));
        assert!(zoom_changed(&last, 125));
        assert!(!zoom_changed(&last, 125));
        assert!(zoom_changed(&last, 100));
    }
}
