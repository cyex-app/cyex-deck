//! Saving and restoring window position and size, in logical pixels.
//!
//! New windows get their saved geometry through the builder. Calling `set_size` on a
//! hidden frameless window is off by the hidden caption height on Windows (a restored
//! window grew 30 px per start), so it's only used on windows that are already open.
//! W1 Where a window lands is worked out in physical px (`placement`): it always comes back
//! on a connected monitor, inside its work area.

use cyex_settings::Partition;
use cyex_settings::schema::{Geometry, Windows};
use serde_json::json;
use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, WebviewWindow, WebviewWindowBuilder, Window, Wry,
};

use super::placement::{self, Placement, Rect, Screen};
use super::{MAIN, route_of};
use crate::settings::Settings;

/// Logical size a maximized window without a good normal rect gets when un-maximized.
const NORMAL: (f64, f64) = (1280.0, 800.0);

pub(super) fn screen(m: &Monitor) -> Screen {
    let rect = |p: PhysicalPosition<i32>, w: u32, h: u32| Rect {
        x: p.x,
        y: p.y,
        w: i32::try_from(w).unwrap_or(i32::MAX),
        h: i32::try_from(h).unwrap_or(i32::MAX),
    };
    let work = m.work_area();
    let work = rect(work.position, work.size.width, work.size.height);
    let bounds = rect(*m.position(), m.size().width, m.size().height);
    // A work area of nothing (seen on some drivers) is the whole monitor.
    let work = if work.w > 0 && work.h > 0 { work } else { bounds };
    Screen { bounds, work, scale: m.scale_factor() }
}

/// Connected monitors and which one is the primary.
fn screens(app: &AppHandle) -> (Vec<Screen>, usize) {
    let all: Vec<Screen> = app.available_monitors().unwrap_or_default().iter().map(screen).collect();
    let primary = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| screen(&m))
        .and_then(|p| all.iter().position(|s| s.bounds == p.bounds))
        .unwrap_or(0);
    (all, primary)
}

/// Current geometry. While maximized, the last normal size is kept so un-maximizing
/// after a restart lands where it was. Fullscreen keeps what it had before (V103-14): a
/// window closed in fullscreen came back as big as the screen. W1 Minimized, mid-change or
/// junk rects are never saved: the last good one stays.
pub fn capture(window: &Window, previous: Option<Geometry>) -> Option<Geometry> {
    let (minimized, maximized) = crate::sys::frame::state(window)?;
    if minimized {
        return None;
    }
    if window.is_fullscreen().ok()? {
        return previous;
    }
    if maximized {
        // B2 Moved maximized to another monitor: the old monitor's normal rect would bring
        // it back there.
        let on = screen(&window.current_monitor().ok().flatten()?);
        let (all, _) = screens(window.app_handle());
        return Some(placement::maximized_on(previous, &all, &on, NORMAL));
    }
    let scale = window.scale_factor().ok()?;
    // The position at the scale of the monitor it's on now: mid-move, the window's own scale
    // can still be the old monitor's.
    let at = window.current_monitor().ok().flatten().map_or(scale, |m| m.scale_factor());
    let pos = window.outer_position().ok()?.to_logical::<f64>(at);
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    let g = Geometry {
        x: pos.x.round() as i32,
        y: pos.y.round() as i32,
        width: size.width.round() as u32,
        height: size.height.round() as u32,
        maximized: false,
        // B1 The scale the position was mapped with, so it maps back to the same px.
        scale: Some(at),
    };
    placement::is_valid(&g).then_some(g)
}

/// Puts a window that's about to be built where it was last time, or centers it when
/// there's nothing saved (then on the main window's monitor). `min` and `default` are logical. Call `settle` after building.
pub fn on_builder<'a>(
    app: &AppHandle,
    builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    saved: Option<&Geometry>,
    min: (f64, f64),
    default: (f64, f64),
) -> (WebviewWindowBuilder<'a, Wry, AppHandle>, Option<Placement>) {
    let (all, primary) = screens(app);
    // Nothing saved, or junk: centered on the main window's monitor.
    let home = main_screen(app, &all).unwrap_or(primary);
    let g = saved.copied().unwrap_or_default();
    let Some(p) = placement::place(&g, &all, home, min, default) else {
        return (builder.center(), None);
    };
    // tao maps a logical position to the first monitor it fits at that monitor's scale; with
    // mixed scales that can be the wrong one, so `settle` moves it in physical px.
    let ((w, h), (x, y)) = (p.logical_size(), p.logical_position());
    (builder.inner_size(w, h).position(x, y), Some(p))
}

