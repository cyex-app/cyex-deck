//! V102-02 The app's name, in one place: folders, the credential store, user agents and
//! toasts read it from here. `app/tauri.conf.json` repeats `NAME` and `ID`.

/// What people read.
pub const NAME: &str = "cyex deck";
/// Folder names under `%APPDATA%` and `%LOCALAPPDATA%`, the Credential Manager service,
/// user agents.
pub const SLUG: &str = "cyex-deck";
/// The bundle identifier. Windows toasts need it too.
pub const ID: &str = "app.cyex.deck";
/// The folder and credential name before V102-02. Only the one-time copy reads it.
pub const OLD_SLUG: &str = "cyex-viewer";

/// `cyex-deck/0.1.0`, for HTTP requests.
pub fn user_agent() -> String {
    format!("{SLUG}/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_names_the_app() {
        assert!(user_agent().starts_with("cyex-deck/"));
        assert_ne!(SLUG, OLD_SLUG);
    }
}
