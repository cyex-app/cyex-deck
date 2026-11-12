//! Window framework (1.51, 3.9, U2): main window, panel pop-outs, pin, geometry, layouts.

mod geometry;
mod layouts;
mod login;
pub mod paint;
mod placement;
mod private;
pub mod overlay;
mod panels;
pub mod seventv;
pub mod sign_in;
pub mod twitch;
mod usercard;
pub mod zoom;

pub use layouts::{apply_layout, delete_layout, save_layout};
pub use panels::{PanelRegistry, close as close_panel, open_panel, route_of, routes as panel_routes};
pub use usercard::open as open_usercard;

use cyex_settings::Partition;
use cyex_settings::schema::{General, Windows};
use serde_json::json;
use tauri::webview::{NewWindowResponse, PageLoadEvent, PageLoadPayload};
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

/// V1051-02 The app's WebView2 environment (flags and folder). Windows that share it share one
/// browser process. Remote pages (Twitch, 7TV) start from this, without the app-only bits.
pub fn environment<'a>(
    app: &'a AppHandle,
    label: &'a str,
    url: WebviewUrl,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    let builder = WebviewWindowBuilder::new(app, label, url).additional_browser_args(BROWSER_ARGS);
    // T01 A test run keeps its WebView2 folder in the test folder too.
    match crate::paths::webview(app) {
        Some(dir) => builder.data_directory(dir),
        None => builder,
    }
}

/// Shared setup for every app window: frameless, hidden until placed, painted in the theme's
/// background so there's no white flash before the UI loads.
pub fn builder<'a>(
    app: &'a AppHandle,
    label: &'a str,
    url: WebviewUrl,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    environment(app, label, url)
        .decorations(false)
        .visible(false)
        .background_color(crate::theme::background(app))
        // V101-30 Paste in the app's own right-click menu. Only the app's pages run here;
        // plugins are Lua and Twitch's pages have their own window.
        .enable_clipboard_access()
        // Zoom only moves in the app's own steps (appearance.zoom), never by Ctrl+wheel.
        .zoom_hotkeys_enabled(false)
        // V110-F85 Defence in depth: a link, `location` change or `window.open` never takes an
        // app window to a remote page (WebView2 reports main-frame navigations here; the app
        // has no iframes).
        .on_navigation(|u| own_page(u, cfg!(debug_assertions)))
        .on_new_window(|_, _| NewWindowResponse::Deny)
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(PanelRegistry::default());
    app.manage(overlay::ClickThrough::default());
    app.manage(LowMemory::default());
    paint::init(app);
    zoom::init(app);
    let saved: Windows = app.state::<Settings>().read(Partition::Windows);
    let main = builder(app, MAIN, WebviewUrl::default())
        .title(cyex_core::app::NAME)
        .inner_size(1280.0, 800.0)
        .min_inner_size(720.0, 420.0)
        .always_on_top(saved.main_pinned);
    let (main, placed) = geometry::on_builder(app, main, saved.main.as_ref(), (720.0, 420.0), (1280.0, 800.0));
    let main = main.build()?;
    geometry::settle(&main, placed)?;
    geometry::keep_main(app, saved.main.as_ref(), placed);
    icons::apply(&main.as_ref().window());
    crate::sys::frame::attach(&main);
    paint::watch(&main);
    main.show()?;
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        // V110-U50 Back from the tray after a close: the page is awake again.
        set_low_memory(app, MAIN, false);
        let _ = w.set_focus();
        crate::miner::idle::on_visibility(app); // V110-U68 (tray, hotkey)
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
        // V105-18 Pop-ups pin per window; a new one always starts unpinned.
        let Some(route) = route_of(app, window.label()).filter(|r| !usercard::is_popup(r)) else {
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
    // V105-18 A pop-up's pin lives only on its window.
    if route_of(app, label).is_some_and(|r| usercard::is_popup(&r)) {
        return app
            .get_webview_window(label)
            .and_then(|w| w.is_always_on_top().ok())
            .unwrap_or(false);
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
    // V110-F02 A moved or resized window gets a fresh start in the frame watchdog.
    if let WindowEvent::Moved(_) | WindowEvent::Resized(_) = event {
        paint::touch(app, window.label());
    }
    // V106-35 Any cyex window in front stops the taskbar flash (Q7).
    if let WindowEvent::Focused(true) = event {
        crate::toasts::focused(app);
    }
    match event {
        // V110-U50 × and Alt+F4 hide the main window to the tray; Tray -> Quit still exits.
        WindowEvent::CloseRequested { api, .. }
            if close_to_tray(window.label(), &app.state::<Settings>().read::<General>(Partition::General)) =>
        {
            api.prevent_close();
            hide_on_close(app, window);
        }
        WindowEvent::Resized(_) if window.label() == MAIN && hides_to_tray(app, window) => {
            let _ = window.hide();
        }
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => geometry::remember(app, window),
        WindowEvent::ThemeChanged(_) => icons::apply(window),
        // V1051-05 What was dropped, for the upload question's thumbnails.
        WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
            if let Some(uploads) = app.try_state::<crate::upload::Uploads>() {
                crate::upload::dropped(&uploads, paths);
            }
        }
        WindowEvent::Focused(focused) if route_of(app, window.label()).is_some_and(|r| usercard::is_card(&r)) => {
            usercard::on_focus(window, *focused);
        }
        WindowEvent::Destroyed if window.label() == MAIN => app.exit(0),
        WindowEvent::Destroyed => {
            close_views(app, window.label());
            // V108-N04/N23 Topics the page followed; its own cleanup never ran.
            crate::hermes::release(app, window.label());
            crate::chat::shown::gone(app, window.label());
            panels::forget(app, window.label());
            overlay::forget(app, window.label());
            low_memory_windows(app).remove(window.label());
            paint::forget(app, window.label());
            // V110-N13 Its poll and prediction cards' watches.
            crate::streammanager::votes::forget(app, window.label());
        }
        _ => {}
    }
    // V110-U68 The miner slows down while every window is hidden, and wakes when one shows.
    if !matches!(event, WindowEvent::Moved(_) | WindowEvent::DragDrop(_) | WindowEvent::ThemeChanged(_)) {
        crate::miner::idle::on_visibility(app);
    }
}

