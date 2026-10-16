//! V1051-04 Every change to the saved combos. The menu and `/combo` send a [`ComboOp`];
//! the result is saved and sent to every window.

use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Combo, ComboItem, Combos, MAX_COMBOS, MAX_NAME, NameError, clean_name, fold, random_id};

#[derive(Clone, Debug, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ComboOp {
    Save {
        name: String,
        paint: Option<ComboItem>,
        badge: Option<ComboItem>,
    },
    Rename {
        id: String,
        name: String,
    },
    Edit {
        id: String,
        paint: Option<ComboItem>,
        badge: Option<ComboItem>,
    },
    Duplicate {
        id: String,
    },
    Delete {
        id: String,
    },
    /// Undo of a delete: back at `at` (clamped).
    Restore {
        combo: Combo,
        at: usize,
    },
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ComboDone {
    Saved { combo: Combo },
    Changed { combo: Combo },
    Deleted { combo: Combo, at: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComboError {
    Name(NameError),
    Taken(String),
    Full,
    Missing,
    /// The settings file couldn't be written (the app's side, not the op's).
    Unsaved,
}

impl fmt::Display for ComboError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComboError::Name(e) => e.fmt(f),
            ComboError::Taken(n) => write!(f, "You have a combo called “{n}” already."),
            ComboError::Full => write!(f, "You have {MAX_COMBOS} combos. Delete one first."),
            ComboError::Missing => f.write_str("That combo is gone."),
            ComboError::Unsaved => f.write_str("Couldn’t save your combos. Try again."),
        }
    }
}

impl std::error::Error for ComboError {}

impl From<NameError> for ComboError {
    fn from(e: NameError) -> Self {
        ComboError::Name(e)
    }
}

impl Combos {
    pub fn change(&mut self, account: &str, op: ComboOp) -> Result<ComboDone, ComboError> {
        match op {
            ComboOp::Save { name, paint, badge } => {
                let name = self.free_name(account, &name, None)?;
                let combo = Combo {
                    id: self.free_id(account, random_id()),
                    name,
                    paint,
                    badge,
                };
                self.list_mut(account).push(combo.clone());
                Ok(ComboDone::Saved { combo })
            }
            ComboOp::Rename { id, name } => {
                self.index(account, &id)?;
                let name = self.free_name(account, &name, Some(&id))?;
                let combo = self.get_mut(account, &id)?;
                combo.name = name;
                Ok(ComboDone::Changed { combo: combo.clone() })
            }
            ComboOp::Edit { id, paint, badge } => {
                let combo = self.get_mut(account, &id)?;
                combo.paint = paint;
                combo.badge = badge;
                Ok(ComboDone::Changed { combo: combo.clone() })
            }
            ComboOp::Duplicate { id } => {
                let at = self.index(account, &id)?;
                if self.list(account).len() >= MAX_COMBOS {
                    return Err(ComboError::Full);
                }
                let mut combo = self.list(account)[at].clone();
                combo.name = self.copy_name(account, &combo.name);
                combo.id = self.free_id(account, random_id());
                self.list_mut(account).insert(at + 1, combo.clone());
                Ok(ComboDone::Saved { combo })
            }
            ComboOp::Delete { id } => {
                let at = self.index(account, &id)?;
                let list = self.list_mut(account);
                let combo = list.remove(at);
                if list.is_empty() {
                    self.accounts.remove(account);
                }
                Ok(ComboDone::Deleted { combo, at })
            }
            ComboOp::Restore { mut combo, at } => {
                combo.name = self.free_name(account, &combo.name, None)?;
                if combo.id.is_empty() || self.index(account, &combo.id).is_ok() {
                    combo.id = self.free_id(account, random_id());
                }
                let list = self.list_mut(account);
                let at = at.min(list.len());
                list.insert(at, combo.clone());
                Ok(ComboDone::Saved { combo })
            }
        }
    }

    fn list_mut(&mut self, account: &str) -> &mut Vec<Combo> {
        self.accounts.entry(account.to_owned()).or_default()
    }

    fn index(&self, account: &str, id: &str) -> Result<usize, ComboError> {
        self.list(account)
            .iter()
            .position(|c| c.id == id)
            .ok_or(ComboError::Missing)
    }

    fn get_mut(&mut self, account: &str, id: &str) -> Result<&mut Combo, ComboError> {
        self.accounts
            .get_mut(account)
            .and_then(|l| l.iter_mut().find(|c| c.id == id))
            .ok_or(ComboError::Missing)
    }

    /// The cleaned name, if it's valid and nobody else (not `except`) has it. A new combo
    /// (`except` none) also needs room.
    fn free_name(&self, account: &str, raw: &str, except: Option<&str>) -> Result<String, ComboError> {
        let name = clean_name(raw)?;
        let key = fold(&name);
        if self
            .list(account)
            .iter()
            .any(|c| fold(&c.name) == key && Some(c.id.as_str()) != except)
        {
            return Err(ComboError::Taken(name));
        }
        if except.is_none() && self.list(account).len() >= MAX_COMBOS {
            return Err(ComboError::Full);
        }
        Ok(name)
    }

    fn free_id(&self, account: &str, mut id: String) -> String {
        while self.index(account, &id).is_ok() {
            id = random_id();
        }
        id
    }

    fn copy_name(&self, account: &str, base: &str) -> String {
        for n in 1usize.. {
            let suffix = if n == 1 {
                " copy".to_owned()
            } else {
                format!(" copy {n}")
            };
            let room = MAX_NAME - suffix.chars().count();
            let head: String = base.chars().take(room).collect();
            let name = format!("{}{suffix}", head.trim_end());
            if self.find(account, &name).is_none() {
                return name;
            }
        }
        unreachable!()
    }
}
