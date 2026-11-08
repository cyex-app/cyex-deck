//! V110-C59 Rule conditions, ready to check. Built once with the rules; per message it's a
//! few compares and binary searches on what the room already knows, no allocation.

use std::collections::{HashMap, VecDeque};

use cyex_settings::schema::{LiveWhen, RoleWhen, When};

use crate::model::ModRole;

/// One rule's conditions. A rule with none has no `Cond` (`None`).
#[derive(Debug)]
pub struct Cond {
    /// Sorted, lowercase.
    channels: Box<[String]>,
    not_channels: bool,
    /// Sorted, lowercase.
    categories: Box<[String]>,
    live: Option<LiveWhen>,
    role: Option<RoleWhen>,
    /// Messages, within this many ms.
    burst: Option<(usize, i64)>,
    first_only: bool,
}

/// What the message's room knows, for conditions.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ctx<'a> {
    /// The room's login (lowercase).
    pub channel: &'a str,
    /// `None`: not known (not a tab channel, or not looked up yet). Live and offline
    /// conditions don't hold then.
    pub live: Option<bool>,
    /// Lowercase; `None` while offline or not known.
    pub category: Option<&'a str>,
    pub role: Option<ModRole>,
    pub vip: bool,
    /// The sender's first message in the channel.
    pub first: bool,
    /// The message's time, unix ms.
    pub ts: i64,
    /// The sender's recent message times, this one included. `None`: not counted (history).
    pub recent: Option<&'a VecDeque<i64>>,
}

