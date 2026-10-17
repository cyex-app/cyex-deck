//! C1 Our pending and recent edits, so 7TV's event for one of them prints no second line
//! (pure; `now` is passed in). Data is never filtered, only lines.

use std::time::{Duration, Instant};

use cyex_emotes::seventv::events::SetChange;

use super::ops::EditOp;

/// How long after our reply 7TV's event still counts as ours.
const ECHO: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Add,
    Remove,
    Rename,
}

#[derive(Debug)]
struct Echo {
    set_id: String,
    kind: Kind,
    emote_id: String,
    /// Add and remove: the alias; rename: the new name.
    name: String,
    /// 7TV's event for it came while the request was out.
    seen: bool,
    /// `None` = request still out.
    until: Option<Instant>,
}

fn kind(op: &EditOp) -> Kind {
    match op {
        EditOp::Add { .. } => Kind::Add,
        EditOp::Remove { .. } => Kind::Remove,
        EditOp::Rename { .. } => Kind::Rename,
    }
}

#[derive(Debug, Default)]
pub struct Echoes(Vec<Echo>);

impl Echoes {
    fn find(&self, op: &EditOp) -> Option<usize> {
        let k = kind(op);
        // A settled entry of an earlier, equal edit is not this one.
        self.0.iter().position(|e| {
            e.until.is_none()
                && e.kind == k
                && e.set_id == op.set_id()
                && e.emote_id == op.emote_id()
                && e.name == op.name()
        })
    }

    #[cfg(test)]
    pub fn all_settled(&self) -> bool {
        self.0.iter().all(|e| e.until.is_some())
    }

    /// The request goes out.
    pub fn expect(&mut self, op: &EditOp) {
        self.0.push(Echo {
            set_id: op.set_id().to_owned(),
            kind: kind(op),
            emote_id: op.emote_id().to_owned(),
            name: op.name().to_owned(),
            seen: false,
            until: None,
        });
    }

    /// It worked: an event already seen is done with; else wait for it a while.
    pub fn settle(&mut self, op: &EditOp, now: Instant) {
        let Some(n) = self.find(op) else {
            return;
        };
        if self.0[n].seen {
            self.0.remove(n);
        } else {
            self.0[n].until = Some(now + ECHO);
        }
    }

    /// It failed: drops the entry. True when 7TV's event showed it happened anyway.
    pub fn forget(&mut self, op: &EditOp) -> bool {
        self.find(op).is_some_and(|n| self.0.remove(n).seen)
    }

    /// Takes the first live entry that matches, giving its name; pending ones are marked seen,
    /// settled ones go.
    fn take(&mut self, set_id: &str, k: Kind, emote_id: &str, name: Option<&str>) -> Option<String> {
        let hit = self.0.iter().position(|e| {
            !e.seen
                && e.kind == k
                && e.set_id == set_id
                && e.emote_id == emote_id
                && name.is_none_or(|n| e.name == n)
        });
        let n = hit?;
        if self.0[n].until.is_some() {
            Some(self.0.remove(n).name)
        } else {
            self.0[n].seen = true;
            Some(self.0[n].name.clone())
        }
    }

    /// `c` minus the parts that are our own edits, and our removals in it as (id, the alias we
    /// removed): those apply exactly, never "the id held once" (that would take a second name).
    /// Remove matches by emote id alone (7TV's names can lag).
    pub fn filter(&mut self, c: &SetChange, now: Instant) -> (SetChange, Vec<(String, String)>) {
        self.0.retain(|e| e.until.is_none_or(|u| now <= u));
        let set = c.set_id.as_str();
        let mut ours = Vec::new();
        let mut removed = Vec::new();
        for (id, name) in &c.removed {
            match self.take(set, Kind::Remove, id, None) {
                Some(alias) => ours.push((id.clone(), alias)),
                None => removed.push((id.clone(), name.clone())),
            }
        }
        let shown = SetChange {
            set_id: c.set_id.clone(),
            actor: c.actor.clone(),
            added: c
                .added
                .iter()
                .filter(|e| self.take(set, Kind::Add, &e.id, Some(&e.name)).is_none())
                .cloned()
                .collect(),
            removed,
            renamed: c
                .renamed
                .iter()
                .filter(|(id, _, new)| self.take(set, Kind::Rename, id, Some(&new.name)).is_none())
                .cloned()
                .collect(),
        };
        (shown, ours)
    }
}
