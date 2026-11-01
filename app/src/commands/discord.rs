//! V108-N26 IPC for Discord Rich Presence.

use crate::discord::{self, Watch};

/// Whether this build knows a Discord app ID. When it doesn't, the setting is greyed out.
#[tauri::command]
pub fn discord_available() -> bool {
    discord::available()
}

/// The channel playing in the main window, or `None` for nothing (or the setting off).
#[tauri::command]
pub fn discord_watch(watch: Option<Watch>) {
    discord::watch(watch);
}
