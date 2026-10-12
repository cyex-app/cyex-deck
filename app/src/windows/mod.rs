//! Window framework (1.51, 3.9, U2): main window, panel pop-outs, pin, geometry, layouts.

mod geometry;
mod layouts;
pub mod overlay;
mod panels;
pub mod twitch;
mod usercard;

pub use layouts::{apply_layout, delete_layout, save_layout};
pub use panels::{PanelRegistry, close as close_panel, open_panel, route_of, routes as panel_routes};
pub use usercard::open as open_usercard;

use cyex_settings::Partition;
use cyex_settings::schema::{General, Windows};
use serde_json::json;
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Manager, Webview, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};

use crate::icons;
use crate::settings::Settings;

pub const MAIN: &str = "main";

/// WebView2 flags for every window. All webviews in one profile must use the same set.
/// Turns off background services the app never uses (saves ~6 MB, measured 2026/10/09) and
/// runs the network service inside the browser process (one process fewer, ~8 MB, D13).
/// V103-11 WebView2's own scroll animation stays off: the page glides the wheel itself
/// (`ui/src/lib/smoothScroll.ts`), so the setting applies at once; keys jump, as in Chatterino.
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate,\
OptimizationHints,AutofillServerCommunication --enable-features=NetworkServiceInProcess2 \
--disable-background-networking --process-per-site --disable-smooth-scrolling";

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
        .background_color(crate::theme::background(app))
        .additional_browser_args(BROWSER_ARGS)
        // V101-30 Paste in the app's own right-click menu. Only the app's pages run here;
        // plugins are Lua and Twitch's pages have their own window.
        .enable_clipboard_access()
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(PanelRegistry::default());
    app.manage(overlay::ClickThrough::default());
    app.manage(LowMemory::default());
    let saved: Windows = app.state::<Settings>().read(Partition::Windows);
    let main = builder(app, MAIN, WebviewUrl::default())
        .title(cyex_core::app::NAME)
        .inner_size(1280.0, 800.0)
        .min_inner_size(720.0, 420.0)
        .always_on_top(saved.main_pinned);
    let main = geometry::on_builder(app, main, saved.main.as_ref());
    let main = main.build()?;
    if saved.main.is_some_and(|g| g.maximized) {
        main.maximize()?;
    }
    icons::apply(&main.as_ref().window());
    crate::sys::fill_fullscreen(&main);
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

/// S2 global hotkey: brings the app up, or puts it away when it's already in front.
pub fn toggle_main(app: &AppHandle) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    let in_front = w.is_visible().unwrap_or(false)
        && !w.is_minimized().unwrap_or(false)
        && w.is_focused().unwrap_or(false);
    if in_front {
        let _ = w.minimize(); // goes on to the tray when that's set (U2)
    } else {
        show_main(app);
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
        json!({ "panels": { usercard::settings_key(&route): { "pinned": pinned } } })
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
        .and_then(|route| saved.panels.get(usercard::settings_key(&route)).map(|p| p.pinned))
        .unwrap_or(false)
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let app = window.app_handle();
    if let WindowEvent::Resized(_) = event {
        memory_level(app, window);
    }
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
            crate::chat::shown::gone(app, window.label());
            panels::forget(app, window.label());
            overlay::forget(app, window.label());
            low_memory_windows(app).remove(window.label());
        }
        _ => {}
    }
}

/// V103-22 A window that can show chats counts as loading until a moment after its page
/// loaded, so the views' own chats don't decide while its chats are still opening.
pub fn on_page_load(webview: &Webview, payload: &PageLoadPayload<'_>) {
    let app = webview.app_handle();
    let label = webview.label();
    let shows_chats = || label == MAIN || route_of(app, label).is_some_and(|r| panels::shows_chats(&r));
    match payload.event() {
        PageLoadEvent::Started if shows_chats() => crate::chat::shown::loading(app, label),
        PageLoadEvent::Started => {}
        PageLoadEvent::Finished => crate::chat::shown::loaded(app, label),
    }
}

/// Windows that WebView2 was asked to keep small (D28).
#[derive(Default)]
pub struct LowMemory(std::sync::Mutex<std::collections::HashSet<String>>);

fn low_memory_windows(app: &AppHandle) -> std::sync::MutexGuard<'_, std::collections::HashSet<String>> {
    app.state::<LowMemory>().inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// A minimized window (or one in the tray) lets WebView2 give memory back; it goes back to
/// normal once restored. Each change is sent once.
fn memory_level(app: &AppHandle, window: &tauri::Window) {
    let low = window.is_minimized().unwrap_or(false);
    let changed = {
        let mut set = low_memory_windows(app);
        if low {
            set.insert(window.label().to_owned())
        } else {
            set.remove(window.label())
        }
    };
    if changed && let Some(w) = app.get_webview_window(window.label()) {
        crate::sys::low_memory(&w, low);
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
