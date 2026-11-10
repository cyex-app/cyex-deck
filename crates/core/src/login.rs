//! V110-F95 The one Twitch login rule: 1 to 25 letters, digits or `_`, any case.

/// Whether `s` can be a Twitch login. Callers that need lowercase only (an import format)
/// add `&& s == s.to_lowercase()` on top.
pub fn is_login(s: &str) -> bool {
    (1..=25).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A typed name as a login: trimmed, one leading `#` or `@` dropped, lowercased.
pub fn normalize(s: &str) -> Option<String> {
    let s = s.trim();
    let s = s.strip_prefix(['#', '@']).unwrap_or(s);
    let lower = s.to_ascii_lowercase();
    is_login(&lower).then_some(lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_a_login() {
        for (s, ok) in [
            ("xqc", true),
            ("xQc", true),
            ("a_b_1", true),
            ("_", true),
            ("", false),
            ("a b", false),
            ("a-b", false),
            ("#xqc", false),
            ("\u{e9}clair", false),
            ("abcdefghijklmnopqrstuvwxy", true),
            ("abcdefghijklmnopqrstuvwxyz", false),
        ] {
            assert_eq!(is_login(s), ok, "{s:?}");
        }
    }

    #[test]
    fn normalizing() {
        for (s, out) in [
            ("xQc", Some("xqc")),
            ("  @Pokimane ", Some("pokimane")),
            ("#lirik", Some("lirik")),
            ("##lirik", None),
            ("@", None),
            ("", None),
            ("a b", None),
        ] {
            assert_eq!(normalize(s).as_deref(), out, "{s:?}");
        }
    }
}
