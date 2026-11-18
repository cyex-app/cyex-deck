//! V111-60 The rest of Chatterino's settings.json that cyex has a home for: ignored
//! phrases, people who never highlight, muted chats, filters and nicknames. Only those keys
//! are read; the file also holds account tokens, which are never read, kept or logged.
//! Rows you have already win: the import only adds.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Skipped;
use crate::schema::{FilterDef, Highlights, IgnoreRule, MAX_FILTERS, MAX_NEVER, MAX_RULES, UserNote};

/// Longest nickname the chat settings keep.
const MAX_NICKNAME: usize = 25;
const MAX_FILTER_EXPR: usize = 2000;
const MAX_FILTER_NAME: usize = 40;

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    ignore: Option<IgnoreSection>,
    #[serde(default)]
    highlighting: Option<Highlighting>,
    #[serde(default)]
    pings: Option<Pings>,
    #[serde(default)]
    filtering: Option<Filtering>,
    #[serde(default)]
    nicknames: Vec<Nick>,
}

#[derive(Deserialize)]
struct IgnoreSection {
    #[serde(default)]
    phrases: Vec<Phrase>,
}

#[derive(Deserialize)]
struct Phrase {
    #[serde(default)]
    pattern: String,
    #[serde(default)]
    regex: bool,
    #[serde(default, rename = "isBlock")]
    is_block: bool,
    #[serde(default, rename = "replaceWith")]
    replace_with: String,
    #[serde(default, rename = "caseSensitive")]
    case_sensitive: bool,
}

#[derive(Deserialize)]
struct Highlighting {
    #[serde(default)]
    blacklist: Vec<Blacklisted>,
}

#[derive(Deserialize)]
struct Blacklisted {
    #[serde(default)]
    pattern: String,
    #[serde(default)]
    regex: bool,
}

#[derive(Deserialize)]
struct Pings {
    #[serde(default)]
    muted: Vec<String>,
}

#[derive(Deserialize)]
struct Filtering {
    #[serde(default)]
    filters: Vec<Filter>,
}

#[derive(Deserialize)]
struct Filter {
    #[serde(default)]
    name: String,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    id: String,
}

#[derive(Deserialize)]
struct Nick {
    #[serde(default)]
    name: String,
    #[serde(default)]
    replace: String,
    #[serde(default, rename = "isRegex")]
    is_regex: bool,
}

/// The file isn't Chatterino's settings.json.
#[derive(Debug, PartialEq, Eq)]
pub struct NotSettings;

/// A nickname for a login; `id` once the login was looked up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nickname {
    pub login: String,
    pub nickname: String,
    pub id: Option<String>,
}

/// What the file holds, as our rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Extras {
    pub ignores: Vec<IgnoreRule>,
    pub never: Vec<String>,
    pub muted: Vec<String>,
    pub filters: Vec<FilterDef>,
    pub nicknames: Vec<Nickname>,
    pub skipped: Vec<Skipped>,
}

/// New rows per kind (preview: would be added; result: were added).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExtrasPreview {
    pub ignores: u32,
    pub never: u32,
    pub muted: u32,
    pub filters: u32,
    pub nicknames: u32,
    /// Rows you have already; yours stay.
    pub already: u32,
    pub skipped: Vec<Skipped>,
}

impl ExtrasPreview {
    pub fn added(&self) -> u32 {
        self.ignores + self.never + self.muted + self.filters + self.nicknames
    }
}

fn skip(pattern: &str, reason: &str) -> Skipped {
    Skipped {
        pattern: pattern.to_owned(),
        reason: reason.to_owned(),
    }
}

fn login(name: &str) -> Option<String> {
    let l = name.trim().trim_start_matches(['@', '#']).to_ascii_lowercase();
    cyex_core::login::is_login(&l).then_some(l)
}

