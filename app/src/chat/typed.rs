//! F14 (v1.04 batch 2b) Which typed lines are commands. "/" always starts one. A dot does
//! only when the word right after it is a command the app knows (Twitch's, the app's, your
//! own, a plugin's), so ".gg", ".5k is crazy" and ". hi" go out as typed. ".." never does.
//! F20 Twitch itself reads a line that starts like a command as one, so those get [`escape`]d.

/// Invisible, and Twitch keeps it (Chatterino's duplicate suffix is " \u{34F}").
pub const INVISIBLE: char = '\u{34F}';

/// F20 Text Twitch would take for a command gets [`INVISIBLE`] in front, so it arrives as
/// typed instead of "unrecognized command" or cut (". hi" would post "hi"). Twitch's rule as
/// Chatterino has it (`isUnknownCommand`): "/", or a dot before anything but a dot or the end.
/// "/me" and ".me" stay Twitch's action (text sent through Helix or GQL can hold them).
pub fn escape(text: String) -> String {
    let rest = text.strip_prefix('/').or_else(|| {
        text.strip_prefix('.')
            .filter(|r| !r.is_empty() && !r.starts_with('.'))
    });
    let Some(rest) = rest else {
        return text;
    };
    let me = rest.get(..2).is_some_and(|w| w.eq_ignore_ascii_case("me"))
        && rest[2..].chars().next().is_none_or(char::is_whitespace);
    if me { text } else { format!("{INVISIBLE}{text}") }
}

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
    use super::{INVISIBLE, command, escape};

    fn escaped(text: &str) -> bool {
        escape(text.to_owned()) == format!("{INVISIBLE}{text}")
    }

    #[test]
    fn twitch_commands_get_escaped() {
        for text in [
            ".gg",
            ".5k is crazy",
            ". hi",
            ".gg.",
            ".mewaves",
            ". .gg",
            ".\u{34F}",
            "/gg",
            "/",
            "/ hi",
        ] {
            assert!(escaped(text), "{text}");
        }
        assert_eq!(escape(".gg".into()), "\u{34F}.gg");
        assert_eq!(
            escape(". hi".into()).chars().skip(1).collect::<String>(),
            ". hi",
            "the rest stays"
        );
    }

    #[test]
    fn plain_text_stays_as_is() {
        for text in [
            "gg",
            "hi .gg",
            ".",
            "..",
            "...",
            "..gg",
            "... hi",
            "",
            " .gg",
            "\u{34F}.gg",
            "/me waves",
            ".me waves",
            ".ME\twaves",
            "/me",
        ] {
            assert_eq!(escape(text.to_owned()), text, "{text}");
        }
    }

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
