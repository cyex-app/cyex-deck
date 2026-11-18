//! V111-06 Commands from Chatterino's commands.json (`{"commands": [{name, func,
//! showInMsgContextMenu}]}`). Each `func` is read the way Chatterino's
//! `CommandController::execCustomCommand` reads it (its regex, `{{` and the leading-`{` quirk
//! included) and written again in this app's syntax. What has no match here is skipped with a
//! reason; what changes on the way in carries a note. Nothing else in the file is read.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::Skipped;
use crate::schema::{CommandRow, Commands, MAX_LINES};

/// Commands read from one file; the rest are counted in `unread`.
pub const MAX_READ: usize = 2000;
/// V110-F90 Longest name kept, with its "/".
const NAME_MAX: usize = 32;
/// The longest row a command list keeps (`schema::commands`).
const ROW_MAX: usize = 2000;
/// The submenu imported message-menu entries go in.
pub const SUBMENU: &str = "@Commands";
/// The note row above imported commands.
pub const NOTE: &str = "# From Chatterino";

/// The file isn't Chatterino's commands.json.
#[derive(Debug, PartialEq, Eq)]
pub struct NotCommands;

/// One command that can go in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incoming {
    /// As Chatterino has it.
    pub name: String,
    /// Lower case, without the "/": what the app looks it up by.
    pub key: String,
    /// `/name body`.
    pub row: String,
    pub notes: Vec<String>,
    /// V111-56 What it does here, in plain words.
    pub does: String,
    /// Chatterino showed it in the message menu: our entry (`.label=/name`) or why there's none.
    pub menu: Option<Result<String, String>>,
    /// V111-56 The entry when the name stays cyex's or yours: the body itself, if it maps.
    pub menu_kept: Option<String>,
}

/// What the file holds, in its order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub commands: Vec<Incoming>,
    pub skipped: Vec<Skipped>,
    /// Commands after the first `MAX_READ`.
    pub unread: u32,
}

/// What a command's name runs into here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Conflict {
    None,
    /// One of your commands has the name.
    Yours,
    /// The app's or Twitch's own command has it.
    BuiltIn,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandItem {
    pub id: u32,
    /// As Chatterino has it.
    pub name: String,
    /// What goes in the list.
    pub row: String,
    pub notes: Vec<String>,
    /// V111-56 What it does here, in plain words.
    pub does: String,
    pub conflict: Conflict,
    /// Your row with that name.
    pub yours: Option<String>,
    /// The message-menu entry, if Chatterino showed it there and it maps.
    pub menu: Option<String>,
    /// Why Chatterino's menu entry can't come along.
    pub menu_note: Option<String>,
    /// V111-56 The entry that comes along when the name stays cyex's or yours.
    pub menu_kept: Option<String>,
}

/// What importing would do, before the choices.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandPreview {
    pub items: Vec<CommandItem>,
    pub skipped: Vec<Skipped>,
    /// Rows left in the command list.
    pub room: u32,
    /// Rows left in the message menu.
    pub menu_room: u32,
    pub limit: u32,
    pub unread: u32,
}

/// What an import did.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandImport {
    pub added: u32,
    pub replaced: u32,
    /// Conflicts that kept yours or the built-in.
    pub kept: u32,
    /// Names that didn't fit under the limit.
    pub cut: Vec<String>,
    /// Message-menu entries added.
    pub menu: u32,
    pub menu_cut: u32,
}

fn skip(name: &str, reason: &str) -> Skipped {
    Skipped {
        pattern: name.to_owned(),
        reason: reason.to_owned(),
    }
}

// ---- Chatterino's placeholders ----

/// A piece of a func after Chatterino's regex: text, `{1}` / `{1+}`, or one of its variables.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Text(String),
    /// V111-59 With its alt text too (`{2;none}`).
    Word { from: u32, rest: bool, alt: Option<String> },
    Var { name: &'static str, alt: Option<String> },
}

