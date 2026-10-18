//! Saved window layouts (U2): the main window plus every open pop-out, by name.

use std::collections::BTreeMap;

use cyex_settings::Partition;
use cyex_settings::schema::{Layout, PanelWindow, Windows};
use serde_json::json;
use tauri::{AppHandle, Manager};

use super::{MAIN, geometry, open_panel, panels, zoom};
use crate::settings::Settings;

pub fn save_layout(app: &AppHandle, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the layout a name".into());
    }
    let settings = app.state::<Settings>();
    let mut saved: Windows = settings.read(Partition::Windows);

    let main = app
        .get_webview_window(MAIN)
        .and_then(|w| geometry::capture(&w.as_ref().window(), saved.main));
    let mut open = BTreeMap::new();
    for (route, window) in panels::open_panels(app) {
        let previous = saved.panels.get(&route).cloned().unwrap_or_default();
        let geometry = geometry::capture(&window.as_ref().window(), previous.geometry);
        let title = window.title().unwrap_or(previous.title);
        open.insert(
            route,
            PanelWindow {
                title,
                geometry,
                pinned: previous.pinned,
            },
        );
    }

    let layout = Layout {
        name: name.to_owned(),
        main,
        panels: open,
    };
    match saved.layouts.iter_mut().find(|l| l.name == layout.name) {
        Some(existing) => *existing = layout,
        None => saved.layouts.push(layout),
    }
    write_layouts(app, &saved.layouts)
}

pub fn apply_layout(app: &AppHandle, name: &str) -> Result<(), String> {
    let settings = app.state::<Settings>();
    let saved: Windows = settings.read(Partition::Windows);
    let mut layout = saved
        .layouts
        .into_iter()
        .find(|l| l.name == name)
        .ok_or_else(|| format!("No layout called \"{name}\""))?;
    // V103-08 Settings saved as a pop-out stay in the main window.
    layout.panels.retain(|route, _| panels::pops_out(route));

    if let (Some(g), Some(main)) = (layout.main, app.get_webview_window(MAIN)) {
        geometry::apply(&main.as_ref().window(), &g);
        // K2 A layout saved at a lower zoom must not undercut the zoom minimum.
        zoom::fit(app, &main, zoom::zoom_percent(app));
    }

    for (route, window) in panels::open_panels(app) {
        if !layout.panels.contains_key(&route) {
            let _ = window.close();
        }
    }

    // Store each panel's saved state first so opening it restores that state.
    let patch = json!({ "panels": layout.panels });
    settings.patch(app, Partition::Windows, &patch, false)?;
    // V103-22 Every window with chats counts as loading before the first one opens, so a
    // mod view opened first waits for the tab's chat instead of showing its own.
    for route in layout.panels.keys().filter(|r| panels::shows_chats(r)) {
        let label = panels::label_for(route);
        if app.get_webview_window(&label).is_none() {
            crate::chat::shown::loading(app, &label);
        }
    }
    for (route, panel) in &layout.panels {
        let label = panels::label_for(route);
        if let Some(open) = app.get_webview_window(&label) {
            let window = open.as_ref().window();
            if let Some(g) = &panel.geometry {
                geometry::apply(&window, g);
            }
            if zoom::follows(open.label()) {
                zoom::fit(app, &open, zoom::zoom_percent(app));
            }
            let _ = window.set_always_on_top(panel.pinned);
        } else {
            open_panel(app, route, &panel.title, None).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn delete_layout(app: &AppHandle, name: &str) -> Result<(), String> {
    let saved: Windows = app.state::<Settings>().read(Partition::Windows);
    let kept: Vec<Layout> = saved.layouts.into_iter().filter(|l| l.name != name).collect();
    write_layouts(app, &kept)
}

fn write_layouts(app: &AppHandle, layouts: &[Layout]) -> Result<(), String> {
    let patch = json!({ "layouts": layouts });
    app.state::<Settings>()
        .patch(app, Partition::Windows, &patch, true)?;
    Ok(())
}
