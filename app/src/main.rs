//! cyex viewer: Tauri shell. Commands live in `commands/`, window handling in `windows/`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod browse;
mod chat;
mod commands;
mod connections;
mod icons;
mod player;
mod settings;
mod tabs;
mod tray;
mod windows;

use tauri::RunEvent;

fn main() {
    let app = tauri::Builder::default()
        // Must be the first plugin: a second launch focuses the running app instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_main(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol(player::SCHEME, player::handle)
        .setup(|app| {
            let handle = app.handle();
            settings::init(handle)?;
            connections::init(handle);
            auth::init(handle)?;
            chat::init(handle);
            tabs::init(handle);
            player::init(handle);
            browse::init(handle);
            windows::init(handle)?;
            tray::init(handle)?;
            Ok(())
        })
        .on_window_event(windows::on_window_event)
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())
        .expect("cyex viewer failed to start");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            settings::flush_now(app);
        }
    });
}
