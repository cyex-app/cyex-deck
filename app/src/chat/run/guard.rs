//! V110-F87 What an automation's line may do. A rule's answer holds text the rule didn't
//! write (the chatter's words, their name, a web answer); `commands::taint` marks it while
//! the line is filled in. Only the rule's own text can name a command, at any depth: marked
//! text is said as text, is never filled in again, and never becomes a line to run later.

use cyex_chat::commands::taint;

use super::super::typed;

pub const NAME: &str = "the command name can’t come from the message.";
pub const REFILL: &str = "// can’t take text from the message.";
pub const TIMER: &str = "/timer can’t take text from the message.";
pub const LOOP: &str = "/foreach’s command can’t come from the message.";
pub const PLUGIN: &str = "a plugin’s answer can’t run a command here.";

/// The built-ins that run what follows them as lines: they get their arguments still marked.
const NESTS: &[&str] = &["chain", "foreach", "runin"];

/// A line that names no command, ready to send: the marks off, and anything Twitch could read
/// as a command (after spaces and characters nobody sees) behind `typed::INVISIBLE`.
pub fn say(text: &str) -> String {
    let plain = taint::clean(text);
    let seen = plain.trim_start_matches(|c: char| c.is_whitespace() || c.is_control() || unseen(c));
    if seen.starts_with(['/', '.']) {
        format!("{}{plain}", typed::INVISIBLE)
    } else {
        plain
    }
}

/// Zero-width and other characters that show nothing, which a server may drop from the start.
fn unseen(c: char) -> bool {
    matches!(
        c,
        '\u{AD}' | '\u{34F}' | '\u{61C}' | '\u{115F}' | '\u{1160}' | '\u{17B4}' | '\u{17B5}'
            | '\u{180B}'..='\u{180F}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}' | '\u{2800}' | '\u{3164}' | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}'
            | '\u{FFA0}' | '\u{FFF0}'..='\u{FFFB}' | '\u{1D173}'..='\u{1D17A}' | '\u{E0000}'..='\u{E0FFF}'
    )
}

/// The name after a `/` the rule wrote must be the rule's too, all of it.
pub fn name(name: &str) -> Result<(), String> {
    if taint::has(name) { Err(NAME.into()) } else { Ok(()) }
}

/// The arguments a built-in gets: plain, or still marked for the ones that run lines. `/timer`
/// keeps its line for later, where no mark would be left to stop it.
pub fn args(name: &str, marked: &str) -> Result<String, String> {
    if name == "timer" && taint::has(marked) {
        return Err(TIMER.into());
    }
    // V110-F94 `/set` too: it stores the value with a flag when the message put it there.
    // V110-F99 And `/spam`: it has to tell its own `/me` from the message's.
    Ok(if NESTS.contains(&name) || matches!(name, "set" | "spam") { marked.to_owned() } else { plain(marked) })
}

/// V110-F94 `/spam`'s text in a guarded line: the entry may write `/me`, anything else that
/// Twitch could read as a command goes behind `typed::INVISIBLE`.
/// V110-F99 `marked` is the text with its marks still on: only a `/me ` the entry wrote itself
/// counts, a chatter's own "/me dances" is said as text.
pub fn spam(marked: &str) -> String {
    let plain = taint::clean(marked);
    if marked.starts_with("/me ") { plain } else { say(&plain) }
}

/// V110-F99 `/spam 3 text` without its count, marks kept.
pub fn after_first_word(marked: &str) -> &str {
    marked.trim().split_once(char::is_whitespace).map_or("", |(_, rest)| rest.trim_start())
}

/// V110-F99 What a built-in is about to send that it got from elsewhere (a translation of the
/// message): text, whatever it starts with.
pub fn sent(place_guarded: bool, text: String) -> String {
    if place_guarded { say(&text) } else { text }
}

/// Marked text for a reader that takes plain words (a custom command's `$1`, a plugin).
pub fn plain(marked: &str) -> String {
    taint::clean(marked).trim().to_owned()
}

/// `/foreach`'s command and a `//` body are filled in again: they must be the rule's own text.
pub fn own(text: &str, why: &str) -> Result<(), String> {
    if taint::has(text) { Err(why.into()) } else { Ok(()) }
}

/// What a plugin's command answered: text to say, never a command.
pub fn plugin_answer(next: &str, known: impl Fn(&str) -> bool) -> Result<String, String> {
    let next = next.trim();
    if next.starts_with("//") || typed::command(next, known).is_some() {
        return Err(PLUGIN.into());
    }
    Ok(taint::mark(next))
}

#[cfg(test)]
#[path = "guard_tests.rs"]
mod tests;
