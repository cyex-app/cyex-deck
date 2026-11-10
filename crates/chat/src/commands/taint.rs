//! V110-F87 Text an automation didn't write itself (the chatter's words, their name, a web
//! answer) is marked while a line is filled in, so nothing that reads the line later takes it
//! for a command, a `/chain` or `/foreach` split, or a body to fill in again. Every such
//! character gets [`MARK`] in front; `|` and `>` become stand-ins, so no split finds them at
//! any depth. The marks hold through trimming and splitting, since each character carries its
//! own. [`clean`] takes them off right before the text is used.

/// Unicode noncharacters: never in real text, and Twitch has no use for them.
const MARK: char = '\u{FDD0}';
const PIPE: char = '\u{FDD1}';
const GT: char = '\u{FDD2}';

fn ours(c: char) -> bool {
    matches!(c, MARK | PIPE | GT)
}

/// `value` with every character marked. Marks already in it are dropped: a chatter can't
/// bring their own.
pub fn mark(value: &str) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for c in value.chars() {
        match c {
            c if ours(c) => {}
            '|' => out.push(PIPE),
            '>' => out.push(GT),
            c => {
                out.push(MARK);
                out.push(c);
            }
        }
    }
    out
}

/// V110-F94 `value` as `/set var` stores it when someone else's message put it there: [`has`]
/// finds the flag when it is read, [`clean`] gives the value back.
pub fn flag(value: &str) -> String {
    format!("{MARK}{}", clean(value))
}

/// Whether any of `text` is marked.
pub fn has(text: &str) -> bool {
    text.contains(ours)
}

/// `text` as it was before [`mark`].
pub fn clean(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            MARK => None,
            PIPE => Some('|'),
            GT => Some('>'),
            c => Some(c),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marked_text_comes_back_as_it_was() {
        for text in ["", "/ban x", "a | b > c", "  .me  ", "ẞ😀 x"] {
            assert_eq!(clean(&mark(text)), text);
            assert_eq!(has(&mark(text)), !text.is_empty(), "{text}");
        }
        assert!(!has("/ban x | y > z"));
        assert_eq!(clean("plain"), "plain");
    }

    #[test]
    fn marked_text_starts_no_command_and_splits_nothing() {
        // V110-F87 Whatever a later reader looks for, it isn't there, however the text is cut.
        let m = mark(" /ban x | .mod y > //z");
        assert!(!m.contains(['|', '>']));
        for (i, c) in m.char_indices() {
            if matches!(c, '/' | '.') {
                assert!(m[..i].ends_with(MARK), "{c} at {i}");
            }
        }
        assert!(!m.trim().starts_with(['/', '.']));
    }

    #[test]
    fn v110_f94_a_flagged_value_reads_back_clean() {
        for value in ["/ban x", "a | b", ""] {
            assert!(has(&flag(value)));
            assert_eq!(clean(&flag(value)), value);
        }
        assert_eq!(flag(&flag("x")), flag("x"));
    }

    #[test]
    fn a_chatter_cant_bring_marks() {
        let forged = format!("{MARK}{PIPE}/ban{GT}");
        assert_eq!(clean(&mark(&forged)), "/ban");
    }
}
