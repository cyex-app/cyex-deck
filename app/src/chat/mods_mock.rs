//! V110-T06 Test builds only: `CYEX_GQL_MODS_MOCK=login,login` makes every signed-in account a
//! moderator of those channels (the room's role and the roles list), so the mod view can be
//! tested without a real mod seat. Needs a test data folder; nothing is sent to Twitch.

use tauri::AppHandle;

const VAR: &str = "CYEX_GQL_MODS_MOCK";

/// The channel logins named by a comma-separated list, lower-cased.
fn parse(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().trim_start_matches('#').to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The channels the mock names; empty when unset or not a test data folder.
pub fn channels(app: &AppHandle) -> Vec<String> {
    if crate::paths::webview(app).is_none() {
        return Vec::new();
    }
    std::env::var(VAR).map(|r| parse(&r)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn t06_list_is_trimmed_lowercased_and_sans_hash() {
        assert_eq!(parse(" Foo,#bar ,,BAZ"), ["foo", "bar", "baz"]);
        assert!(parse("").is_empty());
    }
}
