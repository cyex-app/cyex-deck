//! V107-F10 Which of the account's Twitch emotes it may use in a channel. Twitch's lists
//! can hold more than that (follower emotes of other channels), and completion must
//! offer only what a message would actually show.
//!
//! - Subscriber emotes: only while subscribed to the owner, then in any channel.
//! - Follower emotes: in their own channel by following (or being subscribed), in other
//!   channels only while subscribed to the owner.
//! - Bits, tier and the rest: owned when the list has them.

use std::collections::HashSet;

use cyex_twitch::gql::{EmoteKind, EmoteSet};

/// Why an emote can't be used in `current`, `None` when it can.
pub fn lock_reason(
    kind: EmoteKind,
    owner: Option<&str>,
    current: &str,
    subbed: &HashSet<&str>,
    follows_current: bool,
) -> Option<&'static str> {
    let owner = owner?;
    let sub = subbed.contains(owner);
    match kind {
        EmoteKind::Subscription if !sub => Some("Sub to use"),
        EmoteKind::Follower if owner == current && !(follows_current || sub) => Some("Follow to use"),
        EmoteKind::Follower if owner != current && !sub => Some("Sub to use"),
        _ => None,
    }
}

/// Owners the account is subscribed to, as far as its lists show: a sub emote is there
/// only for them.
pub fn subbed_owners(sets: &[EmoteSet]) -> HashSet<&str> {
    sets.iter()
        .filter(|s| s.emotes.iter().any(|e| e.kind == EmoteKind::Subscription))
        .filter_map(|s| s.owner.as_ref().map(|o| o.id.as_str()))
        .collect()
}

/// Whether the lists carry follower emotes of `channel` (the only case a follow check
/// is worth a request).
pub fn has_follower_emotes(sets: &[EmoteSet], channel: &str) -> bool {
    sets.iter().any(|s| {
        s.owner.as_ref().is_some_and(|o| o.id == channel) && s.emotes.iter().any(|e| e.kind == EmoteKind::Follower)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use EmoteKind::{Bits, Follower, Subscription};

    /// (kind, owner is this channel, subbed to owner, follows this channel) -> locked.
    #[test]
    fn who_may_use_what() {
        let table = [
            // subscriber emotes: the sub decides, follow and channel don't
            (Subscription, true, true, false, false),
            (Subscription, true, false, true, true),
            (Subscription, false, true, false, false),
            (Subscription, false, false, true, true),
            // follower emotes, own channel: follow or sub
            (Follower, true, false, true, false),
            (Follower, true, true, false, false),
            (Follower, true, true, true, false),
            (Follower, true, false, false, true),
            // follower emotes, other channel: sub only, following doesn't help
            (Follower, false, true, false, false),
            (Follower, false, true, true, false),
            (Follower, false, false, true, true),
            (Follower, false, false, false, true),
            // bits and the rest: owned is enough
            (Bits, true, false, false, false),
            (Bits, false, false, true, false),
        ];
        for (kind, here, sub, follow, locked) in table {
            let owner = if here { "7" } else { "9" };
            let subbed: HashSet<&str> = if sub { HashSet::from([owner]) } else { HashSet::new() };
            let got = lock_reason(kind, Some(owner), "7", &subbed, follow).is_some();
            assert_eq!(got, locked, "{kind:?} here={here} sub={sub} follow={follow}");
        }
    }

    #[test]
    fn no_owner_is_never_locked() {
        assert_eq!(lock_reason(Subscription, None, "7", &HashSet::new(), false), None);
    }
}
