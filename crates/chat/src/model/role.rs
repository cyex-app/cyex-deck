//! 1.57, V102-21 The one role check: what an account may do in a channel, and what a
//! chatter's badges say they are. Every mod-only control in Rust and the UI reads it.

use serde::Serialize;
use ts_rs::TS;

use super::Badge;

/// Ordered: each role can do what the ones before it can.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModRole {
    Moderator,
    /// Twitch's "lead moderator" (head mod): a mod who can also add and remove mods and VIPs.
    LeadModerator,
    Broadcaster,
}

impl ModRole {
    /// From badge sets, plus USERSTATE's `mod=1` where there is one. `owner`: the
    /// account is the channel itself, which is the broadcaster whatever the badges say.
    pub fn of<'a>(sets: impl IntoIterator<Item = &'a str>, mod_tag: bool, owner: bool) -> Option<Self> {
        let (mut broadcaster, mut lead, mut moderator) = (owner, false, mod_tag);
        for set in sets {
            match set {
                "broadcaster" => broadcaster = true,
                "lead_moderator" => lead = true,
                "moderator" => moderator = true,
                _ => {}
            }
        }
        if broadcaster {
            Some(Self::Broadcaster)
        } else if lead {
            Some(Self::LeadModerator)
        } else if moderator {
            Some(Self::Moderator)
        } else {
            None
        }
    }

    /// A chatter's role, from their badges.
    pub fn of_badges(badges: &[Badge]) -> Option<Self> {
        Self::of(badges.iter().map(|b| b.set.as_str()), false, false)
    }
}

/// Badge sets that say someone moderates; dropped from our own badges when Twitch says
/// we don't anymore.
pub(crate) const MOD_SETS: [&str; 2] = ["moderator", "lead_moderator"];

#[cfg(test)]
mod tests {
    use super::*;

    fn badge(set: &str) -> Badge {
        Badge {
            set: set.into(),
            version: "1".into(),
            info: None,
        }
    }

    #[test]
    fn badges_and_tags() {
        assert_eq!(ModRole::of_badges(&[]), None);
        assert_eq!(ModRole::of_badges(&[badge("vip"), badge("subscriber")]), None);
        assert_eq!(
            ModRole::of_badges(&[badge("moderator")]),
            Some(ModRole::Moderator)
        );
        assert_eq!(
            ModRole::of_badges(&[badge("lead_moderator")]),
            Some(ModRole::LeadModerator)
        );
        assert_eq!(
            ModRole::of_badges(&[badge("moderator"), badge("lead_moderator")]),
            Some(ModRole::LeadModerator)
        );
        assert_eq!(
            ModRole::of_badges(&[badge("broadcaster")]),
            Some(ModRole::Broadcaster)
        );
        // `mod=1` without a badge (badges hidden), and the channel's own account.
        assert_eq!(ModRole::of([], true, false), Some(ModRole::Moderator));
        assert_eq!(ModRole::of(["vip"], false, true), Some(ModRole::Broadcaster));
    }

    #[test]
    fn roles_are_ordered() {
        assert!(ModRole::Moderator < ModRole::LeadModerator);
        assert!(ModRole::LeadModerator < ModRole::Broadcaster);
    }
}
