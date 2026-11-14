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
    /// Chatterino showed it in the message menu: our entry (`.label=body`) or why there's none.
    pub menu: Option<Result<String, String>>,
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
    pub conflict: Conflict,
    /// Your row with that name.
    pub yours: Option<String>,
    /// The message-menu entry, if Chatterino showed it there and it maps.
    pub menu: Option<String>,
    /// Why Chatterino's menu entry can't come along.
    pub menu_note: Option<String>,
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
    Word { from: u32, rest: bool },
    Var { name: &'static str, alt: Option<String> },
}

/// The variables `COMMAND_VARS` knows (case matters there too). Anything else in braces is text.
const VARS: [&str; 15] = [
    "channel.name",
    "channel.id",
    "channel",
    "stream.game",
    "stream.title",
    "my.id",
    "my.name",
    "user.name",
    "user",
    "msg.id",
    "msg-id",
    "msg.text",
    "message",
    "input.text",
    "element.copytext",
];

enum Body {
    Digits { digits: String, plus: bool },
    Name { name: String, alt: Option<String> },
}

struct Match {
    /// Where the match starts: the character before the braces, or 0 for `^`.
    start: usize,
    end: usize,
    body: Body,
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '.' || c == '-'
}

/// `({{)*{(\d+\+?|([a-zA-Z.-]+)(?:;(.+?))?)}` at `q`: the end and the body.
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
        let close = e + usize::from(plus);
        return (s.get(close) == Some(&'}')).then_some((close + 1, Body::Digits { digits, plus }));
    }
    if !is_name_char(first) {
        return None;
    }
    let mut e = b;
    while s.get(e).is_some_and(|&c| is_name_char(c)) {
        e += 1;
    }
    let name: String = s[b..e].iter().collect();
    match s.get(e) {
        Some('}') => Some((e + 1, Body::Name { name, alt: None })),
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
            Some((j + 1, Body::Name { name, alt: Some(alt) }))
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
            Body::Digits { digits, plus } => match digits.parse::<i32>() {
                Ok(n) if n > 0 => Some(Piece::Word {
                    from: n.unsigned_abs(),
                    rest: plus,
                }),
                // `{0}` and numbers too big for an int stay as they are.
                _ => {
                    text.push('{');
                    text.push_str(&digits);
                    if plus {
                        text.push('+');
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

/// Where the command runs: typed in a chat, or from the message menu.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Typed,
    Menu,
}

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

/// A Chatterino variable in our syntax, `Err` when there's no match.
fn var(name: &str, alt: Option<&str>, place: Place) -> Result<String, String> {
    let alt_text = || alt.map(|a| escape(a, false)).unwrap_or_default();
    Ok(match (name, place) {
        ("channel.name" | "channel", _) => value("chan", alt, true),
        ("channel.id", _) => return Err("Uses {channel.id}, which has no match here.".into()),
        ("stream.game", _) => value("streamgame", alt, false),
        ("stream.title", _) => value("streamtitle", alt, false),
        ("my.id", _) => value("my-user-id", alt, false),
        ("my.name", _) => value("my-nick", alt, false),
        // Typed, Chatterino has no message: these give their alt text.
        (_, Place::Typed) => alt_text(),
        ("user.name" | "user", Place::Menu) => value("nick", alt, false),
        ("msg.id" | "msg-id", Place::Menu) => value("msg-id", alt, false),
        ("msg.text" | "message", Place::Menu) => value("msg", alt, false),
        ("input.text", Place::Menu) => return Err("Uses the text in the input box.".into()),
        (_, Place::Menu) => return Err("Uses the word you clicked.".into()),
    })
}

/// The pieces as a body of ours.
fn body(pieces: &[Piece], place: Place) -> Result<String, String> {
    let mut out = String::new();
    for (i, p) in pieces.iter().enumerate() {
        match p {
            Piece::Text(t) => out.push_str(&escape(t, false)),
            Piece::Word { from, rest } => {
                if place == Place::Menu {
                    // Chatterino passes the message as the words: all of it is our $(msg).
                    if *from == 1 && *rest {
                        out.push_str("$(msg)");
                        continue;
                    }
                    return Err("Uses single words of the message.".into());
                }
                // `$1` right before a "-" would read as `$1-`.
                let dash = matches!(pieces.get(i + 1), Some(Piece::Text(t)) if t.starts_with('-'));
                let tail = if *rest { "-" } else { "" };
                if *from > 9 || (dash && !rest) {
                    out.push_str(&format!("$({from}{tail})"));
                } else {
                    out.push_str(&format!("${from}{tail}"));
                }
            }
            Piece::Var { name, alt } => out.push_str(&var(name, alt.as_deref(), place)?),
        }
    }
    Ok(out)
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

fn incoming(raw_name: &str, raw_func: &str, in_menu: bool, known: &dyn Fn(&str) -> bool) -> Result<Incoming, String> {
    let (ours, name_note) = name(raw_name)?;
    let mut notes: Vec<String> = name_note.into_iter().collect();
    let mut func = raw_func.trim().to_owned();
    if func.contains(['\n', '\r']) {
        func = func.split(['\r', '\n']).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join(" ");
        notes.push("Line breaks became spaces.".into());
    }
    if func.is_empty() {
        return Err("Sends nothing.".into());
    }
    let ps = pieces(&func);
    let typed = body(&ps, Place::Typed)?;
    let typed = typed.trim();
    if typed.is_empty() {
        return Err("Sends nothing.".into());
    }
    let row = format!("{ours} {typed}");
    if row.chars().count() > ROW_MAX {
        return Err("Over 2.000 characters.".into());
    }
    // What it runs first, when that's a command this app doesn't have.
    if let Some(Piece::Text(t)) = ps.first() {
        let t = t.trim_start();
        // The whole first word is text, not half a placeholder.
        let whole = ps.len() == 1 || t.contains(char::is_whitespace);
        let first = t.split_whitespace().next().unwrap_or("");
        if whole
            && let Some(cmd) = first.strip_prefix('/').filter(|c| !c.is_empty() && !c.starts_with('/'))
            && !known(&cmd.to_ascii_lowercase())
        {
            notes.push(format!("Runs /{cmd}, which isn’t a command here."));
        }
    }
    let menu = in_menu.then(|| {
        let l = label(&ours).ok_or_else(|| "Its name can’t be a menu label.".to_owned())?;
        let b = body(&ps, Place::Menu)?;
        let entry = format!(".{l}={b}");
        if entry.chars().count() > ROW_MAX {
            return Err("Over 2.000 characters.".into());
        }
        Ok(entry)
    });
    let key = ours[1..].to_ascii_lowercase();
    Ok(Incoming {
        name: raw_name.trim().to_owned(),
        key,
        row,
        notes,
        menu,
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
        match incoming(raw_name, func, in_menu, known) {
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
                conflict,
                yours: at.map(|i| current.lines[i].text.clone()),
                menu,
                menu_note,
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

/// Puts `entries` under the `@Commands` submenu (made at the end when there's none), leaving
/// out any the menu has already. Added and cut.
fn add_menu(rows: &mut Vec<CommandRow>, entries: Vec<String>) -> (u32, u32) {
    let have: HashSet<String> = rows.iter().map(|r| r.text.trim().to_owned()).collect();
    let mut seen = HashSet::new();
    let fresh: Vec<String> = entries
        .into_iter()
        .filter(|e| !have.contains(e.as_str()) && seen.insert(e.clone()))
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
