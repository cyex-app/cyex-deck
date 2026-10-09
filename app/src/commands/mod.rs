//! IPC commands, one file per area. Names are `<area>_<verb>`.

mod accounts;
mod chat;
mod settings;
mod system;
mod tabs;
mod theme;
mod windows;

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        accounts::accounts_snapshot,
        accounts::accounts_sign_in,
        accounts::accounts_cancel_sign_in,
        accounts::accounts_add_token,
        accounts::accounts_remove,
        accounts::accounts_remove_token,
        accounts::accounts_set_default,
        accounts::accounts_set_client_id,
        accounts::connections_counts,
        chat::chat_open,
        chat::chat_close,
        chat::chat_send,
        chat::chat_global_catalog,
        chat::chat_channel_catalog,
        chat::chat_points,
        chat::chat_redeem,
        chat::chat_user_messages,
        chat::chat_resolve_link,
        chat::user_profile,
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
        system::system_open_link,
        system::system_open_chat_link,
        tabs::tabs_apply,
        tabs::tabs_reopen,
        tabs::tabs_reopenable,
        tabs::tabs_unread,
        tabs::tabs_lookup,
        tabs::tabs_live,
        tabs::channel_following,
        tabs::channel_follow,
        theme::theme_accent_tokens,
        windows::window_info,
        windows::window_set_pinned,
        windows::window_set_title,
        windows::window_open_panel,
        windows::window_close_panel,
        windows::window_panels,
        windows::window_open_usercard,
        windows::layout_save,
        windows::layout_apply,
        windows::layout_delete,
    ]
}