/// `regex` turns a PCRE2 pattern into the one we store, `filter` says why a filter
/// doesn't read (the app passes the chat crate's).
pub fn parse(
    text: &str,
    regex: impl Fn(&str) -> Result<String, String>,
    filter: impl Fn(&str) -> Result<(), String>,
) -> Result<Extras, NotSettings> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| NotSettings)?;
    if !value.is_object() {
        return Err(NotSettings);
    }
    let file: File = serde_json::from_value(value).map_err(|_| NotSettings)?;
    let mut out = Extras::default();
    for p in file.ignore.map(|s| s.phrases).unwrap_or_default() {
        let pattern = p.pattern.trim();
        if pattern.is_empty() {
            continue;
        }
        let stored = if p.regex {
            match regex(pattern) {
                Ok(r) => r,
                Err(why) => {
                    out.skipped.push(skip(pattern, &format!("Ignore: {why}")));
                    continue;
                }
            }
        } else {
            pattern.to_owned()
        };
        out.ignores.push(IgnoreRule {
            pattern: stored,
            regex: p.regex,
            case_sensitive: p.case_sensitive,
            block: p.is_block,
            replace: p.replace_with,
            ..IgnoreRule::default()
        });
    }
    for b in file.highlighting.map(|h| h.blacklist).unwrap_or_default() {
        let pattern = b.pattern.trim();
        if pattern.is_empty() {
            continue;
        }
        match login(pattern).filter(|_| !b.regex) {
            Some(l) => out.never.push(l),
            None => out
                .skipped
                .push(skip(pattern, "Never highlight takes one name per row, no regex.")),
        }
    }
    for m in file.pings.map(|p| p.muted).unwrap_or_default() {
        match login(&m) {
            Some(l) => out.muted.push(l),
            None if m.trim().is_empty() => {}
            None => out.skipped.push(skip(&m, "Muted chat: not a channel name.")),
        }
    }
    for f in file.filtering.map(|f| f.filters).unwrap_or_default() {
        let expr = f.filter.trim();
        let name = f.name.trim();
        if expr.is_empty() {
            continue;
        }
        if expr.chars().count() > MAX_FILTER_EXPR {
            out.skipped.push(skip(name, "Filter: longer than 2.000 characters."));
            continue;
        }
        if let Err(why) = filter(expr) {
            out.skipped.push(skip(name, &format!("Filter: {why}")));
            continue;
        }
        let name = if name.is_empty() { "From Chatterino" } else { name };
        out.filters.push(FilterDef {
            id: f.id.chars().filter(char::is_ascii_hexdigit).take(8).collect::<String>().to_ascii_lowercase(),
            name: name.chars().take(MAX_FILTER_NAME).collect(),
            expr: expr.to_owned(),
        });
    }
    for n in file.nicknames {
        let nickname: String = n.replace.trim().chars().take(MAX_NICKNAME).collect();
        let name = n.name.trim();
        if name.is_empty() || nickname.is_empty() {
            continue;
        }
        if n.is_regex {
            out.skipped
                .push(skip(name, "Nickname: regex ones don’t map, a nickname here is for one person."));
            continue;
        }
        match login(name) {
            Some(login) if !out.nicknames.iter().any(|x| x.login == login) => {
                out.nicknames.push(Nickname { login, nickname, id: None })
            }
            Some(_) => {}
            None => out.skipped.push(skip(name, "Nickname: not a Twitch name.")),
        }
    }
    Ok(out)
}

impl Extras {
    /// The logins whose nicknames need a user id, once each.
    pub fn nickname_logins(&self) -> Vec<String> {
        self.nicknames.iter().map(|n| n.login.clone()).collect()
    }

    /// Fills in the ids the lookup found (login → id); the rest are skipped as not found.
    pub fn resolve_nicknames(&mut self, ids: &HashMap<String, String>) {
        let mut kept = Vec::with_capacity(self.nicknames.len());
        for mut n in self.nicknames.drain(..) {
            match ids.get(&n.login) {
                Some(id) => {
                    n.id = Some(id.clone());
                    kept.push(n);
                }
                None => self.skipped.push(skip(&n.login, "Nickname: not found on Twitch.")),
            }
        }
        self.nicknames = kept;
    }

    /// The lookup couldn't run: every nickname is skipped with `why`.
    pub fn drop_nicknames(&mut self, why: &str) {
        for n in self.nicknames.drain(..) {
            self.skipped.push(skip(&n.login, &format!("Nickname: {why}")));
        }
    }
}

