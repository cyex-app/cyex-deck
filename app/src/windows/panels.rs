//! Pop-out panels (1.51): any panel opens in its own frameless window, addressed by a
//! route like `streammanager` or `chat/12345`. One window per route.

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::schema::Windows;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl};

use super::geometry;
use crate::icons;
use crate::settings::Settings;

const CHANGED_EVENT: &str = "panels:changed";
const DEFAULT_SIZE: (f64, f64) = (480.0, 640.0);
const MIN_SIZE: (f64, f64) = (320.0, 240.0);

/// Window label → route, for the panels open right now.
#[derive(Default)]
pub struct PanelRegistry(Mutex<HashMap<String, String>>);

impl PanelRegistry {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn insert(&self, label: &str, route: &str) {
        self.lock().insert(label.to_owned(), route.to_owned());
    }
}

pub fn label_for(route: &str) -> String {
    let safe: String = route
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("panel-{safe}")
}

/// V103-08 Settings stay in the main window: a `settings` pop-out saved in a layout, or
/// asked for by anything else, doesn't open.
pub fn pops_out(route: &str) -> bool {
    route != "settings"
}

pub fn route_of(app: &AppHandle, label: &str) -> Option<String> {
    app.state::<PanelRegistry>().lock().get(label).cloned()
}

/// Open panels as (route, window). Usercards aren't panels for layouts.
pub fn open_panels(app: &AppHandle) -> Vec<(String, tauri::WebviewWindow)> {
    let routes: Vec<(String, String)> = app
        .state::<PanelRegistry>()
        .lock()
        .iter()
        .filter(|(_, route)| !super::usercard::is_usercard(route))
        .map(|(label, route)| (label.clone(), route.clone()))
        .collect();
    routes
        .into_iter()
        .filter_map(|(label, route)| app.get_webview_window(&label).map(|w| (route, w)))
        .collect()
}

/// Every open panel as (route, label), usercards and overlays too.
pub fn open_panels_all(app: &AppHandle) -> Vec<(String, String)> {
    app.state::<PanelRegistry>()
        .lock()
        .iter()
        .map(|(label, route)| (route.clone(), label.clone()))
        .collect()
}

/// Routes of the open pop-outs, usercards left out.
pub fn routes(app: &AppHandle) -> Vec<String> {
    let mut routes: Vec<String> = app
        .state::<PanelRegistry>()
        .lock()
        .values()
        .filter(|r| !super::usercard::is_usercard(r))
        .cloned()
        .collect();
    routes.sort();
    routes
}

/// Tells every window which panels are popped out, so the main window can show a
/// popped-out tab as such and take it back when its window closes (3.14).
fn notify(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, routes(app));
}

pub fn forget(app: &AppHandle, label: &str) {
    let removed = app.state::<PanelRegistry>().lock().remove(label);
    if removed.is_some_and(|r| !super::usercard::is_usercard(&r)) {
        notify(app);
    }
}

/// 3.14 Closing a pop-out docks its panel back.
pub fn close(app: &AppHandle, route: &str) -> tauri::Result<()> {
    match app.get_webview_window(&label_for(route)) {
        Some(w) => w.close(),
        None => Ok(()),
    }
}

/// Opens the panel, or focuses it if it's already open. Restores its last position
/// and pinned state.
pub fn open_panel(app: &AppHandle, route: &str, title: &str, size: Option<(f64, f64)>) -> tauri::Result<()> {
    if !pops_out(route) {
        return Ok(());
    }
    let label = label_for(route);
    if let Some(existing) = app.get_webview_window(&label) {
        existing.unminimize()?;
        existing.show()?;
        return existing.set_focus();
    }

    let settings = app.state::<Settings>();
    let key = super::usercard::settings_key(route);
    let saved = settings
        .read::<Windows>(Partition::Windows)
        .panels
        .remove(key)
        .unwrap_or_default();
    let (width, height) = size.unwrap_or(DEFAULT_SIZE);
    let url = WebviewUrl::App(format!("index.html#/panel/{route}").into());
    let overlay = super::overlay::is_overlay(route);
    let builder = super::builder(app, &label, url)
        .title(title)
        .inner_size(width, height)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1)
        .always_on_top(saved.pinned);
    let builder = if overlay {
        super::overlay::style(builder)
    } else {
        builder
    };
    let builder = geometry::on_builder(app, builder, saved.geometry.as_ref());
    let window = builder.build()?;
    app.state::<PanelRegistry>()
        .lock()
        .insert(label, route.to_owned());
    notify(app);
    if overlay {
        super::overlay::sync_tray(app);
    }

    if saved.geometry.is_some_and(|g| g.maximized) {
        window.maximize()?;
    } else if route.starts_with("room/")
        && let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor())
        && let Some(at) = super::usercard::near_cursor(app, f64::from(size.width) / scale, f64::from(size.height) / scale)
    {
        // V102-03 A `#channel` chat opens next to where it was clicked, like a usercard,
        // not on top of the last one.
        window.set_position(at)?;
    }
    icons::apply(&window.as_ref().window());
    window.show()?;
    window.set_focus()?;

    // Titles are kept so saved layouts can reopen panels.
    let patch = json!({ "panels": { key: { "title": title } } });
    let _ = settings.patch(app, Partition::Windows, &patch, false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{label_for, pops_out};

    #[test]
    fn labels_only_use_safe_characters() {
        assert_eq!(label_for("settings"), "panel-settings");
        assert_eq!(label_for("chat/12345"), "panel-chat_12345");
        assert_eq!(label_for("a b:c"), "panel-a_b_c");
    }

    #[test]
    fn settings_never_pop_out() {
        assert!(!pops_out("settings"));
        assert!(pops_out("streammanager"));
        assert!(pops_out("modview/xqc"));
    }
}
