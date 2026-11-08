//! V110-C59 When a highlight or ignore rule applies: in some channels, categories, while
//! live or offline, with a role of yours there, or after a burst of messages. A rule
//! without one (`None`, every older file) always applies.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

const MAX_ITEMS: usize = 100;
const MAX_ITEM_CHARS: usize = 60;
const MAX_BURST: u8 = 50;
const MAX_BURST_SECS: u16 = 600;

/// Live or offline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum LiveWhen {
    Live,
    Offline,
}

/// Your role in the channel, at least this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum RoleWhen {
    /// VIP, mod or the broadcaster.
    Vip,
    /// Mod (or lead mod) or the broadcaster.
    Mod,
    Broadcaster,
}

/// Every condition set must hold. Empty lists and `None` are not set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct When {
    /// Channel logins.
    pub channels: Vec<String>,
    /// Never in `channels`, instead of only there.
    pub not_channels: bool,
    /// Stream categories (Twitch's names, any case). Only while live: offline has none.
    pub categories: Vec<String>,
    #[ts(optional = nullable)]
    pub live: Option<LiveWhen>,
    #[ts(optional = nullable)]
    pub role: Option<RoleWhen>,
    /// Only once the sender wrote this many messages within `burst_secs` (this one counts).
    /// 0: off.
    pub burst: u8,
    pub burst_secs: u16,
    /// Only for a user's first message in the channel.
    pub first_only: bool,
}

impl When {
    /// Nothing set: the rule always applies.
    pub fn is_always(&self) -> bool {
        self.channels.is_empty()
            && self.categories.is_empty()
            && self.live.is_none()
            && self.role.is_none()
            && self.burst == 0
            && !self.first_only
    }

    /// Logins and categories lowercase, sorted and once each (the matcher binary-searches
    /// them); a burst in range.
    fn sanitize(&mut self) {
        let clean = |list: &mut Vec<String>, login: bool| {
            for s in list.iter_mut() {
                let t = s.trim();
                let t = if login { t.trim_start_matches(['#', '@']) } else { t };
                *s = t.chars().take(MAX_ITEM_CHARS).collect::<String>().to_lowercase();
            }
            list.retain(|s| !s.is_empty());
            list.sort();
            list.dedup();
            list.truncate(MAX_ITEMS);
        };
        clean(&mut self.channels, true);
        clean(&mut self.categories, false);
        if self.channels.is_empty() {
            self.not_channels = false;
        }
        self.burst = self.burst.min(MAX_BURST);
        if self.burst < 2 {
            self.burst = 0;
        }
        self.burst_secs = if self.burst == 0 { 0 } else { self.burst_secs.clamp(1, MAX_BURST_SECS) };
    }
}

/// Cleans a rule's conditions; none set becomes `None`, so the file stays as it was.
pub(crate) fn sanitize(when: &mut Option<When>) {
    if let Some(w) = when.as_mut() {
        w.sanitize();
        if w.is_always() {
            *when = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_drops_empty() {
        let mut w = Some(When {
            channels: vec![" #Foo".into(), "bar".into(), "foo".into(), "".into()],
            categories: vec!["Just Chatting".into()],
            burst: 3,
            burst_secs: 0,
            ..When::default()
        });
        sanitize(&mut w);
        let w = w.unwrap();
        assert_eq!(w.channels, ["bar", "foo"]);
        assert_eq!(w.categories, ["just chatting"]);
        assert_eq!((w.burst, w.burst_secs), (3, 1));

        let mut none = Some(When { not_channels: true, burst: 1, burst_secs: 9, ..When::default() });
        sanitize(&mut none);
        assert_eq!(none, None, "nothing that holds anything");
    }
}