/// V103-22 A window that can show chats counts as loading until a moment after its page
/// loaded, so the views' own chats don't decide while its chats are still opening.
pub fn on_page_load(webview: &Webview, payload: &PageLoadPayload<'_>) {
    let app = webview.app_handle();
    let label = webview.label();
    zoom::apply(webview);
    let shows_chats = || label == MAIN || route_of(app, label).is_some_and(|r| panels::shows_chats(&r));
    match payload.event() {
        PageLoadEvent::Started => {
            // V110-F01 Once per webview; V110-F02 the new page gets its load grace.
            paint::attach(webview);
            paint::page_started(app, label);
            // V107-R1 A reload opens its views again; the old ones would stay as dead sinks.
            close_views(app, label);
            // V108-N04/N23 The old page's topics; the new one follows them again.
            crate::hermes::release(app, label);
            crate::streammanager::votes::forget(app, label);
            if shows_chats() {
                crate::chat::shown::loading(app, label);
            }
        }
        PageLoadEvent::Finished => {
            crate::chat::shown::loaded(app, label);
            if let Some(w) = app.get_webview_window(label).filter(|w| zoom::follows(w.label())) {
                zoom::fit(app, &w, zoom::zoom_percent(app));
            }
            // V1051-01 The page's origin is known now (at setup it may not be yet).
            if label == MAIN {
                crate::sound::reset_permit(app);
            }
        }
    }
}

/// V107-R1 The chat and feed views of a window that closed or (re)loads its page.
fn close_views(app: &AppHandle, label: &str) {
    crate::chat::close_window(app, label);
    crate::chat::feeds::close_window(app, label);
    crate::feeds::close_window(app, label);
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
    set_low_memory(app, window.label(), window.is_minimized().unwrap_or(false));
}

fn set_low_memory(app: &AppHandle, label: &str, low: bool) {
    let changed = {
        let mut set = low_memory_windows(app);
        if low {
            set.insert(label.to_owned())
        } else {
            set.remove(label)
        }
    };
    if changed && let Some(w) = app.get_webview_window(label) {
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

/// V110-U50 What closing the main window does: stay in the tray when the setting is on.
/// Pop-outs close as before.
pub fn close_to_tray(label: &str, g: &General) -> bool {
    label == MAIN && g.close_to_tray
}

/// V110-U50 Closing the main window with the setting on: hide it like minimize-to-tray does
/// (the page goes low-memory and not visible, so the frame watchdog leaves it alone), and say
/// once that the app is still running.
fn hide_on_close(app: &AppHandle, window: &tauri::Window) {
    let _ = window.hide();
    set_low_memory(app, window.label(), true);
    let settings = app.state::<Settings>();
    if settings.read::<General>(Partition::General).close_to_tray_told {
        return;
    }
    let _ = settings.patch(app, Partition::General, &json!({ "closeToTrayTold": true }), false);
    crate::toasts::show(
        app,
        crate::toasts::Pop {
            title: "cyex deck is still running in the tray.".into(),
            text: String::new(),
            avatar: None,
            open: crate::toasts::Open::Settings { page: "general".into() },
            action: None,
        },
    );
}

/// V110-F85 Whether an app window (main, pop-outs, overlay, usercard) may navigate to `url`:
/// only its own pages, `tauri://` or `tauri.localhost`, the Vite dev server in a debug build,
/// and `about:blank`. A stray link, `location` change or form post to anywhere else is refused.
fn own_page(url: &tauri::Url, debug: bool) -> bool {
    match url.scheme() {
        "tauri" => true,
        "about" => url.as_str() == "about:blank",
        "http" | "https" => match url.host_str() {
            Some("tauri.localhost") => true,
            Some("localhost" | "127.0.0.1") => debug,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod nav_tests {
    use super::own_page;

    fn ok(url: &str, debug: bool) -> bool {
        own_page(&url.parse().unwrap(), debug)
    }

    #[test]
    fn v110_f85_app_windows_stay_on_the_apps_own_pages() {
        for url in [
            "tauri://localhost/index.html",
            "http://tauri.localhost/index.html#/panel/room/x",
            "https://tauri.localhost/",
            "about:blank",
        ] {
            assert!(ok(url, false), "{url}");
        }
        for url in [
            "https://twitch.tv/",
            "https://tauri.localhost.evil.com/",
            "http://tauri.localhost@evil.com/",
            "https://evil.com/?https://tauri.localhost/",
            "http://localhost:5173/",
            "http://127.0.0.1:5173/",
            "file:///C:/Windows/win.ini",
            "about:srcdoc",
            "javascript:alert(1)",
            "data:text/html,hi",
            "blob:https://tauri.localhost/x",
            "ftp://tauri.localhost/",
        ] {
            assert!(!ok(url, false), "{url}");
        }
        // The dev server only in a debug build, and only on the local names.
        assert!(ok("http://localhost:5173/", true));
        assert!(ok("http://127.0.0.1:5173/", true));
        assert!(!ok("http://localhost.evil.com/", true));
        assert!(!ok("https://evil.com/", true));
    }
}
