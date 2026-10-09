//! Join budget (1.10, 1.13, 1.23, D7). Twitch lets one account sit in 100 chat rooms at
//! once. Rooms where the account is broadcaster or moderator don't count.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

pub const ROOM_LIMIT: usize = 100;

/// Rooms one account has joined, by channel id. `true` = broadcaster or moderator there.
#[derive(Clone, Debug, Default)]
pub struct JoinBudget {
    rooms: BTreeMap<String, bool>,
}

impl JoinBudget {
    /// Joining an unprivileged room needs a free slot.
    pub fn can_join(&self, channel_id: &str, privileged: bool) -> bool {
        privileged || self.rooms.contains_key(channel_id) || self.counted() < ROOM_LIMIT
    }

    /// Returns false, and changes nothing, if the budget is full.
    pub fn join(&mut self, channel_id: &str, privileged: bool) -> bool {
        if !self.can_join(channel_id, privileged) {
            return false;
        }
        let entry = self.rooms.entry(channel_id.to_owned()).or_default();
        *entry |= privileged;
        true
    }

    pub fn part(&mut self, channel_id: &str) {
        self.rooms.remove(channel_id);
    }

    /// Mod status can change while joined (USERSTATE badges).
    pub fn set_privileged(&mut self, channel_id: &str, privileged: bool) {
        if let Some(p) = self.rooms.get_mut(channel_id) {
            *p = privileged;
        }
    }

    /// Rooms that count toward the limit.
    pub fn counted(&self) -> usize {
        self.rooms.values().filter(|p| !**p).count()
    }

    pub fn total(&self) -> usize {
        self.rooms.len()
    }
}

/// One account's numbers for the counter in the top bar and the account manager.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectionCount {
    pub user_id: String,
    /// Rooms that count toward the limit.
    pub counted: u32,
    /// All joined rooms, moderated ones included.
    pub total: u32,
    pub limit: u32,
}

impl ConnectionCount {
    pub fn of(user_id: &str, budget: &JoinBudget) -> Self {
        let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
        Self {
            user_id: user_id.to_owned(),
            counted: n(budget.counted()),
            total: n(budget.total()),
            limit: n(ROOM_LIMIT),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> JoinBudget {
        let mut b = JoinBudget::default();
        for i in 0..ROOM_LIMIT {
            assert!(b.join(&i.to_string(), false));
        }
        b
    }

    #[test]
    fn caps_unprivileged_rooms_at_100() {
        let mut b = full();
        assert_eq!(b.counted(), 100);
        assert!(!b.join("extra", false));
        assert_eq!(b.total(), 100);
        // Rejoining a room already in the list is fine.
        assert!(b.join("5", false));
        b.part("5");
        assert!(b.join("extra", false));
    }

    #[test]
    fn moderated_rooms_never_count() {
        let mut b = full();
        assert!(b.join("mine", true));
        assert_eq!((b.counted(), b.total()), (100, 101));
        b.set_privileged("7", true);
        assert_eq!(b.counted(), 99);
        b.set_privileged("7", false);
        assert_eq!(b.counted(), 100);
    }

    #[test]
    fn counts_for_the_ui() {
        let mut b = JoinBudget::default();
        b.join("1", false);
        b.join("2", true);
        assert_eq!(
            ConnectionCount::of("me", &b),
            ConnectionCount {
                user_id: "me".into(),
                counted: 1,
                total: 2,
                limit: 100
            }
        );
    }
}
