//! V107-P7 A message's text for a one-line notice ("Deleted message from x: “…”").

/// Most a quoted text shows, in characters.
pub const QUOTE_MAX: usize = 50;

/// `text` with whitespace collapsed, cut to `QUOTE_MAX` characters then "…".
/// `None` when nothing is left.
pub fn quote(text: &str) -> Option<String> {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return None;
    }
    if flat.chars().count() <= QUOTE_MAX {
        return Some(flat);
    }
    let cut: String = flat.chars().take(QUOTE_MAX).collect();
    // Don't end on a joiner or a space.
    Some(format!("{}…", cut.trim_end_matches(|c: char| c == '\u{200d}' || c.is_whitespace())))
}

#[cfg(test)]
mod tests {
    use super::quote as q;

    #[test]
    fn v107_p7_quote_cuts_at_50_chars() {
        assert_eq!(q(""), None);
        assert_eq!(q("  \t\n"), None);
        assert_eq!(q(" short\n\ntext "), Some("short text".into()));
        let fifty = "a".repeat(50);
        assert_eq!(q(&fifty), Some(fifty.clone()));
        assert_eq!(q(&format!("{fifty}b")), Some(format!("{fifty}…")));
        // Characters, not bytes: 51 emoji keep 50 whole ones.
        assert_eq!(q(&"😀".repeat(51)), Some(format!("{}…", "😀".repeat(50))));
        assert_eq!(q(&"😀".repeat(50)), Some("😀".repeat(50)));
        // No space or joiner right before the "…".
        assert_eq!(q(&format!("{} tail", "a".repeat(49))), Some(format!("{}…", "a".repeat(49))));
        assert_eq!(q(&format!("{}\u{200d}👩", "a".repeat(49))), Some(format!("{}…", "a".repeat(49))));
    }
}
