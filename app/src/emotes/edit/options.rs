//! V106-31/32 What the menus, the card and the picker offer for one emote (pure; Q1-Q3, Q6).

use super::ops::{AddBlocked, AddOptions, AddTarget, Blocked, EditOptions, HereEdit, Reachability};
use super::reach::Reach;

/// The emote.
pub struct Facts<'a> {
    pub seventv: bool,
    pub emote_id: &'a str,
    pub own_name: &'a str,
    pub private: bool,
    /// 7TV user id of who made it.
    pub owner_id: Option<&'a str>,
}

/// This chat's channel.
pub struct Viewed<'a> {
    pub owner_id: Option<&'a str>,
    pub set_id: Option<&'a str>,
    /// rc-2 The emote's name in this chat's channel set (`set_alias`), if it's there.
    pub alias: Option<&'a str>,
}

/// rc-2 The emote's name in a set: the line's `name` when (id, name) is there, else the id's
/// current name (a line from before a rename still finds it).
pub fn set_alias<'a>(entries: impl IntoIterator<Item = (&'a str, &'a str)>, id: &str, name: &str) -> Option<&'a str> {
    let mut by_id = None;
    for (e, alias) in entries {
        if e != id {
            continue;
        }
        if alias == name {
            return Some(alias);
        }
        by_id.get_or_insert(alias);
    }
    by_id
}

pub fn options(state: Reachability, reach: Option<&Reach>, viewed: &Viewed, f: &Facts) -> EditOptions {
    let mut out = EditOptions {
        state,
        own_name: f.own_name.to_owned(),
        add: None,
        here: None,
    };
    let Some(reach) = reach.filter(|_| state == Reachability::Ready && f.seventv) else {
        return out;
    };
    let here = reach
        .targets
        .iter()
        .find(|t| viewed.owner_id == Some(t.owner_id.as_str()));
    out.here = here
        .filter(|t| viewed.set_id == Some(t.set_id.as_str()))
        .zip(viewed.alias)
        .map(|(t, alias)| HereEdit {
            set_id: t.set_id.clone(),
            channel: t.channel.clone(),
            alias: alias.to_owned(),
        });
    let mut targets: Vec<AddTarget> = reach
        .targets
        .iter()
        .map(|t| {
            let blocked = t.blocked(f.emote_id, f.private, f.owner_id);
            AddTarget {
                set_id: t.set_id.clone(),
                channel: t.channel.clone(),
                own: t.own,
                viewed: viewed.owner_id == Some(t.owner_id.as_str()),
                blocked,
            }
        })
        .collect();
    if targets.is_empty() {
        return out;
    }
    targets.sort_by_cached_key(|t| (!t.viewed, !t.own, t.channel.to_lowercase()));
    let all = |b: Blocked| targets.iter().all(|t| t.blocked == Some(b));
    let blocked = if targets.iter().any(|t| t.blocked.is_none()) {
        None
    } else if all(Blocked::Already) {
        Some(AddBlocked::Already)
    } else if all(Blocked::Private) {
        Some(AddBlocked::Private)
    } else {
        Some(AddBlocked::Nowhere)
    };
    out.add = Some(AddOptions { blocked, targets });
    out
}
