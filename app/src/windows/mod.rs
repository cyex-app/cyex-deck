//! Window framework (1.51, 3.9, U2): main window, panel pop-outs, pin, geometry, layouts.

mod geometry;
mod layouts;
mod panels;
mod usercard;

pub use layouts::{apply_layout, delete_layout, save_layout};
pub use panels::{PanelRegistry, open_panel, route_of};
pub use usercard::open as open_usercard;

use cyex_core::color::Theme;
use cyex_settings::Partition;
use cyex_settings::schema::{Appearance, General, Windows};
use serde_json::json;
use tauri::window::Color;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};

use crate::icons;
use crate::settings::Settings;

pub const MAIN: &str = "main";

/// WebView2 flags for every window. All webviews in one profile must use the same set.
/// Turns off background services the app never uses (saves ~6 MB, measured 2026/10/09).
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate,\
OptimizationHints,AutofillServerCommunication --disable-background-networking --process-per-site";

/// Shared setup for every window: frameless, hidden until placed, painted in the theme's
/// background so there's no white flash before the UI loads.
pub fn builder<'a>(
    app: &'a AppHandle,
    label: &'a str,
    url: WebviewUrl,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, url)
        .decorations(false)
        .visible(false)
        .background_color(background(app))
        .additional_browser_args(BROWSER_ARGS)
}

fn background(app: &AppHandle) -> Color {
    // Keep in sync with --bg in ui/src/design/tokens.css.
    match app
        .state::<Settings>()
        .read::<Appearance>(Partition::Appearance)
        .theme
    {
        Theme::Light => Color(0xf3, 0xf3, 0xf4, 0xff),
        Theme::Dark => Color(0x1e, 0x1f, 0x23, 0xff),
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(PanelRegistry::default());
    let saved: Windows = app.state::<Settings>().read(Partition::Windows);
    let main = builder(app, MAIN, WebviewUrl::default())
        .title("cyex viewer")
        .inner_size(1280.0, 800.0)
        .min_inner_size(720.0, 420.0)
        .always_on_top(saved.main_pinned);
    let main = geometry::on_builder(app, main, saved.main.as_ref());
    let main = main.build()?;
    if saved.main.is_some_and(|g| g.maximized) {
        main.maximize()?;
    }
    icons::apply(&main.as_ref().window());
    main.show()?;
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Persists a window's pinned (always on top) state.
pub fn set_pinned(app: &AppHandle, window: &tauri::Window, pinned: bool) -> Result<(), String> {
    window.set_always_on_top(pinned).map_err(|e| e.to_string())?;
    let patch = if window.label() == MAIN {
        json!({ "mainPinned": pinned })
    } else {
        // Usercards pin per window; a new one always starts unpinned.
        let Some(route) = route_of(app, window.label()).filter(|r| !usercard::is_usercard(r)) else {
            return Ok(());
        };
        json!({ "panels": { route: { "pinned": pinned } } })
    };
    app.state::<Settings>()
        .patch(app, Partition::Windows, &patch, false)?;
    Ok(())
}

pub fn is_pinned(app: &AppHandle, label: &str) -> bool {
    let saved: Windows = app.state::<Settings>().read(Partition::Windows);
    if label == MAIN {
        return saved.main_pinned;
    }
    route_of(app, label)
        .and_then(|route| saved.panels.get(&route).map(|p| p.pinned))
        .unwrap_or(false)
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let app = window.app_handle();
    match event {
        WindowEvent::Resized(_) if window.label() == MAIN && hides_to_tray(app, window) => {
            let _ = window.hide();
        }
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => geometry::remember(app, window),
        WindowEvent::ThemeChanged(_) => icons::apply(window),
        WindowEvent::Focused(focused) if route_of(app, window.label()).is_some_and(|r| usercard::is_usercard(&r)) => {
            usercard::on_focus(window, *focused);
        }
        WindowEvent::Destroyed if window.label() == MAIN => app.exit(0),
        WindowEvent::Destroyed => {
            crate::chat::close_window(app, window.label());
            panels::forget(app, window.label());
        }
        _ => {}
    }
}

/// U2: a minimized main window goes to the tray when the setting is on.
fn hides_to_tray(app: &AppHandle, window: &tauri::Window) -> bool {
    window.is_minimized().unwrap_or(false)
        && app
            .state::<Settings>()
            .read::<General>(Partition::General)
            .minimize_to_tray
}