/// The variables `COMMAND_VARS` knows (case matters there too). Anything else in braces is text.
/// V111-56 With `{user.id}` and `{user.display_name}` from newer builds. V111-59 And
/// `{time}` and `{user.last_message.*}` (docs/research/chatterino-placeholders.md).
pub const VARS: [&str; 20] = [
    "channel.name",
    "channel.id",
    "channel",
    "stream.game",
    "stream.title",
    "my.id",
    "my.name",
    "user.name",
    "user.id",
    "user.display_name",
    "user",
    "msg.id",
    "msg-id",
    "msg.text",
    "message",
    "input.text",
    "element.copytext",
    "time",
    "user.last_message.id",
    "user.last_message.text",
];

enum Body {
    Digits { digits: String, plus: bool, alt: Option<String> },
    Name { name: String, alt: Option<String> },
}

struct Match {
    /// Where the match starts: the character before the braces, or 0 for `^`.
    start: usize,
    end: usize,
    body: Body,
}

/// V111-56 `[a-zA-Z_.-][a-zA-Z0-9_.-]*`, as newer builds read names (`{user.display_name}`).
fn is_name_char(c: char, first: bool) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '.' | '-' | '_') || (!first && c.is_ascii_digit())
}

/// `({{)*{(\d+\+?|([a-zA-Z_.-][a-zA-Z0-9_.-]*)(?:;(.+?))?)}` at `q`: the end and the body.
fn braces_at(s: &[char], q: usize) -> Option<(usize, Body)> {
    let mut b = q;
    while s.get(b) == Some(&'{') {
        b += 1;
    }
    // Pairs, then one: a body never starts with "{", so every brace in the run is used.
    if (b - q).is_multiple_of(2) {
        return None;
    }
    let first = *s.get(b)?;
    if first.is_ascii_digit() {
        let mut e = b;
        while s.get(e).is_some_and(char::is_ascii_digit) {
            e += 1;
        }
        let digits: String = s[b..e].iter().collect();
        let plus = s.get(e) == Some(&'+');
        let (end, alt) = close(s, e + usize::from(plus))?;
        return Some((end, Body::Digits { digits, plus, alt }));
    }
    if !is_name_char(first, true) {
        return None;
    }
    let mut e = b + 1;
    while s.get(e).is_some_and(|&c| is_name_char(c, false)) {
        e += 1;
    }
    let name: String = s[b..e].iter().collect();
    let (end, alt) = close(s, e)?;
    Some((end, Body::Name { name, alt }))
}

/// `(?:;(.+?))?}` at `e`: the end and the alt text.
fn close(s: &[char], e: usize) -> Option<(usize, Option<String>)> {
    match s.get(e) {
        Some('}') => Some((e + 1, None)),
        Some(';') => {
            // `.+?`: at least one character, no line break, up to the first "}" after it.
            let from = e + 1;
            let mut j = from + 1;
            if s.get(from).is_none_or(|&c| c == '\n') {
                return None;
            }
            loop {
                match s.get(j)? {
                    '}' => break,
                    '\n' => return None,
                    _ => j += 1,
                }
            }
            let alt: String = s[from..j].iter().collect();
            Some((j + 1, Some(alt)))
        }
        _ => None,
    }
}

/// The leftmost match of `(^|[^{])…` from `offset`.
fn find(s: &[char], offset: usize) -> Option<Match> {
    for p in offset..s.len() {
        if p == 0
            && let Some((end, body)) = braces_at(s, 0)
        {
            return Some(Match { start: 0, end, body });
        }
        if s[p] != '{'
            && let Some((end, body)) = braces_at(s, p + 1)
        {
            return Some(Match { start: p, end, body });
        }
    }
    None
}

