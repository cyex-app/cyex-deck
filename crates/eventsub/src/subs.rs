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
}

impl ChannelSubs {
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

/// V107-A2 Which channels to subscribe next, from `wanted` in priority order (the shown tab
/// first, then the order they were opened), with the types each still needs. Skips ones in
/// flight, settled or backing off, and leaves out what would pass the session's cap.
/// `in_flight` counts subscriptions being made right now. Second value: channels left out.
pub fn next_channels<'a>(
    wanted: &[&'a str],
    state: &HashMap<String, ChannelSubs>,
    pending: &HashSet<String>,
    in_flight: usize,
    now_ms: i64,
) -> (Vec<(&'a str, Vec<usize>)>, usize) {
    let mut held: usize = state.values().map(ChannelSubs::held).sum::<usize>() + in_flight;
    let mut out = Vec::new();
    let mut left_out = 0;
    for &cid in wanted {
        if pending.contains(cid) {
            continue;
        }
        let todo = state.get(cid).map_or_else(|| (0..MOD_TYPES.len()).collect(), |s| s.todo(now_ms));
        if todo.is_empty() {
            continue;
        }
        if held + todo.len() > MAX_PER_SESSION {
            left_out += 1;
            continue;
        }
        held += todo.len();
        out.push((cid, todo));
    }
    (out, left_out)
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
        let (next, left) = next_channels(&wanted, &none, &pending, 0, 0);
        assert_eq!((next.len(), left), (27, 3));
        assert_eq!((next[0].0, next[26].0), ("c0", "c26"), "priority order kept");
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
        let (next, left) = next_channels(&wanted, &state, &pending, MOD_TYPES.len(), 10);
        assert_eq!(next[0].0, "c3");
        // 300 - 11 (c0) - 10 (c1) - 11 (c2 in flight) = 268: 24 more.
        assert_eq!((next.len(), left), (24, 3));
        // After the wait the one failed type goes again, alone.
        let (next, _) = next_channels(&["c1"], &state, &HashSet::new(), 0, RETRY_MS);
        assert_eq!(next, vec![("c1", vec![0])]);
    }
}
