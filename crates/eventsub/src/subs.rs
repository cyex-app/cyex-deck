//! What a moderator's session follows for one channel. Every type's condition names the
//! channel and the moderator, so they cost nothing against Twitch's limit.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

/// One subscription to make.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sub {
    pub kind: &'static str,
    pub version: &'static str,
    pub broadcaster_id: String,
    pub moderator_id: String,
}

impl Sub {
    pub fn condition(&self) -> Value {
        json!({ "broadcaster_user_id": self.broadcaster_id, "moderator_user_id": self.moderator_id })
    }
}

/// Types and versions for a channel the account moderates.
pub const MOD_TYPES: [(&str, &str); 11] = [
    ("channel.moderate", "2"),
    ("automod.message.hold", "2"),
    ("automod.message.update", "2"),
    ("channel.follow", "2"),
    ("channel.suspicious_user.message", "1"),
    ("channel.suspicious_user.update", "1"),
    ("channel.unban_request.create", "1"),
    ("channel.unban_request.resolve", "1"),
    ("channel.shield_mode.begin", "1"),
    ("channel.shield_mode.end", "1"),
    ("channel.warning.acknowledge", "1"),
];

/// Twitch's cap per websocket.
pub const MAX_PER_SESSION: usize = 300;

/// V107-A2 What one subscribe answer means for that subscription on this session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Made, or there already (409).
    Done,
    /// Won't work on this session: a missing scope or a bad condition. Not tried again.
    Final,
    /// 429: the socket's cap or the rate limit. Stop for now.
    Full,
    /// Twitch or the network failed: try again later.
    Retry,
}

/// `status` is `None` for a network error, `message` Twitch's text.
pub fn classify(status: Option<u16>, message: Option<&str>) -> Answer {
    match status {
        Some(200..=299 | 409) => Answer::Done,
        _ if message.is_some_and(|m| m.contains("already")) => Answer::Done,
        Some(429) => Answer::Full,
        Some(400..=499) => Answer::Final,
        _ => Answer::Retry,
    }
}

/// How many more channels' mod types fit on a session holding `held` subscriptions.
pub fn channels_that_fit(held: usize) -> usize {
    MAX_PER_SESSION.saturating_sub(held) / MOD_TYPES.len()
}

/// First wait after a failed subscribe, doubled per failure up to [`RETRY_MAX_MS`].
pub const RETRY_MS: i64 = 60_000;
pub const RETRY_MAX_MS: i64 = 30 * 60_000;

/// V107-A2 One channel's mod types on one session: which are settled (made, or final),
/// which count against the cap, and when failed ones may go again.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChannelSubs {
    /// Bit `i`: `MOD_TYPES[i]` is made or won't work here.
    settled: u16,
    /// Bit `i`: `MOD_TYPES[i]` is made (counts against the cap).
    made: u16,
    fails: u32,
    next_ms: i64,
    /// V107-R17 Twitch's id per made type, so it can be deleted again (none after a 409).
    ids: [Option<String>; MOD_TYPES.len()],
}

impl ChannelSubs {
    /// V107-R17 Made with this subscription id.
    pub fn made_as(&mut self, i: usize, id: String, now_ms: i64) {
        self.answer(i, Answer::Done, now_ms);
        self.ids[i] = Some(id);
    }

    pub fn id(&self, i: usize) -> Option<&str> {
        self.ids[i].as_deref()
    }

    /// V107-R17 Deleted again: free on the session and wanted like a new one.
    pub fn dropped(&mut self, i: usize) {
        self.settled &= !(1 << i);
        self.made &= !(1 << i);
        self.ids[i] = None;
    }

    /// The types (indexes into [`MOD_TYPES`]) to send now; empty while backing off.
    pub fn todo(&self, now_ms: i64) -> Vec<usize> {
        if now_ms < self.next_ms {
            return Vec::new();
        }
        (0..MOD_TYPES.len()).filter(|i| self.settled & (1 << i) == 0).collect()
    }

    pub fn answer(&mut self, i: usize, a: Answer, now_ms: i64) {
        match a {
            Answer::Done => {
                self.settled |= 1 << i;
                self.made |= 1 << i;
            }
            Answer::Final => self.settled |= 1 << i,
            Answer::Full | Answer::Retry => {
                self.fails += 1;
                let wait = RETRY_MS.saturating_mul(1 << (self.fails - 1).min(10)).min(RETRY_MAX_MS);
                self.next_ms = now_ms + wait;
            }
        }
    }