/// The func as Chatterino reads it.
fn pieces(func: &str) -> Vec<Piece> {
    let s: Vec<char> = func.chars().collect();
    let mut out: Vec<Piece> = Vec::new();
    let mut text = String::new();
    let mut last = 0;
    let mut offset = 0;
    while let Some(m) = find(&s, offset) {
        // Up to and with the character before the braces; the `{{` pairs fall out.
        if m.start + 1 > last {
            text.extend(&s[last..=m.start]);
        }
        last = m.end;
        offset = m.end - 1;
        let piece = match m.body {
            Body::Digits { digits, plus, alt } => match digits.parse::<i32>() {
                Ok(n) if n > 0 => Some(Piece::Word {
                    from: n.unsigned_abs(),
                    rest: plus,
                    alt,
                }),
                // `{0}` and numbers too big for an int stay as they are.
                _ => {
                    text.push('{');
                    text.push_str(&digits);
                    if plus {
                        text.push('+');
                    }
                    if let Some(a) = alt {
                        text.push(';');
                        text.push_str(&a);
                    }
                    text.push('}');
                    None
                }
            },
            Body::Name { name, alt } => match VARS.iter().find(|v| **v == name) {
                Some(v) => Some(Piece::Var { name: v, alt }),
                None => {
                    text.push('{');
                    text.push_str(&name);
                    if let Some(a) = alt {
                        text.push(';');
                        text.push_str(&a);
                    }
                    text.push('}');
                    None
                }
            },
        };
        if let Some(p) = piece {
            if !text.is_empty() {
                out.push(Piece::Text(std::mem::take(&mut text)));
            }
            out.push(p);
        }
    }
    text.extend(&s[last.min(s.len())..]);
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    // Chatterino drops a leading "{" (left over from a placeholder at the start), then reads
    // every "{{" as "{".
    if let Some(Piece::Text(t)) = out.first_mut()
        && t.starts_with('{')
    {
        t.remove(0);
    }
    for p in &mut out {
        if let Piece::Text(t) = p {
            *t = t.replace("{{", "{");
        }
    }
    out.retain(|p| !matches!(p, Piece::Text(t) if t.is_empty()));
    out
}

// ---- Our syntax ----

/// `{` that would start one of our 7TV variables (`commands::vars`).
fn opens_var(rest: &[char]) -> bool {
    let Some(close) = rest.iter().position(|&c| c == '}') else {
        return false;
    };
    let inner: String = rest[..close].iter().collect::<String>().to_ascii_lowercase();
    matches!(
        inner.as_str(),
        "paint" | "paint.name" | "paint.id" | "badge" | "badge.name" | "badge.id" | "combo"
    ) || inner.starts_with("combo:")
}

