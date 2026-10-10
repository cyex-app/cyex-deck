//! Emote menu favorites and most used (C12). A partition of its own, since usage
//! changes with every message sent.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

const MAX_FAVORITES: usize = 300;
/// Most used keeps this many; the least used make room for new ones.
const MAX_USAGE: usize = 100;
const MAX_NAME_CHARS: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Emotes {
    pub favorites: Vec<EmoteKey>,
    /// Most used first.
    pub usage: Vec<EmoteUse>,
}

/// Where an emote comes from. Emoji (E1) use the emoji itself as id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EmoteSource {
    Twitch,
    Seventv,
    Bttv,
    Ffz,
    Emoji,
}

impl EmoteSource {
    /// C11 The providers the priority order ranks.
    pub const THIRD_PARTY: [EmoteSource; 3] = [EmoteSource::Seventv, EmoteSource::Bttv, EmoteSource::Ffz];
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteKey {
    pub source: EmoteSource,
    pub id: String,
    /// The name it had when saved, to show and insert.
    pub name: String,
}

impl EmoteKey {
    fn valid(&self) -> bool {
        !self.id.is_empty() && !self.name.is_empty() && self.name.chars().count() <= MAX_NAME_CHARS
    }

    fn same(&self, other: &EmoteKey) -> bool {
        self.source == other.source && self.id == other.id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteUse {
    pub emote: EmoteKey,
    pub count: u32,
    /// Unix seconds.
    #[ts(type = "number")]
    pub last: i64,
}

impl Emotes {
    /// Counts emotes sent in one message, each once.
    pub fn record(&mut self, used: &[EmoteKey], now: i64) {
        for key in used {
            match self.usage.iter_mut().find(|u| u.emote.same(key)) {
                Some(u) => {
                    u.count = u.count.saturating_add(1);
                    u.last = now;
                    u.emote.name.clone_from(&key.name);
                }
                None => {
                    if self.usage.len() >= MAX_USAGE {
                        self.usage.pop();
                    }
                    self.usage.push(EmoteUse {
                        emote: key.clone(),
                        count: 1,
                        last: now,
                    });
                }
            }
            self.rank();
        }
    }

    fn rank(&mut self) {
        self.usage
            .sort_by(|a, b| b.count.cmp(&a.count).then(b.last.cmp(&a.last)));
    }
}

/// Drops invalid and repeated emotes, keeping the first.
pub(super) fn dedupe(list: &mut Vec<EmoteKey>, max: usize) {
    let mut seen = HashSet::new();
    list.retain(|k| k.valid() && seen.insert((k.source, k.id.clone())));
    list.truncate(max);
}

impl Schema for Emotes {
    fn sanitize(&mut self) {
        dedupe(&mut self.favorites, MAX_FAVORITES);
        let mut seen = HashSet::new();
        self.usage
            .retain(|u| u.count > 0 && u.emote.valid() && seen.insert((u.emote.source, u.emote.id.clone())));
        self.rank();
        self.usage.truncate(MAX_USAGE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(id: &str) -> EmoteKey {
        EmoteKey {
            source: EmoteSource::Seventv,
            id: id.into(),
            name: format!("e{id}"),
        }
    }

    #[test]
    fn usage_ranks_by_count_then_recency() {
        let mut e = Emotes::default();
        e.record(&[key("a"), key("b")], 1);
        e.record(&[key("b")], 2);
        e.record(&[key("c")], 3);
        let order: Vec<&str> = e.usage.iter().map(|u| u.emote.id.as_str()).collect();
        assert_eq!(order, ["b", "c", "a"]);
        assert_eq!(e.usage[0].count, 2);
    }

    #[test]
    fn usage_drops_the_least_used_when_full() {
        let mut e = Emotes::default();
        for i in 0..MAX_USAGE {
            e.record(&[key(&i.to_string()), key("top")], i as i64);
        }
        e.record(&[key("new")], 1_000);
        assert_eq!(e.usage.len(), MAX_USAGE);
        assert_eq!(e.usage[0].emote.id, "top");
        assert!(e.usage.iter().any(|u| u.emote.id == "new"));
    }

    #[test]
    fn sanitize_drops_repeats_and_empty_ids() {
        let mut e = Emotes {
            favorites: vec![key("a"), key("a"), key("")],
            usage: vec![EmoteUse {
                emote: key("x"),
                count: 0,
                last: 0,
            }],
        };
        e.sanitize();
        assert_eq!(e.favorites, [key("a")]);
        assert!(e.usage.is_empty());
    }
}
