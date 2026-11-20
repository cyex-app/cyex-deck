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
/// V111-41 People you mention most keep this many.
const MAX_MENTIONS: usize = 200;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Emotes {
    pub favorites: Vec<EmoteKey>,
    /// Most used first.
    pub usage: Vec<EmoteUse>,
    /// V111-41 Who you `@` most in your own messages, most first: `@` completion ranks them first.
    pub mentions: Vec<MentionUse>,
    /// V108-N06 Extra badge sets from community services. Each is its own switch, off until picked:
    /// turning one on fetches that service’s list.
    pub badge_sets: BadgeSets,
}

/// V108-N06 FFZ:AP, Chatsen, DankChat and Homies badges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeSets {
    pub ffzap: bool,
    pub chatsen: bool,
    pub dankchat: bool,
    pub homies: bool,
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
    /// V112-01 cyex.tv.
    Cyextv,
    Emoji,
}

impl EmoteSource {
    /// C11 The providers the priority order ranks, in the default order. V110-U71 Twitch is one
    /// of them now (it was always first before).
    pub const RANKED: [EmoteSource; 5] = [
        EmoteSource::Twitch,
        EmoteSource::Seventv,
        EmoteSource::Bttv,
        EmoteSource::Ffz,
        EmoteSource::Cyextv,
    ];
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

/// V111-41 One person you mention, by login (lowercase).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MentionUse {
    pub login: String,
    pub count: u32,
    /// Unix seconds.
    #[ts(type = "number")]
    pub last: i64,
}

fn valid_login(l: &str) -> bool {
    (1..=25).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl Emotes {
    /// V111-41 Counts the people one message mentions, each once.
    pub fn record_mentions(&mut self, logins: &[String], now: i64) {
        let mut seen = HashSet::new();
        for raw in logins {
            let login = raw.to_ascii_lowercase();
            if !valid_login(&login) || !seen.insert(login.clone()) {
                continue;
            }
            match self.mentions.iter_mut().find(|m| m.login == login) {
                Some(m) => {
                    m.count = m.count.saturating_add(1);
                    m.last = now;
                }
                None => {
                    if self.mentions.len() >= MAX_MENTIONS {
                        self.mentions.pop();
                    }
                    self.mentions.push(MentionUse { login, count: 1, last: now });
                }
            }
            self.rank_mentions();
        }
    }

    fn rank_mentions(&mut self) {
        self.mentions.sort_by(|a, b| b.count.cmp(&a.count).then(b.last.cmp(&a.last)));
    }

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
        let mut seen = HashSet::new();
        for m in &mut self.mentions {
            m.login.make_ascii_lowercase();
        }
        self.mentions.retain(|m| m.count > 0 && valid_login(&m.login) && seen.insert(m.login.clone()));
        self.rank_mentions();
        self.mentions.truncate(MAX_MENTIONS);
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
    fn v111_41_mentions_rank_by_count_then_recency_once_per_message() {
        let mut e = Emotes::default();
        e.record_mentions(&["Bob".into(), "bob".into(), "amy".into()], 1);
        e.record_mentions(&["zed".into()], 2);
        e.record_mentions(&["zed".into(), "not a login".into()], 3);
        let order: Vec<(&str, u32)> = e.mentions.iter().map(|m| (m.login.as_str(), m.count)).collect();
        assert_eq!(order, [("zed", 2), ("bob", 1), ("amy", 1)]);
        let mut full = Emotes::default();
        for i in 0..MAX_MENTIONS + 5 {
            full.record_mentions(&[format!("u{i}")], i as i64);
        }
        assert_eq!(full.mentions.len(), MAX_MENTIONS);
        let mut bad = Emotes {
            mentions: vec![
                MentionUse { login: "Amy".into(), count: 1, last: 0 },
                MentionUse { login: "amy".into(), count: 3, last: 0 },
                MentionUse { login: "x y".into(), count: 1, last: 0 },
            ],
            ..Emotes::default()
        };
        bad.sanitize();
        assert_eq!(bad.mentions.len(), 1);
    }

    #[test]
    fn badge_sets_start_off_and_read_alone() {
        let e: Emotes = serde_json::from_str(r#"{"badgeSets":{"homies":true}}"#).unwrap();
        assert!(e.badge_sets.homies && !e.badge_sets.ffzap);
        assert_eq!(Emotes::default().badge_sets, BadgeSets::default());
        assert!(!BadgeSets::default().chatsen && !BadgeSets::default().dankchat);
    }

    #[test]
    fn removed_badgehq_key_in_old_files_is_ignored() {
        let e: Emotes = serde_json::from_str(r#"{"badgeSets":{"badgehq":true,"ffzap":true}}"#).unwrap();
        assert!(e.badge_sets.ffzap);
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
            ..Emotes::default()
        };
        e.sanitize();
        assert_eq!(e.favorites, [key("a")]);
        assert!(e.usage.is_empty());
    }
}
