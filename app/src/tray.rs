//! Tray icon (U2): left click shows the app. The menu has Show, streamer mode (U4),
//! overlay click-through (U7, only while an overlay is open) and Quit.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Theme, Wry};

use crate::{icons, windows};

/// The menu entries that follow the app's state.
struct Items {
    streamer: CheckMenuItem<Wry>,
    click_through: CheckMenuItem<Wry>,
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show cyex", true, None::<&str>)?;
    let streamer = CheckMenuItem::with_id(app, "streamer", "Streamer mode", true, false, None::<&str>)?;
    let click_through = CheckMenuItem::with_id(
        app,
        "click-through",
        "Overlay click-through",
        false,
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let (line1, line2) = (
        PredefinedMenuItem::separator(app)?,
        PredefinedMenuItem::separator(app)?,
    );
    let menu = Menu::with_items(app, &[&show, &line1, &streamer, &click_through, &line2, &quit])?;
    app.manage(Items {
        streamer,
        click_through,
    });
    set_streamer(app, crate::streamer::state(app).active);
    let theme = app
        .get_webview_window(windows::MAIN)
        .and_then(|w| w.theme().ok())
        .unwrap_or(Theme::Dark);

    TrayIconBuilder::with_id("main")
        .icon(icons::for_theme(theme))
        .tooltip("cyex viewer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => windows::show_main(app),
            "streamer" => crate::streamer::toggle(app),
            "click-through" => windows::overlay::toggle_click_through(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn set_streamer(app: &AppHandle, on: bool) {
    if let Some(items) = app.try_state::<Items>() {
        let _ = items.streamer.set_checked(on);
    }
}

pub fn set_overlays(app: &AppHandle, open: bool, click_through: bool) {
    if let Some(items) = app.try_state::<Items>() {
        let _ = items.click_through.set_enabled(open);
        let _ = items.click_through.set_checked(click_through);
    }
}
