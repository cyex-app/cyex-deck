//! Saving and restoring window position and size, in logical pixels.
//!
//! New windows get their saved geometry through the builder. Calling `set_size` on a
//! hidden frameless window is off by the hidden caption height on Windows (a restored
//! window grew 30 px per start), so it's only used on windows that are already open.

use cyex_settings::Partition;
use cyex_settings::schema::{Geometry, Windows};
use serde_json::json;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewWindowBuilder, Window, Wry};

use super::{MAIN, route_of};
use crate::settings::Settings;

const MIN_SANE: (u32, u32) = (200, 150);

/// Current geometry. While maximized, the last normal size is kept so un-maximizing
/// after a restart lands where it was. Fullscreen keeps what it had before (V103-14): a
/// window closed in fullscreen came back as big as the screen.
pub fn capture(window: &Window, previous: Option<Geometry>) -> Option<Geometry> {
    if window.is_minimized().ok()? {
        return None;
    }
    if window.is_fullscreen().ok()? {
        return previous;
    }
    if window.is_maximized().ok()? {
        return Some(Geometry {
            maximized: true,
            ..previous.unwrap_or_default()
        });
    }
    let scale = window.scale_factor().ok()?;
    let pos = window.outer_position().ok()?.to_logical::<f64>(scale);
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    Some(Geometry {
        x: pos.x.round() as i32,
        y: pos.y.round() as i32,
        width: size.width.round() as u32,
        height: size.height.round() as u32,
        maximized: false,
    })
}

/// Puts a window that's about to be built where it was last time, or centers it when
/// there's no saved spot or that monitor is gone. Maximizing happens after building.
pub fn on_builder<'a>(
    app: &AppHandle,
    mut builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    saved: Option<&Geometry>,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    let Some(g) = saved else {
        return builder.center();
    };
    if is_sane(g) {
        builder = builder.inner_size(f64::from(g.width), f64::from(g.height));
    }
    if is_on_screen(app, g) {
        builder.position(f64::from(g.x), f64::from(g.y))
    } else {
        builder.center()
    }
}

/// Moves and resizes a window that's already open (layouts).
pub fn apply(window: &Window, g: &Geometry) {
    if g.maximized {
        let _ = window.maximize();
        return;
    }
    let _ = window.unmaximize();
    if is_sane(g) {
        let _ = window.set_size(LogicalSize::new(g.width, g.height));
    }
    if is_on_screen(window.app_handle(), g) {
        let _ = window.set_position(LogicalPosition::new(g.x, g.y));
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
        json!({ "panels": { route: { "geometry": g } } })
    };
    let _ = settings.patch(app, Partition::Windows, &patch, false);
}

fn is_sane(g: &Geometry) -> bool {
    g.width >= MIN_SANE.0 && g.height >= MIN_SANE.1
}

/// Skips positions on a monitor that's no longer connected.
fn is_on_screen(app: &AppHandle, g: &Geometry) -> bool {
    let Ok(monitors) = app.available_monitors() else {
        return false;
    };
    // A point just inside the title bar must land on some monitor.
    let (x, y) = (f64::from(g.x) + 40.0, f64::from(g.y) + 10.0);
    monitors.iter().any(|m| {
        let pos = m.position().to_logical::<f64>(m.scale_factor());
        let size = m.size().to_logical::<f64>(m.scale_factor());
        x >= pos.x && y >= pos.y && x < pos.x + size.width && y < pos.y + size.height
    })
}