    /// Subscriptions this channel holds on the session.
    pub fn held(&self) -> usize {
        self.made.count_ones() as usize
    }

    /// Whether `kind` is made (e.g. `channel.follow`, which the stream manager hands over).
    pub fn has(&self, kind: &str) -> bool {
        MOD_TYPES.iter().position(|(k, _)| *k == kind).is_some_and(|i| self.made & (1 << i) != 0)
    }

    /// When failed types may go again, if any wait.
    pub fn retry_at(&self) -> Option<i64> {
        (self.settled.count_ones() as usize != MOD_TYPES.len() && self.fails > 0).then_some(self.next_ms)
    }
}

/// V107-R17 AutoMod's two types: every channel gets these first.
pub const AUTOMOD: [usize; 2] = [1, 2];
/// The other types, in the order they fill a channel: `channel.moderate` first, warning
/// acknowledge last.
pub const REST: [usize; 9] = [0, 3, 4, 5, 6, 7, 8, 9, 10];

/// V107-R17 What one session does next.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan<'a> {
    /// Delete these first (channel, index into [`MOD_TYPES`]), to make room for `add`.
    pub drop: Vec<(&'a str, usize)>,
    /// Then make these, in this order.
    pub add: Vec<(&'a str, usize)>,
    /// Wanted channels whose AutoMod didn't fit.
    pub without_automod: Vec<&'a str>,
}

/// V107-R17 Plans one session within `cap`, from `wanted` in priority order (the first
/// `fronts` are shown, then the order they were opened). AutoMod for every channel comes
/// first, then the rest of the shown channels, then the rest of the others. AutoMod and the
/// shown channels' sets may push out what is made but matters less: closed channels' types
/// first, then the rest of the lowest-priority channels. Never another open channel's AutoMod.
/// The others only push out closed channels. Channels in `pending` (a task running) are left alone;
/// `in_flight` subscriptions count as held.
pub fn plan<'a>(
    wanted: &[&'a str],
    fronts: usize,
    state: &'a HashMap<String, ChannelSubs>,
    pending: &HashSet<String>,
    in_flight: usize,
    now_ms: i64,
    cap: usize,
) -> Plan<'a> {
    let held: usize = state.values().map(ChannelSubs::held).sum::<usize>() + in_flight;
    let rank: HashMap<&str, usize> = wanted.iter().enumerate().map(|(r, c)| (*c, r)).collect();
    // What may go, sorted so the worst is last: (class, rank, position, channel, type).
    // Class 3 closed, 2 others' rest, 1 shown channels' rest.
    let mut spare: Vec<(u8, usize, usize, &'a str, usize)> = Vec::new();
    for (cid, s) in state.iter().filter(|(c, _)| !pending.contains(*c)) {
        match rank.get(cid.as_str()) {
            None => spare.extend((0..MOD_TYPES.len()).filter(|&i| s.id(i).is_some()).map(|i| (3, 0, i, cid.as_str(), i))),
            Some(&r) => {
                let class = if r < fronts { 1 } else { 2 };
                let made = REST.iter().enumerate().filter(|(_, i)| s.id(**i).is_some());
                spare.extend(made.map(|(pos, &i)| (class, r, pos, cid.as_str(), i)));
            }
        }
    }
    spare.sort_unstable();
    let mut room = Room { free: cap.saturating_sub(held), spare, plan: Plan::default() };
    let todo: Vec<Option<Vec<usize>>> = wanted
        .iter()
        .map(|c| {
            (!pending.contains(*c)).then(|| state.get(*c).map_or_else(|| (0..MOD_TYPES.len()).collect(), |s| s.todo(now_ms)))
        })
        .collect();
    for (&cid, t) in wanted.iter().zip(&todo) {
        let Some(t) = t else { continue };
        let need: Vec<usize> = AUTOMOD.into_iter().filter(|i| t.contains(i)).collect();
        // Both or neither: one alone is no feed.
        if room.available(0) < need.len() {
            room.plan.without_automod.push(cid);
            continue;
        }
        need.into_iter().for_each(|i| _ = room.take(cid, i, 0));
    }
    for shown in [true, false] {
        let class = if shown { 1 } else { 2 };
        for (_, (&cid, t)) in wanted.iter().zip(&todo).enumerate().filter(|(r, _)| (*r < fronts) == shown) {
            let Some(t) = t else { continue };
            for i in REST.into_iter().filter(|i| t.contains(i)) {
                if !room.take(cid, i, class) {
                    break;
                }
            }
        }
    }
    room.plan
}

/// [`plan`]'s free room and what may still go.
struct Room<'a> {
    free: usize,
    spare: Vec<(u8, usize, usize, &'a str, usize)>,
    plan: Plan<'a>,
}

impl<'a> Room<'a> {
    /// Slots a need of `class` can get: free room, and made ones of a lower class (higher number).
    fn available(&self, class: u8) -> usize {
        let pushable = self.spare.iter().filter(|s| s.0 > class).count();
        self.free + pushable
    }

    fn take(&mut self, cid: &'a str, i: usize, class: u8) -> bool {
        if self.free > 0 {
            self.free -= 1;
        } else if self.spare.last().is_some_and(|s| s.0 > class) {
            let Some((_, _, _, c, j)) = self.spare.pop() else { return false };
            self.plan.drop.push((c, j));
        } else {
            return false;
        }
        self.plan.add.push((cid, i));
        true
    }
}

pub fn for_moderator(broadcaster_id: &str, moderator_id: &str) -> Vec<Sub> {
    MOD_TYPES
        .iter()
        .map(|(kind, version)| Sub {
            kind,
            version,
            broadcaster_id: broadcaster_id.to_owned(),
            moderator_id: moderator_id.to_owned(),
        })
        .collect()
}

/// 1.35 What the stream manager's session follows for the broadcaster's own channel: each
/// type with its version and condition. `channel.follow` names the moderator (cost 0);
/// `channel.raid` watches raids coming *in* (`to_broadcaster_user_id`); the rest name only
/// the broadcaster. Subs, cheers and redemptions need their read scopes.
pub fn for_broadcaster(broadcaster_id: &str, moderator_id: &str) -> Vec<(&'static str, &'static str, Value)> {
    let b = || json!({ "broadcaster_user_id": broadcaster_id });
    vec![
        (
            "channel.follow",
            "2",
            json!({ "broadcaster_user_id": broadcaster_id, "moderator_user_id": moderator_id }),
        ),
        ("channel.subscribe", "1", b()),
        ("channel.subscription.gift", "1", b()),
        ("channel.subscription.message", "1", b()),
        ("channel.cheer", "1", b()),
        ("channel.raid", "1", json!({ "to_broadcaster_user_id": broadcaster_id })),
        ("channel.channel_points_custom_reward_redemption.add", "1", b()),
    ]
}

/// 1.26 Whispers to `user_id`, for accounts with only a Helix sign-in (`user:read:whispers`).
/// Received ones only; Twitch doesn't echo what you send.
pub fn whispers(user_id: &str) -> (&'static str, &'static str, Value) {
    ("user.whisper.message", "1", json!({ "user_id": user_id }))
}

/// The channel an event or revocation is about.
pub fn channel_of(condition: &Value) -> Option<&str> {
    condition
        .get("broadcaster_user_id")
        .or_else(|| condition.get("to_broadcaster_user_id"))?
        .as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditions_name_both() {
        let subs = for_moderator("1", "2");
        assert_eq!(subs.len(), MOD_TYPES.len());
        let c = subs[0].condition();
        assert_eq!((c["broadcaster_user_id"].as_str(), c["moderator_user_id"].as_str()), (Some("1"), Some("2")));
        assert_eq!(channel_of(&c), Some("1"));
    }

    #[test]
    fn broadcaster_conditions() {
        let subs = for_broadcaster("1", "2");
        assert_eq!(subs.len(), 7);
        // follow names the moderator; a raid names the channel it comes in to.
        assert_eq!(subs[0].2["moderator_user_id"].as_str(), Some("2"));
        let raid = &subs.iter().find(|(k, _, _)| *k == "channel.raid").unwrap().2;
        assert_eq!(channel_of(raid), Some("1"));
    }

    #[test]
    fn v107_a2_answers() {
        assert_eq!(classify(Some(202), None), Answer::Done);
        assert_eq!(classify(Some(409), Some("subscription already exists")), Answer::Done);
        assert_eq!(classify(Some(400), Some("subscription already exists")), Answer::Done);
        assert_eq!(classify(Some(403), Some("missing scope")), Answer::Final);
        assert_eq!(classify(Some(400), Some("invalid condition")), Answer::Final);
        assert_eq!(classify(Some(429), None), Answer::Full);
        assert_eq!(classify(Some(503), None), Answer::Retry);
        assert_eq!(classify(None, None), Answer::Retry);
    }

    #[test]
    fn v107_a2_cap_per_session() {
        assert_eq!(channels_that_fit(0), 27);
        assert_eq!(channels_that_fit(297), 0);
        assert_eq!(channels_that_fit(400), 0);
    }

    #[test]
    fn v107_a2_only_failed_types_go_again_with_backoff() {
        let mut c = ChannelSubs::default();
        assert_eq!(c.todo(0).len(), MOD_TYPES.len());
        for (i, (kind, _)) in MOD_TYPES.iter().enumerate() {
            let a = match *kind {
                "channel.warning.acknowledge" => Answer::Final,
                "channel.shield_mode.end" => Answer::Retry,
                _ => Answer::Done,
            };
            c.answer(i, a, 0);
        }
        assert_eq!(c.held(), MOD_TYPES.len() - 2);
        assert!(c.has("channel.follow"));
        assert!(!c.has("channel.warning.acknowledge"));
        assert!(c.todo(1).is_empty(), "backing off");
        assert_eq!(c.retry_at(), Some(RETRY_MS));
        let shield_end = MOD_TYPES.iter().position(|(k, _)| *k == "channel.shield_mode.end").unwrap();
        assert_eq!(c.todo(RETRY_MS), vec![shield_end], "only the failed one, never the final one");
        c.answer(shield_end, Answer::Retry, RETRY_MS);
        assert_eq!(c.retry_at(), Some(3 * RETRY_MS), "doubled");
        c.answer(shield_end, Answer::Done, 3 * RETRY_MS);
        assert!(c.todo(i64::MAX).is_empty());
        assert_eq!(c.retry_at(), None);
        let mut slow = ChannelSubs::default();
        for _ in 0..20 {
            slow.answer(0, Answer::Retry, 0);
        }
        assert_eq!(slow.retry_at(), Some(RETRY_MAX_MS));
    }

    #[test]
    fn v107_a2_next_channels_keep_order_and_cap() {
        let ids: Vec<String> = (0..30).map(|i| format!("c{i}")).collect();
        let wanted: Vec<&str> = ids.iter().map(String::as_str).collect();
        let (none, pending) = (HashMap::new(), HashSet::new());
        let p = plan(&wanted, 0, &none, &pending, 0, 0, MAX_PER_SESSION);
        assert_eq!((p.add.len(), p.without_automod.len()), (MAX_PER_SESSION, 0));
        assert_eq!((p.add[0], p.add[60]), (("c0", 1), ("c0", 0)), "AutoMod first, then priority order");
        // Settled, in flight or backing off: skipped; in-flight ones count against the cap.
        let mut state = HashMap::new();
        let mut done = ChannelSubs::default();
        (0..MOD_TYPES.len()).for_each(|i| done.answer(i, Answer::Done, 0));
        state.insert("c0".to_owned(), done);
        let mut one_failed = ChannelSubs::default();
        (1..MOD_TYPES.len()).for_each(|i| one_failed.answer(i, Answer::Done, 0));
        one_failed.answer(0, Answer::Retry, 0);
        state.insert("c1".to_owned(), one_failed);
        let pending: HashSet<String> = ["c2".to_owned()].into();
        let p = plan(&wanted, 0, &state, &pending, MOD_TYPES.len(), 10, MAX_PER_SESSION);
        assert_eq!(p.add[0], ("c3", 1));
        assert!(p.add.iter().all(|(c, _)| !["c0", "c1", "c2"].contains(c)));
        // 300 - 11 (c0) - 10 (c1) - 11 (c2 in flight) = 268.
        assert_eq!(p.add.len(), 268);
        // After the wait the one failed type goes again, alone.
        let p = plan(&["c1"], 0, &state, &HashSet::new(), 0, RETRY_MS, MAX_PER_SESSION);
        assert_eq!(p.add, vec![("c1", 0)]);
    }

    type Owned = Vec<(String, usize)>;

    /// One plan, then Twitch says yes to all of it. Returns (dropped, added, without AutoMod).
    fn step(wanted: &[String], fronts: usize, state: &mut HashMap<String, ChannelSubs>) -> (Owned, Owned, Vec<String>) {
        let w: Vec<&str> = wanted.iter().map(String::as_str).collect();
        let p = plan(&w, fronts, state, &HashSet::new(), 0, 0, MAX_PER_SESSION);
        let own = |v: &[(&str, usize)]| v.iter().map(|(c, i)| ((*c).to_owned(), *i)).collect::<Owned>();
        let (drop, add, without) = (own(&p.drop), own(&p.add), p.without_automod.iter().map(|c| (*c).to_owned()).collect());
        for (c, i) in &drop {
            state.get_mut(c).unwrap().dropped(*i);
        }
        for (n, (c, i)) in add.iter().enumerate() {
            state.entry(c.clone()).or_default().made_as(*i, format!("{c}-{i}-{n}"), 0);
        }
        assert!(state.values().map(ChannelSubs::held).sum::<usize>() <= MAX_PER_SESSION, "never past the cap");
        (drop, add, without)
    }

    fn chans(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("c{i}")).collect()
    }

    fn has_automod(s: &ChannelSubs) -> bool {
        AUTOMOD.iter().all(|&i| s.id(i).is_some())
    }

    #[test]
    fn v107_r17_automod_first_table() {
        // (channels, subscriptions made, channels with all 11, channels without AutoMod)
        for (n, made, full, without) in [(10, 110, 10, 0), (27, 297, 27, 0), (100, 300, 11, 0), (150, 300, 0, 0), (200, 300, 0, 50)] {
            let mut state = HashMap::new();
            let wanted = chans(n);
            let (drop, add, out) = step(&wanted, 1, &mut state);
            assert!(drop.is_empty());
            assert_eq!(add.len(), made, "{n} channels");
            assert_eq!(state.values().filter(|s| s.held() == MOD_TYPES.len()).count(), full, "{n} channels");
            assert_eq!(out.len(), without, "{n} channels");
            assert_eq!(state.values().filter(|s| has_automod(s)).count(), n - without, "{n} channels");
            if without > 0 {
                assert_eq!(out[0], format!("c{}", n - without), "the last opened go without");
            }
            // Planned again: nothing to do.
            let (drop, add, _) = step(&wanted, 1, &mut state);
            assert!(drop.is_empty() && add.is_empty(), "{n} channels stable");
        }
        // 100 channels: the rest in order, channel.moderate first.
        let mut state = HashMap::new();
        step(&chans(100), 1, &mut state);
        assert!(state["c11"].has("channel.moderate") && state["c11"].held() == 3);
        assert_eq!(state["c12"].held(), 2);
    }

    #[test]
    fn v107_r17_shown_tab_takes_the_lowest_rest_never_automod() {
        let mut state = HashMap::new();
        let mut wanted = chans(100);
        step(&wanted, 1, &mut state);
        // c99 is shown now: its 9 come from the lowest-priority holders, warning acknowledge first.
        let last = wanted.pop().unwrap();
        wanted.insert(0, last);
        let (drop, add, _) = step(&wanted, 1, &mut state);
        let mut want_drop = vec![("c11".to_owned(), 0)];
        want_drop.extend([10, 9, 8, 7, 6, 5, 4, 3].map(|i| ("c10".to_owned(), i)));
        assert_eq!(drop, want_drop);
        assert_eq!(add, REST.map(|i| ("c99".to_owned(), i)).to_vec());
        assert_eq!(state["c99"].held(), MOD_TYPES.len());
        assert!(state.values().all(has_automod), "no AutoMod dropped");
        // Back to c0: c99 keeps its set, nobody is pushed out for a channel not shown.
        let (drop, add, _) = step(&chans(100), 1, &mut state);
        assert!(drop.is_empty() && add.is_empty());
        // A new channel's AutoMod pushes out the lowest-priority rest.
        let mut more = wanted.clone();
        more.push("c100".to_owned());
        let (drop, add, _) = step(&more, 1, &mut state);
        assert_eq!(drop, vec![("c10".to_owned(), 0), ("c9".to_owned(), 10)]);
        assert_eq!(add, vec![("c100".to_owned(), 1), ("c100".to_owned(), 2)]);
    }

    #[test]
    fn v107_r17_closed_channels_go_first() {
        let mut state = HashMap::new();
        step(&chans(27), 1, &mut state);
        // c0 closed, two new channels: room comes from c0 only.
        let mut wanted: Vec<String> = chans(29).split_off(1);
        let (drop, add, _) = step(&wanted, 1, &mut state);
        assert_eq!(add.iter().filter(|(c, _)| c == "c27" || c == "c28").count(), 14);
        assert!(drop.iter().all(|(c, _)| c == "c0"), "{drop:?}");
        // 150 open, all held: one closed channel's AutoMod makes room for one new channel.
        let mut state = HashMap::new();
        wanted = chans(151);
        step(&wanted, 1, &mut state);
        wanted.remove(5);
        let (drop, add, out) = step(&wanted, 1, &mut state);
        assert_eq!(drop.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(), ["c5", "c5"]);
        assert_eq!(add, vec![("c150".to_owned(), 1), ("c150".to_owned(), 2)]);
        assert!(out.is_empty());
    }
}
