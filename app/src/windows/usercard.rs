//! Usercard windows (1.36): small pop-ups next to the mouse. Unpinned ones close when
//! they lose focus; pinned ones stay on top until closed. They share one saved width; the
//! height is fixed and the messages scroll inside (V101-05).

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::Windows;
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl};

use super::panels::{PanelRegistry, label_for};
use crate::icons;
use crate::settings::Settings;

const KIND: &str = "usercard";
const SIZE: (f64, f64) = (380.0, 560.0);
const WIDTHS: std::ops::RangeInclusive<f64> = 340.0..=640.0;
/// Focus can bounce between the window and its webview; only a lasting loss counts.
const BLUR_GRACE: Duration = Duration::from_millis(250);

/// Cards that have had focus. A card that never had it isn't closed for losing it,
/// e.g. while it's still being created.
fn focused_once() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(Mutex::default)
}

/// 1.36 An unpinned card closes when you click elsewhere.
pub fn on_focus(window: &tauri::Window, focused: bool) {
    let label = window.label().to_owned();
    let mut seen = focused_once().lock().unwrap_or_else(|p| p.into_inner());
    if focused {
        seen.insert(label);
        return;
    }
    if !seen.contains(&label) {
        return;
    }
    drop(seen);
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(BLUR_GRACE).await;
        let still_away = !window.is_focused().unwrap_or(false);
        if still_away && !window.is_always_on_top().unwrap_or(false) {
            focused_once().lock().unwrap_or_else(|p| p.into_inner()).remove(window.label());
            let _ = window.close();
        }
    });
}

pub fn is_usercard(route: &str) -> bool {
    route == KIND || route.starts_with("usercard/")
}

/// Settings key for a route: all usercards share one entry, and so do the chats opened
/// from a `#channel` (V102-05), so `windows` doesn't grow with every channel.
pub fn settings_key(route: &str) -> &str {
    if is_usercard(route) {
        KIND
    } else if route.starts_with("room/") {
        "room"
    } else {
        route
    }
}

pub fn open(app: &AppHandle, channel: &str, login: &str) -> tauri::Result<()> {
    let route = format!("{KIND}/{channel}/{login}");
    let label = label_for(&route);
    if let Some(existing) = app.get_webview_window(&label) {
        existing.show()?;
        return existing.set_focus();
    }
    let saved = app
        .state::<Settings>()
        .read::<Windows>(Partition::Windows)
        .panels
        .remove(KIND)
        .and_then(|p| p.geometry)
        .map(|g| f64::from(g.width))
        .filter(|w| WIDTHS.contains(w));
    let (w, h) = (saved.unwrap_or(SIZE.0), SIZE.1);
    let url = WebviewUrl::App(format!("index.html#/panel/{route}").into());
    let window = super::builder(app, &label, url)
        // V102-14 The channel: the name is on the card already.
        .title(format!("#{channel}"))
        .inner_size(w, h)
        .min_inner_size(*WIDTHS.start(), h)
        .max_inner_size(*WIDTHS.end(), h)
        .skip_taskbar(true)
        .center()
        .build()?;
    // Placed in physical pixels while still hidden: the mouse may be on a monitor with
    // another scale than the one the window was created on, and Windows rescales it
    // when it moves there.
    if let Some(p) = near_cursor(app, w, h) {
        window.set_position(p)?;
    }
    app.state::<PanelRegistry>().insert(&label, &route);
    icons::apply(&window.as_ref().window());
    window.show()?;
    window.set_focus()
}

/// Just right of and below the mouse, kept on its monitor and off the taskbar (V101-04).
/// Physical pixels.
fn near_cursor(app: &AppHandle, w: f64, h: f64) -> Option<PhysicalPosition<i32>> {
    let cursor = app.cursor_position().ok()?;
    let monitor = app.monitor_from_point(cursor.x, cursor.y).ok().flatten()?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (pos, size) = (area.position, area.size);
    let (w, h) = (w * scale, h * scale);
    let right = f64::from(pos.x) + f64::from(size.width) - w;
    let bottom = f64::from(pos.y) + f64::from(size.height) - h;
    let x = (cursor.x + 12.0 * scale).min(right).max(f64::from(pos.x));
    let y = (cursor.y + 12.0 * scale).min(bottom).max(f64::from(pos.y));
    Some(PhysicalPosition::new(x as i32, y as i32))
}
