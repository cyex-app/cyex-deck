//! Capability gating (1.54). Every gated tool is listed once with what it needs. The UI
//! hides a tool when the account can't use it. Anonymous features aren't listed.

use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Helix,
    Gql,
    /// Helix with the scopes, or any GQL token.
    Either,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Feature {
    // Helix
    SendChat,
    /// 1.6 Home. GQL works too.
    FollowedList,
    /// 1.40 Your Twitch emotes in the emote menu. GQL works too.
    EmoteMenu,
    /// 1.40 GQL works too.
    ChatColor,
    Bio,
    Whispers,
    Blocks,
    ModTools,
    ModView,
    StreamManager,
    MassbanHelix,
    /// P9
    Clips,
    // GQL
    Follow,
    Points,
    BadgePicker,
    DisplayName,
    OnsiteNotifications,
    ViewerPins,
    SendGifs,
    PowerUps,
    ViewerVotes,
    JoinRaids,
    Miner,
    LeadModRoles,
    MassbanGql,
}

const MOD_FEEDS: &[&str] = &[
    "moderator:read:banned_users",
    "moderator:read:blocked_terms",
    "moderator:read:chat_messages",
    "moderator:read:chat_settings",
    "moderator:read:moderators",
    "moderator:read:unban_requests",
    "moderator:read:vips",
    "moderator:read:warnings",
    "moderator:manage:automod",
];

impl Feature {
    pub const ALL: [Feature; 25] = [
        Feature::SendChat,
        Feature::FollowedList,
        Feature::EmoteMenu,
        Feature::ChatColor,
        Feature::Bio,
        Feature::Whispers,
        Feature::Blocks,
        Feature::ModTools,
        Feature::ModView,
        Feature::StreamManager,
        Feature::MassbanHelix,
        Feature::Clips,
        Feature::Follow,
        Feature::Points,
        Feature::BadgePicker,
        Feature::DisplayName,
        Feature::OnsiteNotifications,
        Feature::ViewerPins,
        Feature::SendGifs,
        Feature::PowerUps,
        Feature::ViewerVotes,
        Feature::JoinRaids,
        Feature::Miner,
        Feature::LeadModRoles,
        Feature::MassbanGql,
    ];

    pub fn tier(self) -> Tier {
        use Feature::*;
        match self {
            SendChat | FollowedList | EmoteMenu | ChatColor => Tier::Either,
            Bio | Whispers | Blocks | ModTools | ModView
            | StreamManager | MassbanHelix | Clips => Tier::Helix,
            _ => Tier::Gql,
        }
    }

    /// Helix scopes it needs. GQL features need none.
    pub fn scopes(self) -> &'static [&'static str] {
        use Feature::*;
        match self {
            SendChat => &["chat:edit"],
            FollowedList => &["user:read:follows"],
            EmoteMenu => &["user:read:emotes"],
            ChatColor => &["user:manage:chat_color"],
            Bio => &["user:edit"],
            Whispers => &["user:manage:whispers", "user:read:whispers"],
            Blocks => &["user:manage:blocked_users", "user:read:blocked_users"],
            ModTools | MassbanHelix => &["moderator:manage:banned_users", "moderator:manage:chat_messages"],
            ModView => MOD_FEEDS,
            StreamManager => &["channel:manage:broadcast"],
            Clips => &["clips:edit"],
            _ => &[],
        }
    }
}

/// Features an account can use. `helix_scopes` is `None` without a usable Helix token.
pub fn features(helix_scopes: Option<&[String]>, gql: bool) -> Vec<Feature> {
    Feature::ALL
        .into_iter()
        .filter(|f| {
            let helix =
                || helix_scopes.is_some_and(|granted| f.scopes().iter().all(|s| granted.iter().any(|g| g == s)));
            match f.tier() {
                Tier::Helix => helix(),
                Tier::Gql => gql,
                Tier::Either => gql || helix(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scopes::HELIX;

    fn all_scopes() -> Vec<String> {
        HELIX.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn helix_only_hides_gql_tools() {
        let f = features(Some(&all_scopes()), false);
        assert!(f.contains(&Feature::SendChat) && f.contains(&Feature::MassbanHelix));
        assert!(!f.contains(&Feature::Miner) && !f.contains(&Feature::MassbanGql));
    }

    #[test]
    fn gql_only_hides_helix_tools() {
        let f = features(None, true);
        assert!(f.contains(&Feature::Miner) && f.contains(&Feature::Follow));
        assert!(!f.contains(&Feature::ModTools));
        // Chat works with either token (phase 3).
        assert!(f.contains(&Feature::SendChat));
    }

    #[test]
    fn a_missing_scope_hides_only_its_tools() {
        let granted: Vec<String> = all_scopes().into_iter().filter(|s| s != "user:edit").collect();
        let f = features(Some(&granted), false);
        assert!(!f.contains(&Feature::Bio));
        assert!(f.contains(&Feature::ChatColor));
    }

    #[test]
    fn every_scope_a_feature_needs_is_requested() {
        for f in Feature::ALL {
            for s in f.scopes() {
                assert!(
                    HELIX.contains(s),
                    "{f:?} needs {s}, which sign-in doesn't ask for"
                );
            }
        }
    }

    #[test]
    fn anonymous_gets_nothing() {
        assert!(features(None, false).is_empty());
    }
}