fn sorted(list: &[String]) -> Box<[String]> {
    let mut v: Vec<String> = list
        .iter()
        .map(|s| s.trim().trim_start_matches(['#', '@']).to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v.into_boxed_slice()
}

fn has(list: &[String], x: &str) -> bool {
    list.binary_search_by(|e| e.as_str().cmp(x)).is_ok()
}

impl Cond {
    /// `None` when nothing is set: the rule always applies.
    pub fn new(when: Option<&When>) -> Option<Self> {
        let w = when.filter(|w| !w.is_always())?;
        let channels = sorted(&w.channels);
        let c = Self {
            not_channels: w.not_channels && !channels.is_empty(),
            channels,
            categories: sorted(&w.categories),
            live: w.live,
            role: w.role,
            burst: (w.burst >= 2).then(|| (usize::from(w.burst), i64::from(w.burst_secs.max(1)) * 1000)),
            first_only: w.first_only,
        };
        let empty = c.channels.is_empty()
            && c.categories.is_empty()
            && c.live.is_none()
            && c.role.is_none()
            && c.burst.is_none()
            && !c.first_only;
        (!empty).then_some(c)
    }

    /// Every condition set holds. Cheapest first.
    pub fn holds(&self, c: &Ctx) -> bool {
        if self.first_only && !c.first {
            return false;
        }
        if let Some(want) = self.live {
            let live = match want {
                LiveWhen::Live => true,
                LiveWhen::Offline => false,
            };
            if c.live != Some(live) {
                return false;
            }
        }
        if let Some(want) = self.role {
            let ok = match want {
                RoleWhen::Vip => c.vip || c.role.is_some(),
                RoleWhen::Mod => c.role.is_some(),
                RoleWhen::Broadcaster => c.role == Some(ModRole::Broadcaster),
            };
            if !ok {
                return false;
            }
        }
        if !self.channels.is_empty() && has(&self.channels, c.channel) == self.not_channels {
            return false;
        }
        if !self.categories.is_empty() && !c.category.is_some_and(|cat| has(&self.categories, cat)) {
            return false;
        }
        if let Some((n, ms)) = self.burst {
            let from = c.ts - ms;
            let count = c.recent.map_or(0, |r| r.iter().rev().take_while(|t| **t >= from).take(n).count());
            if count < n {
                return false;
            }
        }
        true
    }

    pub fn burst(&self) -> Option<(usize, i64)> {
        self.burst
    }
}

/// A rule with no conditions always applies.
pub fn holds(cond: Option<&Cond>, c: &Ctx) -> bool {
    cond.is_none_or(|x| x.holds(c))
}

/// Recent message times per sender, only kept while some rule has a burst condition.
#[derive(Debug, Default)]
pub struct Bursts {
    by: HashMap<String, VecDeque<i64>>,
    noted: u32,
}

/// Every this many messages, senders with nothing recent are dropped.
const SWEEP: u32 = 512;

impl Bursts {
    /// Notes `user`'s message at `ts` and returns their recent times (this one last).
    /// `keep`: the longest burst, as (messages, ms).
    pub fn note(&mut self, user: &str, ts: i64, keep: (usize, i64)) -> &VecDeque<i64> {
        let (n, ms) = keep;
        self.noted = self.noted.wrapping_add(1);
        if self.noted.is_multiple_of(SWEEP) {
            self.by.retain(|_, t| t.back().is_some_and(|last| ts - last <= ms));
        }
        let times = self.by.entry(user.to_owned()).or_default();
        while times.front().is_some_and(|t| ts - t > ms) || times.len() >= n {
            times.pop_front();
        }
        times.push_back(ts);
        times
    }

    /// `user`'s recent times, if any are kept.
    pub fn get(&self, user: &str) -> Option<&VecDeque<i64>> {
        self.by.get(user)
    }

    #[cfg(test)]
    pub fn senders(&self) -> usize {
        self.by.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn when(f: impl FnOnce(&mut When)) -> Cond {
        let mut w = When::default();
        f(&mut w);
        Cond::new(Some(&w)).expect("something set")
    }

    fn ctx() -> Ctx<'static> {
        Ctx { channel: "foo", ts: 100_000, ..Ctx::default() }
    }

    #[test]
    fn nothing_set_is_always() {
        assert!(Cond::new(None).is_none());
        assert!(Cond::new(Some(&When::default())).is_none());
        assert!(Cond::new(Some(&When { not_channels: true, ..When::default() })).is_none());
        assert!(holds(None, &ctx()));
    }

    #[test]
    fn channels_only_and_never() {
        let only = when(|w| w.channels = vec!["Foo".into(), "#bar".into()]);
        assert!(only.holds(&ctx()));
        assert!(only.holds(&Ctx { channel: "bar", ..ctx() }));
        assert!(!only.holds(&Ctx { channel: "baz", ..ctx() }));
        let never = when(|w| {
            w.channels = vec!["foo".into()];
            w.not_channels = true;
        });
        assert!(!never.holds(&ctx()));
        assert!(never.holds(&Ctx { channel: "baz", ..ctx() }));
    }

    #[test]
    fn categories() {
        let c = when(|w| w.categories = vec!["Just Chatting".into(), "Minecraft".into()]);
        assert!(c.holds(&Ctx { category: Some("minecraft"), ..ctx() }));
        assert!(!c.holds(&Ctx { category: Some("fortnite"), ..ctx() }));
        assert!(!c.holds(&ctx()), "offline or not known: no category");
    }

    #[test]
    fn live_state() {
        let live = when(|w| w.live = Some(LiveWhen::Live));
        let off = when(|w| w.live = Some(LiveWhen::Offline));
        assert!(live.holds(&Ctx { live: Some(true), ..ctx() }));
        assert!(!live.holds(&Ctx { live: Some(false), ..ctx() }));
        assert!(off.holds(&Ctx { live: Some(false), ..ctx() }));
        assert!(!off.holds(&Ctx { live: Some(true), ..ctx() }));
        assert!(!live.holds(&ctx()) && !off.holds(&ctx()), "not known: neither");
    }

    #[test]
    fn own_role() {
        let vip = when(|w| w.role = Some(RoleWhen::Vip));
        let m = when(|w| w.role = Some(RoleWhen::Mod));
        let b = when(|w| w.role = Some(RoleWhen::Broadcaster));
        let viewer = ctx();
        let as_vip = Ctx { vip: true, ..ctx() };
        let as_mod = Ctx { role: Some(ModRole::LeadModerator), ..ctx() };
        let owner = Ctx { role: Some(ModRole::Broadcaster), ..ctx() };
        assert!(!vip.holds(&viewer) && vip.holds(&as_vip) && vip.holds(&as_mod) && vip.holds(&owner));
        assert!(!m.holds(&viewer) && !m.holds(&as_vip) && m.holds(&as_mod) && m.holds(&owner));
        assert!(!b.holds(&as_mod) && b.holds(&owner));
    }

    #[test]
    fn first_message_only() {
        let c = when(|w| w.first_only = true);
        assert!(!c.holds(&ctx()));
        assert!(c.holds(&Ctx { first: true, ..ctx() }));
    }

    #[test]
    fn burst_counts_within_the_window() {
        let c = when(|w| {
            w.burst = 3;
            w.burst_secs = 10;
        });
        let mut b = Bursts::default();
        let keep = c.burst().unwrap();
        let at = |b: &mut Bursts, ts: i64| {
            let recent = b.note("u", ts, keep).clone();
            c.holds(&Ctx { ts, recent: Some(&recent), ..ctx() })
        };
        assert!(!at(&mut b, 0));
        assert!(!at(&mut b, 4_000));
        assert!(at(&mut b, 9_000), "third within 10 s");
        assert!(at(&mut b, 12_000), "4 s, 9 s, 12 s");
        assert!(!at(&mut b, 30_000), "the rest are too old");
        assert!(!c.holds(&ctx()), "history isn't counted");
    }

    #[test]
    fn bursts_forget_quiet_senders() {
        let mut b = Bursts::default();
        for i in 0..SWEEP - 1 {
            b.note(&format!("u{i}"), i64::from(i), (3, 10));
        }
        b.note("late", 1_000_000, (3, 10));
        assert_eq!(b.senders(), 1, "the sweep at the 512th line drops the quiet ones");
    }

    #[test]
    fn combined_all_must_hold() {
        let c = when(|w| {
            w.channels = vec!["foo".into()];
            w.live = Some(LiveWhen::Live);
            w.role = Some(RoleWhen::Mod);
        });
        let all = Ctx { live: Some(true), role: Some(ModRole::Moderator), ..ctx() };
        assert!(c.holds(&all));
        assert!(!c.holds(&Ctx { channel: "bar", ..all }));
        assert!(!c.holds(&Ctx { live: Some(false), ..all }));
        assert!(!c.holds(&Ctx { role: None, ..all }));
    }
}
