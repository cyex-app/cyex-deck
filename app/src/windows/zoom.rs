//! App zoom (`appearance.zoom`, percent): WebView2's own page zoom on the main window and
//! every pop-out. Overlays keep their own size setting (U7) and the Twitch and 7TV
//! sign-in pages are not ours to scale, so neither is touched.

use cyex_settings::Partition;
use cyex_settings::schema::Appearance;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Webview, WebviewWindow};

use crate::settings::Settings;

/// Windows that follow the zoom: the main window and pop-outs, not overlays.
pub(super) fn follows(label: &str) -> bool {
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
            fit(app, &window);
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

/// Sets the window's minimum for the current zoom and grows the window to it, keeping it
/// on its monitor.
pub fn fit(app: &AppHandle, window: &WebviewWindow) {
    let Some(base) = base_min(app, window) else { return };
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    let Ok(scale) = window.scale_factor() else { return };
    let area = window.current_monitor().ok().flatten().map(|m| *m.work_area());
    let work = area.map(|r| (f64::from(r.size.width) / scale, f64::from(r.size.height) / scale));
    let (mw, mh) = min_size(base, a.zoom, work);
    let _ = window.set_min_size(Some(LogicalSize::new(mw, mh)));
    if window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false) {
        return;
    }
    let Ok(size) = window.inner_size() else { return };
    let (cw, ch) = (f64::from(size.width) / scale, f64::from(size.height) / scale);
    if cw >= mw && ch >= mh {
        return;
    }
    let _ = window.set_size(LogicalSize::new(cw.max(mw), ch.max(mh)));
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
}