/// Text that stays text: `\`, `$` and a `{paint}`-like brace escaped; in a function's
/// parameter also `,` and `)`.
fn escape(text: &str, in_param: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let special = match c {
            '\\' | '$' => true,
            '{' => opens_var(&chars[i + 1..]),
            ',' | ')' => in_param,
            _ => false,
        };
        if special {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// One of our values, or its alt text when it's empty.
fn value(name: &str, alt: Option<&str>, never_empty: bool) -> String {
    match alt {
        Some(a) if !never_empty => format!("$if({name},$({name}),{})", escape(a, true)),
        _ => format!("$({name})"),
    }
}

/// V111-56 Chatterino's variables that come from the message or name a menu was opened on.
/// Here a menu entry hands them to every command it runs, so typed they're empty (or the alt
/// text) and from the menu they're filled in, as in Chatterino.
fn from_menu(name: &str) -> bool {
    matches!(
        name,
        "user.name"
            | "user"
            | "user.id"
            | "user.display_name"
            | "msg.id"
            | "msg-id"
            | "msg.text"
            | "message"
            | "input.text"
            | "element.copytext"
    )
}

/// A Chatterino variable in our syntax.
fn var(name: &str, alt: Option<&str>) -> String {
    match name {
        "channel.name" | "channel" => value("chan", alt, true),
        // V111-56
        "channel.id" => value("chan-id", alt, false),
        "stream.game" => value("streamgame", alt, false),
        "stream.title" => value("streamtitle", alt, false),
        "my.id" => value("my-user-id", alt, false),
        "my.name" => value("my-nick", alt, false),
        "user.name" | "user" => value("nick", alt, false),
        "user.id" => value("user-id", alt, false),
        "user.display_name" => value("display-nick", alt, false),
        "msg.id" | "msg-id" => value("msg-id", alt, false),
        "msg.text" | "message" => value("msg", alt, false),
        // V111-59
        "input.text" => value("input-text", alt, false),
        "element.copytext" => value("copy-text", alt, false),
        "time" => value("time", alt, true),
        "user.last_message.id" => value("last-msg-id", alt, false),
        "user.last_message.text" => value("last-msg", alt, false),
        // Only `VARS` get here.
        _ => alt.map(|a| escape(a, false)).unwrap_or_default(),
    }
}

/// The pieces as a body of ours. `inline`: for a menu entry that runs the body itself, where
/// Chatterino's words are the message (`{1+}` is our `$(msg)`).
fn body(pieces: &[Piece], inline: bool) -> Result<String, String> {
    let mut out = String::new();
    for (i, p) in pieces.iter().enumerate() {
        match p {
            Piece::Text(t) => out.push_str(&escape(t, false)),
            Piece::Word { from, rest, alt } => {
                if inline {
                    if *from == 1 && *rest {
                        out.push_str(&value("msg", alt.as_deref(), false));
                        continue;
                    }
                    return Err("Uses single words of the message.".into());
                }
                let tail = if *rest { "-" } else { "" };
                // V111-59 `{2;none}`: the alt text when there's no such word.
                if let Some(a) = alt {
                    out.push_str(&format!("$if({from}{tail},${from}{tail},{})", escape(a, true)));
                    continue;
                }
                // `$1` right before a "-" would read as `$1-`.
                let dash = matches!(pieces.get(i + 1), Some(Piece::Text(t)) if t.starts_with('-'));
                if *from > 9 || (dash && !rest) {
                    out.push_str(&format!("$({from}{tail})"));
                } else {
                    out.push_str(&format!("${from}{tail}"));
                }
            }
            Piece::Var { name, alt } => out.push_str(&var(name, alt.as_deref())),
        }
    }
    Ok(out)
}

/// V111-56 Commands Chatterino (or a common plugin) has under another name: ours, or `""`
/// when the text goes out as it is.
const ALIASES: [(&str, &str); 3] = [("pinmessage", "pin"), ("unpinmessage", "unpin"), ("send", "")];

/// The `/command` the func starts with, when the whole word is text (not half a placeholder).
fn head(ps: &[Piece]) -> Option<String> {
    let Some(Piece::Text(t)) = ps.first() else { return None };
    let whole = ps.len() == 1 || t.trim_start().contains(char::is_whitespace);
    let first = t.split_whitespace().next()?;
    let cmd = first.strip_prefix('/').filter(|c| !c.is_empty() && !c.starts_with('/'))?;
    whole.then(|| cmd.to_owned())
}

/// V111-56 Swaps a Chatterino command for ours (`ALIASES`); the note says so.
fn map_head(ps: &mut Vec<Piece>) -> Option<String> {
    let cmd = head(ps)?;
    let lower = cmd.to_ascii_lowercase();
    let (_, ours) = ALIASES.iter().find(|(from, _)| *from == lower)?;
    let Some(Piece::Text(t)) = ps.first_mut() else { return None };
    let rest = t.trim_start()[1 + cmd.len()..].to_owned();
    let note = if ours.is_empty() {
        *t = rest.trim_start().to_owned();
        format!("Sends the text itself; cyex has no /{cmd}.")
    } else {
        *t = format!("/{ours}{rest}");
        format!("Runs /{ours}, cyex’s name for /{cmd}.")
    };
    if t.is_empty() {
        ps.remove(0);
    }
    Some(note)
}

/// What one of our commands does, for the dialog.
fn describe(cmd: &str) -> Option<&'static str> {
    Some(match cmd {
        "pin" => "Pins a message.",
        "unpin" => "Takes the pin down.",
        "copy" => "Copies it to the clipboard.",
        "translate" => "Translates it; only you see that.",
        "tl" => "Sends it translated.",
        "follow" => "Follows that channel.",
        "unfollow" => "Unfollows that channel.",
        "namehistory" => "Lists their past names.",
        "user" | "usercard" => "Opens their user card.",
        "logs" => "Opens their logs.",
        "me" => "Sends it as an action.",
        "echo" => "Shows a line only you see.",
        "w" | "whisper" => "Sends a whisper.",
        "ban" => "Bans someone.",
        "timeout" => "Times someone out.",
        "delete" => "Deletes a message.",
        "shoutout" | "so" => "Gives a shoutout.",
        _ => return None,
    })
}

/// "a", "a and b", "a, b and c".
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// V111-56 What the command does here, in plain words: what it runs, and where its message
/// values come from.
fn does(ps: &[Piece], in_menu: bool, known: &dyn Fn(&str) -> bool, own: &dyn Fn(&str) -> bool) -> String {
    let mut out = match head(ps) {
        Some(cmd) => {
            let lower = cmd.to_ascii_lowercase();
            match describe(&lower) {
                Some(d) if known(&lower) => d.to_owned(),
                _ if known(&lower) => format!("Runs /{cmd}."),
                _ if own(&lower) => format!("Runs your /{cmd}."),
                _ => format!("Can’t run here: cyex has no /{cmd}."),
            }
        }
        None if matches!(ps.first(), Some(Piece::Word { .. })) => "Sends what you type after it.".to_owned(),
        None => "Sends it to chat.".to_owned(),
    };
    let mut seen: Vec<String> = Vec::new();
    let mut alt = false;
    for p in ps {
        if let Piece::Var { name, alt: a } = p
            && from_menu(name)
        {
            let shown = format!("{{{name}}}");
            if !seen.contains(&shown) {
                seen.push(shown);
            }
            alt |= a.is_some();
        }
    }
    if !seen.is_empty() {
        let typed = match (seen.len(), alt) {
            (1, false) => "it’s empty",
            (1, true) => "it’s the text after the ;",
            (_, false) => "they’re empty",
            (_, true) => "they’re empty or the text after the ;",
        };
        out.push_str(&format!(" From a menu it fills in {}; typed, {typed}.", and_list(&seen)));
    }
    // V111-59
    if ps.iter().any(|p| matches!(p, Piece::Var { name, .. } if name.starts_with("user.last_message"))) {
        out.push_str(" Reads their newest message in this chat: the name from a menu, else the first word.");
    }
    if in_menu && ps.iter().any(|p| matches!(p, Piece::Word { .. })) {
        out.push_str(" From the message menu, the message counts as what you type after it.");
    }
    out
}

/// Chatterino's name as ours (`/name`), with a note when it changed, or why it can't be one.
fn name(raw: &str) -> Result<(String, Option<String>), String> {
    let n = raw.trim();
    if n.is_empty() {
        return Err("No name.".into());
    }
    if n.contains(char::is_whitespace) {
        return Err("Names with spaces don’t work here.".into());
    }
    if n.contains('#') {
        return Err("A # in a name means one channel here.".into());
    }
    if n.starts_with("//") {
        return Err("Names can’t start with // here.".into());
    }
    let (ours, note) = if n.starts_with('/') {
        (n.to_owned(), None)
    } else {
        let mut chars = n.chars();
        let first = chars.next().unwrap_or_default();
        if first == '.' {
            let ours = format!("/{}", chars.as_str());
            let note = format!("Saved as {ours}; {n} still works.");
            (ours, Some(note))
        } else if first.is_ascii_punctuation() {
            let ours = format!("/{}", chars.as_str());
            let note = format!("Typed as {ours} here, not {n}.");
            (ours, Some(note))
        } else {
            let ours = format!("/{n}");
            let note = format!("Typed as {ours} here.");
            (ours, Some(note))
        }
    };
    if ours.len() < 2 {
        return Err("No name.".into());
    }
    if ours.chars().count() > NAME_MAX {
        return Err(format!("Name over {NAME_MAX} characters."));
    }
    Ok((ours, note))
}

/// The menu label: the name without its "/" and without "=".
fn label(name: &str) -> Option<String> {
    let l = name.trim_start_matches('/').replace('=', "");
    (!l.is_empty() && !l.chars().all(|c| c == '-')).then_some(l)
}

fn incoming(
    raw_name: &str,
    raw_func: &str,
    in_menu: bool,
    known: &dyn Fn(&str) -> bool,
    own: &dyn Fn(&str) -> bool,
) -> Result<Incoming, String> {
    let (ours, name_note) = name(raw_name)?;
    let mut notes: Vec<String> = name_note.into_iter().collect();
    let mut func = raw_func.trim().to_owned();
    // V111-56 JavaScript, as some Chatterino builds run it.
    if func.starts_with("#js") {
        return Err("Runs JavaScript, which cyex doesn’t run.".into());
    }
    if func.contains(['\n', '\r']) {
        func = func.split(['\r', '\n']).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join(" ");
        notes.push("Line breaks became spaces.".into());
    }
    if func.is_empty() {
        return Err("Sends nothing.".into());
    }
    // V111-59 Every Chatterino placeholder has a value of ours now.
    let mut ps = pieces(&func);
    notes.extend(map_head(&mut ps));
    let typed = body(&ps, false)?;
    let typed = typed.trim();
    if typed.is_empty() {
        return Err("Sends nothing.".into());
    }
    let row = format!("{ours} {typed}");
    if row.chars().count() > ROW_MAX {
        return Err("Over 2.000 characters.".into());
    }
    let words = ps.iter().any(|p| matches!(p, Piece::Word { .. }));
    let l = label(&ours);
    // V111-56 The entry runs the command by its name with the message as its words, the way
    // Chatterino's menu runs it. When the name stays cyex's or yours, the entry runs the body.
    let menu = in_menu.then(|| {
        let l = l.as_deref().ok_or_else(|| "Its name can’t be a menu label.".to_owned())?;
        let tail = if words { " $(msg)" } else { "" };
        Ok(format!(".{l}={ours}{tail}"))
    });
    let menu_kept = l.filter(|_| in_menu).and_then(|l| {
        let b = body(&ps, true).ok()?;
        let entry = format!(".{l}={}", b.trim());
        (entry.chars().count() <= ROW_MAX).then_some(entry)
    });
    let key = ours[1..].to_ascii_lowercase();
    Ok(Incoming {
        name: raw_name.trim().to_owned(),
        key,
        row,
        notes,
        does: does(&ps, in_menu, known, own),
        menu,
        menu_kept,
    })
}

/// Reads commands.json. `known` says whether a (lower-case) name is one of this app's commands,
/// for the note on a func that runs one it doesn't have.
pub fn parse(text: &str, known: &dyn Fn(&str) -> bool) -> Result<Found, NotCommands> {
    let root: Value = serde_json::from_str(text).map_err(|_| NotCommands)?;
    let list = match root {
        Value::Object(mut o) => match o.remove("commands") {
            Some(Value::Array(a)) => a,
            _ => return Err(NotCommands),
        },
        Value::Array(a) => a,
        _ => return Err(NotCommands),
    };
    // V111-56 A command that runs another one from the same file.
    let names: HashSet<String> = list
        .iter()
        .take(MAX_READ)
        .filter_map(|e| e.get("name")?.as_str())
        .filter_map(|n| name(n).ok())
        .map(|(ours, _)| ours[1..].to_ascii_lowercase())
        .collect();
    let own = |n: &str| names.contains(n);
    let mut found = Found::default();
    let mut seen: HashMap<String, String> = HashMap::new();
    for (i, entry) in list.into_iter().enumerate() {
        if i >= MAX_READ {
            found.unread += 1;
            continue;
        }
        let Some(o) = entry.as_object() else {
            found.skipped.push(skip(&format!("Row {}", i + 1), "Not a command."));
            continue;
        };
        let Some(raw_name) = o.get("name").and_then(Value::as_str) else {
            found.skipped.push(skip(&format!("Row {}", i + 1), "Not a command."));
            continue;
        };
        let func = o.get("func").and_then(Value::as_str).unwrap_or("");
        let in_menu = o.get("showInMsgContextMenu").and_then(Value::as_bool).unwrap_or(false);
        match incoming(raw_name, func, in_menu, known, &own) {
            Ok(c) => {
                // Chatterino uses the first of a name; here case doesn't count either.
                if let Some(first) = seen.get(&c.key) {
                    let reason = if *first == c.name {
                        format!("{first} is above it, so Chatterino never ran this one.")
                    } else {
                        format!("Same name as {first} here, where case doesn’t count.")
                    };
                    found.skipped.push(skip(&c.name, &reason));
                    continue;
                }
                seen.insert(c.key.clone(), c.name.clone());
                found.commands.push(c);
            }
            Err(reason) => found.skipped.push(skip(raw_name.trim(), &reason)),
        }
    }
    Ok(found)
}

// ---- Into the list ----

/// Your global commands by key: the first row of each name (`/hi#chan` rows are for one chat).
fn yours(current: &Commands) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, r) in current.lines.iter().enumerate() {
        let t = r.text.trim();
        if t.is_empty() || t.starts_with('#') || r.text.starts_with(char::is_whitespace) {
            continue;
        }
        let head = t.split_once(' ').map_or(t, |(h, _)| h);
        if head.contains('#') {
            continue;
        }
        if let Some(n) = head.strip_prefix('/').filter(|n| !n.is_empty()) {
            map.entry(n.to_ascii_lowercase()).or_insert(i);
        }
    }
    map
}

