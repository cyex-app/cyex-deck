//! F14 (v1.04 batch 2b) Which typed lines are commands. "/" always starts one. A dot does
//! only when the word right after it is a command the app knows (Twitch's, the app's, your
//! own, a plugin's), so ".gg", ".5k is crazy" and ". hi" go out as typed. ".." never does.

/// The command in `text`: its name as typed and what follows it, trimmed. `known` gets the
/// name in lower case and only for a dot.
pub fn command(text: &str, known: impl Fn(&str) -> bool) -> Option<(&str, &str)> {
    let (rest, slash) = match text.strip_prefix('/') {
        Some(rest) => (rest, true),
        None => (text.strip_prefix('.').filter(|r| !r.starts_with('.'))?, false),
    };
    let (name, args) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, ""), |(n, a)| (n, a.trim()));
    (slash || (!name.is_empty() && known(&name.to_ascii_lowercase()))).then_some((name, args))
}

#[cfg(test)]
mod tests {
    use super::command;

    fn known(name: &str) -> bool {
        ["me", "ban", "w", "mycmd"].contains(&name)
    }

    #[test]
    fn a_slash_always_starts_a_command() {
        assert_eq!(command("/ban someone 10", known), Some(("ban", "someone 10")));
        assert_eq!(command("/gg", known), Some(("gg", "")));
        assert_eq!(command("/", known), Some(("", "")));
        assert_eq!(command("/me  waves ", known), Some(("me", "waves")));
    }

    #[test]
    fn a_dot_only_for_known_commands() {
        assert_eq!(command(".me waves", known), Some(("me", "waves")));
        assert_eq!(command(".BAN x", known), Some(("BAN", "x")), "any case");
        assert_eq!(command(".mycmd", known), Some(("mycmd", "")));
        assert_eq!(command(".w\tfriend hi", known), Some(("w", "friend hi")));
        for text in [
            ".gg",
            ".5k is crazy",
            ". hi",
            ".",
            "..",
            "...",
            "..me hi",
            ".gg.",
            ".bans x",
            "gg .ban",
        ] {
            assert_eq!(command(text, known), None, "{text}");
        }
    }

    #[test]
    fn plain_text_is_text() {
        assert_eq!(command("hello /ban", known), None);
        assert_eq!(command("", known), None);
    }
}
