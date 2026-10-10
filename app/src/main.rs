//! cyex viewer: Tauri shell. Commands live in `commands/`, window handling in `windows/`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod alerts;
mod auth;
mod bell;
mod browse;
mod chat;
mod commands;
mod connections;
mod emotes;
mod hermes;
mod hotkeys;
mod icons;
mod miner;
mod moderation;
mod player;
mod plugins;
mod profile;
mod settings;
mod streamer;
mod streammanager;
mod subs;
mod sys;
mod tabs;
mod theme;
mod toasts;
mod tray;
mod whispers;
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
        .plugin(hotkeys::plugin())
        .register_asynchronous_uri_scheme_protocol(player::SCHEME, player::handle)
        .register_uri_scheme_protocol(theme::SCHEME, theme::handle)
        .setup(|app| {
            let handle = app.handle();
            settings::init(handle)?;
            theme::init(handle);
            streamer::init(handle);
            connections::init(handle);
            auth::init(handle)?;
            hermes::init(handle);
            emotes::init(handle);
            plugins::init(handle);
            chat::init(handle);
            tabs::init(handle);
            player::init(handle);
            browse::init(handle);
            miner::init(handle);
            moderation::init(handle);
            streammanager::init(handle);
            whispers::init(handle);
            bell::init(handle);
            alerts::init(handle);
            windows::init(handle)?;
            tray::init(handle)?;
            hotkeys::init(handle);
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
