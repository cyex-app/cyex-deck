//! V106-31/32 The channels an account can edit on 7TV (pure).

use cyex_emotes::seventv::account::sets::{Reachable, SetBrief, SetKind};
use cyex_emotes::seventv::events::SetChange;

use super::ops::EditOp;

/// 7TV reads at most this many sets in one go.
const MAX_SETS: usize = 50;

/// One channel's active set the account may edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// 7TV user id of the channel.
    pub owner_id: String,
    pub channel: String,
    pub own: bool,
    pub set_id: String,
    pub capacity: Option<u32>,
    /// (emote id, alias).
    pub entries: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
    /// C1 Actor of our lines: own `Reachable.name`, else "You".
    pub me: String,
    pub targets: Vec<Target>,
}

/// Own active set, then those of accepted grants with manage; at most 50, no repeats.
pub fn set_ids(me: &Reachable) -> Vec<String> {
    let grants = me
        .grants
        .iter()
        .filter(|g| g.accepted && g.manage && g.user_id != me.user_id)
        .filter_map(|g| g.set_id.clone());
    let mut out: Vec<String> = Vec::new();
    for id in me.set_id.clone().into_iter().chain(grants) {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out.truncate(MAX_SETS);
    out
}

/// Own channel first, then accepted grants with manage; only NORMAL sets that came back (Q4).
/// A grant for the user itself is skipped. Name: the connection's display name, else the 7TV id.
pub fn build(me: Reachable, sets: Vec<SetBrief>) -> Reach {
    let mut targets: Vec<Target> = Vec::new();
    let mut push = |owner: &str, name: &Option<String>, set: &Option<String>, own: bool| {
        let Some(s) = set
            .as_ref()
            .and_then(|id| sets.iter().find(|s| s.id == *id && s.kind == SetKind::Normal))
        else {
            return;
        };
        if targets.iter().any(|t| t.owner_id == owner) {
            return;
        }
        targets.push(Target {
            owner_id: owner.to_owned(),
            channel: name.clone().unwrap_or_else(|| owner.to_owned()),
            own,
            set_id: s.id.clone(),
            capacity: s.capacity,
            entries: s.entries.clone(),
        });
    };
    push(&me.user_id, &me.name, &me.set_id, true);
    for g in me
        .grants
        .iter()
        .filter(|g| g.accepted && g.manage && g.user_id != me.user_id)
    {
        push(&g.user_id, &g.name, &g.set_id, false);
    }
    Reach {
        me: me.name.unwrap_or_else(|| "You".to_owned()),
        targets,
    }
}

impl Target {
    /// By emote id.
    pub fn has(&self, emote: &str) -> bool {
        self.entries.iter().any(|(id, _)| id == emote)
    }

    /// Exact, case-sensitive (research §5).
    pub fn taken(&self, alias: &str) -> bool {
        self.entries.iter().any(|(_, a)| a == alias)
    }

    pub fn full(&self) -> bool {
        self.capacity.is_some_and(|c| self.entries.len() as u32 >= c)
    }

    /// Our own edit went through. A remove is exact: the id's other name stays.
    pub fn apply(&mut self, op: &EditOp) {
        match op {
            EditOp::Add { emote_id, alias, .. } => self.add(emote_id, alias),
            EditOp::Remove { emote_id, alias, .. } => self.remove_exact(emote_id, alias),
            EditOp::Rename {
                emote_id, from, to, ..
            } => self.rename(emote_id, from, to),
        }
    }

    /// 7TV's event. Names can lag behind, so an id held once matches whatever its name; `ours`
    /// (our own removals in it, `Echoes::filter`) only exactly.
    pub fn apply_change(&mut self, c: &SetChange, ours: &[(String, String)]) {
        for (id, name) in ours {
            self.remove_exact(id, name);
        }
        for (id, name) in &c.removed {
            self.remove(id, name);
        }
        for (id, old, new) in &c.renamed {
            self.rename(id, old, &new.name);
        }
        for e in &c.added {
            self.add(&e.id, &e.name);
        }
    }

    fn add(&mut self, id: &str, alias: &str) {
        if !self.entries.iter().any(|(i, a)| i == id && a == alias) {
            self.entries.push((id.to_owned(), alias.to_owned()));
        }
    }

    /// The entry (id, name), or the only entry with that id.
    fn slot(&self, id: &str, name: &str) -> Option<usize> {
        let exact = self.entries.iter().position(|(i, a)| i == id && a == name);
        exact.or_else(|| {
            let mut same = self.entries.iter().enumerate().filter(|(_, (i, _))| i == id);
            match (same.next(), same.next()) {
                (Some((n, _)), None) => Some(n),
                _ => None,
            }
        })
    }

    fn remove(&mut self, id: &str, alias: &str) {
        if let Some(n) = self.slot(id, alias) {
            self.entries.remove(n);
        }
    }

    fn remove_exact(&mut self, id: &str, alias: &str) {
        if let Some(n) = self.entries.iter().position(|(i, a)| i == id && a == alias) {
            self.entries.remove(n);
        }
    }

    fn rename(&mut self, id: &str, from: &str, to: &str) {
        if let Some(n) = self.slot(id, from) {
            self.entries[n].1 = to.to_owned();
        }
    }
}
