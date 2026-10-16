//! V1051-04 Saved 7TV paint + badge pairs, per Twitch account. Names and ids only: no
//! tokens, so it's portable and backed up like any partition.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Schema, random_id};

mod ops;
pub use ops::{ComboDone, ComboError, ComboOp};

pub const MAX_COMBOS: usize = 50;
pub const MAX_NAME: usize = 32;
const MAX_ACCOUNTS: usize = 50;
const MAX_ITEM_NAME: usize = 100;
/// Words `/combo` reads as its own; not allowed as names (Q2).
pub const WORDS: [&str; 4] = ["list", "save", "delete", "rename"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Combos {
    /// By Twitch account id.
    pub accounts: BTreeMap<String, Vec<Combo>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Combo {
    pub id: String,
    pub name: String,
    pub paint: Option<ComboItem>,
    pub badge: Option<ComboItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ComboItem {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    Chars,
    Word(String),
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::Empty => f.write_str("Give it a name."),
            NameError::TooLong => write!(f, "Names can be up to {MAX_NAME} characters."),
            NameError::Chars => f.write_str("Names can have letters, numbers, spaces, ’, - and _."),
            NameError::Word(w) => write!(f, "“{w}” is part of /combo. Pick another name."),
        }
    }
}

impl std::error::Error for NameError {}

/// Trims, collapses runs of whitespace to one space, then checks: 1-32 chars; only letters,
/// numbers (Unicode `is_alphanumeric`), spaces, the curly ’, `-` and `_`; not one of `WORDS` (any case).
pub fn clean_name(raw: &str) -> Result<String, NameError> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.chars().count() > MAX_NAME {
        return Err(NameError::TooLong);
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '’'))
    {
        return Err(NameError::Chars);
    }
    if WORDS.iter().any(|w| name.eq_ignore_ascii_case(w)) {
        return Err(NameError::Word(name));
    }
    Ok(name)
}

fn fold(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Combos {
    pub fn list(&self, account: &str) -> &[Combo] {
        self.accounts.get(account).map_or(&[], Vec::as_slice)
    }

    /// Case-insensitive, spaces collapsed, like `clean_name`.
    pub fn find(&self, account: &str, name: &str) -> Option<&Combo> {
        let want = fold(name);
        self.list(account).iter().find(|c| fold(&c.name) == want)
    }
}

fn clean_item(item: Option<ComboItem>) -> Option<ComboItem> {
    let mut item = item?;
    let ok = (1..=40).contains(&item.id.len()) && item.id.bytes().all(|b| b.is_ascii_alphanumeric());
    if !ok {
        return None;
    }
    if item.name.chars().count() > MAX_ITEM_NAME {
        item.name = item.name.chars().take(MAX_ITEM_NAME).collect();
    }
    Some(item)
}

impl Schema for Combos {
    fn sanitize(&mut self) {
        self.accounts
            .retain(|k, _| (1..=20).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_digit()));
        while self.accounts.len() > MAX_ACCOUNTS {
            self.accounts.pop_last();
        }
        for list in self.accounts.values_mut() {
            let mut names = HashSet::new();
            let mut ids = HashSet::new();
            let mut kept = Vec::new();
            for mut c in std::mem::take(list) {
                let Ok(name) = clean_name(&c.name) else { continue };
                if !names.insert(fold(&name)) {
                    continue;
                }
                c.name = name;
                while c.id.is_empty() || !ids.insert(c.id.clone()) {
                    c.id = random_id();
                }
                c.paint = clean_item(c.paint.take());
                c.badge = clean_item(c.badge.take());
                kept.push(c);
                if kept.len() == MAX_COMBOS {
                    break;
                }
            }
            *list = kept;
        }
        self.accounts.retain(|_, l| !l.is_empty());
    }
}

#[cfg(test)]
mod tests;