fn conflict(key: &str, yours: &HashMap<String, usize>, reserved: &dyn Fn(&str) -> bool) -> (Conflict, Option<usize>) {
    match yours.get(key) {
        Some(&i) => (Conflict::Yours, Some(i)),
        None if reserved(key) => (Conflict::BuiltIn, None),
        None => (Conflict::None, None),
    }
}

fn room(rows: &[CommandRow]) -> usize {
    MAX_LINES.saturating_sub(rows.len())
}

/// What importing `found` into `current` would bring. `reserved`: the app's own command names.
pub fn plan(found: &Found, current: &Commands, reserved: &dyn Fn(&str) -> bool) -> CommandPreview {
    let mine = yours(current);
    let items = found
        .commands
        .iter()
        .enumerate()
        .map(|(id, c)| {
            let (conflict, at) = conflict(&c.key, &mine, reserved);
            let (menu, menu_note) = match &c.menu {
                Some(Ok(m)) => (Some(m.clone()), None),
                Some(Err(why)) => (None, Some(why.clone())),
                None => (None, None),
            };
            CommandItem {
                id: u32::try_from(id).unwrap_or(u32::MAX),
                name: c.name.clone(),
                row: c.row.clone(),
                notes: c.notes.clone(),
                does: c.does.clone(),
                conflict,
                yours: at.map(|i| current.lines[i].text.clone()),
                menu,
                menu_note,
                menu_kept: c.menu_kept.clone(),
            }
        })
        .collect();
    CommandPreview {
        items,
        skipped: found.skipped.clone(),
        room: u32::try_from(room(&current.lines)).unwrap_or(0),
        menu_room: u32::try_from(room(&current.message_menu)).unwrap_or(0),
        limit: u32::try_from(MAX_LINES).unwrap_or(u32::MAX),
        unread: found.unread,
    }
}

