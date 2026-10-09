//! Theme commands (3.3, 3.4).

use cyex_core::color::{AccentTokens, Theme, accent_tokens};

#[tauri::command]
pub fn theme_accent_tokens(accent: String, theme: Theme) -> AccentTokens {
    accent_tokens(&accent, theme)
}