/// Appends `new` rows not in `list` (same `key`), up to `max`.
fn merge<T: Clone, K: PartialEq>(
    list: &mut Vec<T>,
    new: &[T],
    key: impl Fn(&T) -> K,
    label: impl Fn(&T) -> String,
    max: usize,
    already: &mut u32,
    skipped: &mut Vec<Skipped>,
) -> u32 {
    let mut added = 0;
    for row in new {
        let k = key(row);
        if list.iter().any(|r| key(r) == k) {
            *already += 1;
        } else if list.len() >= max {
            skipped.push(skip(&label(row), &format!("Over the {max} limit.")));
        } else {
            list.push(row.clone());
            added += 1;
        }
    }
    added
}

fn ignore_key(r: &IgnoreRule) -> (String, bool) {
    let p = r.pattern.trim();
    let p = if !r.regex && !r.case_sensitive { p.to_lowercase() } else { p.to_owned() };
    (p, r.regex)
}

/// Adds the highlight-partition rows (ignores, never, muted, filters) to `h`.
pub fn apply_lists(found: &Extras, h: &mut Highlights, out: &mut ExtrasPreview) {
    let (mut already, mut skipped) = (0, Vec::new());
    out.ignores = merge(
        &mut h.ignores,
        &found.ignores,
        ignore_key,
        |r| r.pattern.clone(),
        MAX_RULES,
        &mut already,
        &mut skipped,
    );
    out.never = merge(&mut h.never, &found.never, |s| s.to_ascii_lowercase(), String::clone, MAX_NEVER, &mut already, &mut skipped);
    out.muted = merge(&mut h.muted, &found.muted, |s| s.to_ascii_lowercase(), String::clone, MAX_NEVER, &mut already, &mut skipped);
    // A filter with the same name, or the same expression, is already there.
    let mut filters_added = 0;
    for f in &found.filters {
        let same = h
            .filters
            .iter()
            .any(|x| x.name.trim().eq_ignore_ascii_case(f.name.trim()) || x.expr.trim() == f.expr.trim());
        if same {
            already += 1;
        } else if h.filters.len() >= MAX_FILTERS {
            skipped.push(skip(&f.name, &format!("Over the {MAX_FILTERS} limit.")));
        } else {
            let mut f = f.clone();
            f.id = free_id(&f.id, &h.filters);
            h.filters.push(f);
            filters_added += 1;
        }
    }
    out.filters = filters_added;
    out.already += already;
    out.skipped.extend(skipped);
}

/// Chatterino's id when it's free, else the first free `chN`.
fn free_id(wanted: &str, list: &[FilterDef]) -> String {
    let taken = |id: &str| list.iter().any(|f| f.id == id);
    if !wanted.is_empty() && !taken(wanted) {
        return wanted.to_owned();
    }
    (1..).map(|n| format!("ch{n}")).find(|id| !taken(id)).expect("a free id")
}

/// Adds nicknames (looked up ones only) to the chat's people notes; a nickname you set stays.
pub fn apply_nicknames(found: &Extras, users: &mut BTreeMap<String, UserNote>, out: &mut ExtrasPreview) {
    for n in &found.nicknames {
        let Some(id) = n.id.as_deref() else { continue };
        let note = users.entry(id.to_owned()).or_default();
        if note.nickname.trim().is_empty() {
            note.nickname = n.nickname.clone();
            out.nicknames += 1;
        } else {
            out.already += 1;
        }
    }
}

/// The result to fill: starts with what the file had that can't come.
pub fn start(found: &Extras) -> ExtrasPreview {
    ExtrasPreview {
        skipped: found.skipped.clone(),
        ..ExtrasPreview::default()
    }
}

/// What importing `found` would do, on copies.
pub fn plan(found: &Extras, h: &Highlights, users: &BTreeMap<String, UserNote>) -> ExtrasPreview {
    let mut out = start(found);
    apply_lists(found, &mut h.clone(), &mut out);
    apply_nicknames(found, &mut users.clone(), &mut out);
    out
}

#[cfg(test)]
#[path = "extras_tests.rs"]
mod tests;