/// Adds `found` to `current`. `take`: ids of conflicts that take Chatterino's (a row of yours
/// is replaced in place; a built-in's name is added and runs instead of it). `menu`: also add the
/// message-menu entries of the commands that went in.
pub fn apply(
    found: Found,
    current: &mut Commands,
    reserved: &dyn Fn(&str) -> bool,
    take: &[u32],
    menu: bool,
) -> CommandImport {
    let take: HashSet<u32> = take.iter().copied().collect();
    let mine = yours(current);
    let free = room(&current.lines);
    let mut result = CommandImport::default();
    let mut added: Vec<CommandRow> = Vec::new();
    let mut entries: Vec<String> = Vec::new();
    for (id, c) in found.commands.into_iter().enumerate() {
        let id = u32::try_from(id).unwrap_or(u32::MAX);
        let row = CommandRow {
            enabled: true,
            text: c.row,
        };
        let (kind, at) = conflict(&c.key, &mine, reserved);
        if kind != Conflict::None && !take.contains(&id) {
            result.kept += 1;
            // V111-56 The name stays theirs; Chatterino's menu entry still comes, as its body.
            if menu && let Some(entry) = c.menu_kept {
                entries.push(entry);
            }
            continue;
        }
        if let Some(i) = at {
            current.lines[i] = row;
            result.replaced += 1;
        } else if added.len() >= free {
            result.cut.push(c.name);
            continue;
        } else {
            added.push(row);
            result.added += 1;
        }
        if menu && let Some(Ok(entry)) = c.menu {
            entries.push(entry);
        }
    }
    if !added.is_empty() {
        // The note goes in only where it doesn't cost a command its place.
        if added.len() < free {
            added.insert(
                0,
                CommandRow {
                    enabled: true,
                    text: NOTE.into(),
                },
            );
        }
        current.lines.extend(added);
    }
    let (menu_added, menu_cut) = add_menu(&mut current.message_menu, entries);
    result.menu = menu_added;
    result.menu_cut = menu_cut;
    result
}