/// The monitor the main window is on, as an index into `all`.
fn main_screen(app: &AppHandle, all: &[Screen]) -> Option<usize> {
    let m = app.get_webview_window(MAIN)?.current_monitor().ok().flatten()?;
    let s = screen(&m);
    all.iter().position(|a| a.bounds == s.bounds)
}

/// W1 Top-left corner (physical px) that centers a `w` x `h` (logical) window on the main
/// window's monitor, for pop-ups opened without the mouse on the app (keyboard, scripts).
pub fn center_on_main(app: &AppHandle, w: f64, h: f64) -> Option<PhysicalPosition<i32>> {
    let (all, primary) = screens(app);
    let home = main_screen(app, &all).unwrap_or(primary);
    let s = all.get(home).or_else(|| all.first())?;
    let (x, y) = placement::center(s, w * s.scale, h * s.scale);
    Some(PhysicalPosition::new(x, y))
}

/// Whether a point (physical px) is on one of the app's visible windows.
pub fn over_app(app: &AppHandle, x: f64, y: f64) -> bool {
    app.webview_windows().values().any(|w| {
        w.is_visible().unwrap_or(false)
            && !w.is_minimized().unwrap_or(true)
            && w.outer_position().ok().zip(w.outer_size().ok()).is_some_and(|(p, s)| {
                let size = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
                Rect { x: p.x, y: p.y, w: size(s.width), h: size(s.height) }.contains(x, y)
            })
    })
}

/// Moves a just-built, still hidden window to its spot in physical px and maximizes it there
/// when it was saved maximized. Windows keeps its logical size when it changes monitors.
pub fn settle(window: &WebviewWindow, p: Option<Placement>) -> tauri::Result<()> {
    let Some(p) = p else { return Ok(()) };
    window.set_position(PhysicalPosition::new(p.x, p.y))?;
    if p.maximized {
        window.maximize()?;
    }
    Ok(())
}

/// Moves and resizes a window that's already open (layouts).
pub fn apply(window: &Window, g: &Geometry) {
    let (all, primary) = screens(window.app_handle());
    let current = window
        .inner_size()
        .ok()
        .zip(window.scale_factor().ok())
        .map_or(NORMAL, |(s, f)| (f64::from(s.width) / f, f64::from(s.height) / f));
    let min = (f64::from(placement::MIN_SANE.0), f64::from(placement::MIN_SANE.1));
    let Some(p) = placement::place(g, &all, primary, min, current) else {
        return;
    };
    let _ = window.unmaximize();
    // Moved first: on a monitor with another scale the logical size then means that scale.
    let _ = window.set_position(PhysicalPosition::new(p.x, p.y));
    let (w, h) = p.logical_size();
    let _ = window.set_size(LogicalSize::new(w, h));
    if p.maximized {
        let _ = window.maximize();
    }
}

/// Saves geometry after a move or resize. Not broadcast to the UI.
pub fn remember(app: &AppHandle, window: &Window) {
    let settings = app.state::<Settings>();
    let saved: Windows = settings.read(Partition::Windows);
    let patch = if window.label() == MAIN {
        let Some(g) = capture(window, saved.main) else {
            return;
        };
        if saved.main == Some(g) {
            return;
        }
        json!({ "main": g })
    } else {
        let Some(route) = route_of(app, window.label()) else {
            return;
        };
        let route = super::usercard::settings_key(&route).to_owned();
        let previous = saved.panels.get(&route).and_then(|p| p.geometry);
        let Some(g) = capture(window, previous) else {
            return;
        };
        if previous == Some(g) {
            return;
        }
        json!({ "panels": { route: { "geometry": g } } })
    };
    let _ = settings.patch(app, Partition::Windows, &patch, false);
}
