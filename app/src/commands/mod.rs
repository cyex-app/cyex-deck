//! IPC commands, one file per area. Names are `<area>_<verb>`.

mod settings;
mod theme;
mod windows;

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        settings::settings_all,
        settings::settings_patch,
        settings::settings_reset,
        settings::settings_export_file,
        settings::settings_import_file,
        settings::settings_share_code,
        settings::settings_import_code,
        settings::settings_backups,
        settings::settings_restore_backup,
        settings::settings_open_folder,
        settings::settings_take_load_report,
        theme::theme_accent_tokens,
        windows::window_info,
        windows::window_set_pinned,
        windows::window_open_panel,
        windows::layout_save,
        windows::layout_apply,
        windows::layout_delete,
    ]
}