/// A submenu entry's `.label`, or the whole row.
fn entry_key(row: &str) -> &str {
    let row = row.trim();
    match row.split_once('=') {
        Some((label, _)) if row.starts_with('.') => label,
        _ => row,
    }
}

/// Puts `entries` under the `@Commands` submenu (made at the end when there's none), leaving
/// out any the menu has already (V111-56: by label, so an import again doesn't add a twin).
/// Added and cut.
fn add_menu(rows: &mut Vec<CommandRow>, entries: Vec<String>) -> (u32, u32) {
    let have: HashSet<String> = rows.iter().map(|r| entry_key(&r.text).to_owned()).collect();
    let mut seen = HashSet::new();
    let fresh: Vec<String> = entries
        .into_iter()
        .filter(|e| !have.contains(entry_key(e)) && seen.insert(entry_key(e).to_owned()))
        .collect();
    if fresh.is_empty() {
        return (0, 0);
    }
    let head = rows.iter().position(|r| r.text.trim() == SUBMENU);
    let mut free = room(rows);
    if head.is_none() {
        // The submenu row itself.
        if free < 2 {
            return (0, u32::try_from(fresh.len()).unwrap_or(u32::MAX));
        }
        free -= 1;
    }
    let total = fresh.len();
    let fit: Vec<CommandRow> = fresh
        .into_iter()
        .take(free)
        .map(|text| CommandRow { enabled: true, text })
        .collect();
    let n = fit.len();
    match head {
        Some(at) => {
            // After the submenu's own rows (`.` rows, blanks and notes).
            let mut end = at + 1;
            while rows.get(end).is_some_and(|r| {
                let t = r.text.trim();
                t.starts_with('.') || t.is_empty() || t.starts_with('#')
            }) {
                end += 1;
            }
            rows.splice(end..end, fit);
        }
        None => {
            rows.push(CommandRow {
                enabled: true,
                text: SUBMENU.into(),
            });
            rows.extend(fit);
        }
    }
    (
        u32::try_from(n).unwrap_or(u32::MAX),
        u32::try_from(total - n).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
