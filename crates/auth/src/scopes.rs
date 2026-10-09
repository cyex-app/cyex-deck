//! OAuth scopes. Helix sign-in asks for everything the app uses, once (1.5). Every name
//! was checked against Twitch's scope list on 2026/10/09; a misspelled one makes Twitch
//! reject the whole sign-in.

pub const HELIX: &[&str] = &[
    // Chat
    "chat:edit",              // 1.8 send over IRC
    "chat:read",              // 1.8 read over IRC
    "clips:edit",             // 1.45 /clip
    "user:manage:chat_color", // 1.40, 2.2 name color
    "user:read:emotes",       // 1.40 emote menu
    "user:write:chat",        // 1.8 send over Helix
    // User
    "user:edit",                    // 2.2 bio
    "user:manage:blocked_users",    // 1.45 /block
    "user:manage:whispers",         // 1.26 send whispers
    "user:read:blocked_users",      // F2 block list
    "user:read:follows",            // 1.6 home, 1.11
    "user:read:moderated_channels", // 1.57 role gate, 1.10 join budget
    "user:read:subscriptions",      // 1.28 own subs
    // Moderation
    "moderation:read",                   // 1.50 mod list
    "moderator:manage:announcements",    // 1.49 /announce
    "moderator:manage:automod",          // 1.56 approve/deny
    "moderator:manage:automod_settings", // 1.48
    "moderator:manage:banned_users",     // 1.49, 1.55, M1
    "moderator:manage:blocked_terms",    // 1.48
    "moderator:manage:chat_messages",    // 1.49 delete, 1.9 pins
    "moderator:manage:chat_settings",    // 1.48, 1.35 chat modes
    "moderator:manage:shield_mode",      // 1.48
    "moderator:manage:shoutouts",        // 1.49 /shoutout
    "moderator:manage:suspicious_users", // M3
    "moderator:manage:unban_requests",   // M2
    "moderator:manage:warnings",         // 1.49 /warn
    "moderator:read:automod_settings",   // 1.48
    "moderator:read:blocked_terms",      // 1.48
    "moderator:read:chat_messages",      // 1.9 pins
    "moderator:read:chat_settings",      // 1.48
    "moderator:read:chatters",           // 1.48 viewer list, C6
    "moderator:read:followers",          // 1.36, 1.48
    "moderator:read:shield_mode",        // 1.48
    "moderator:read:unban_requests",     // M2
    // Broadcaster, stream manager (1.35)
    "bits:read",
    "channel:edit:commercial",
    "channel:manage:ads",
    "channel:manage:broadcast", // title, category, X4 markers
    "channel:manage:guest_star",
    "channel:manage:moderators", // 1.50
    "channel:manage:polls",
    "channel:manage:predictions",
    "channel:manage:raids",
    "channel:manage:redemptions", // 1.48
    "channel:manage:vips",        // 1.50
    "channel:read:ads",
    "channel:read:charity",
    "channel:read:goals",
    "channel:read:guest_star",
    "channel:read:hype_train",
    "channel:read:polls",
    "channel:read:predictions",
    "channel:read:redemptions",
    "channel:read:subscriptions",
    "channel:read:vips",
    "user:read:broadcast", // X4 markers
    // EventSub only
    "moderator:read:banned_users",     // channel.moderate v2
    "moderator:read:moderators",       // channel.moderate v2
    "moderator:read:shoutouts",        // channel.shoutout.*
    "moderator:read:suspicious_users", // M3, M6
    "moderator:read:vips",             // channel.moderate v2
    "moderator:read:warnings",         // channel.moderate v2
    "user:read:chat",                  // channel.chat.*
    "user:read:whispers",              // 1.26
];

/// What the Android TV client asks for (miner sign-in, D6). GQL ignores scopes.
pub const GQL: &[&str] = &[
    "channel_read",
    "chat:edit", // 1.8 chat for GQL-only accounts
    "chat:read",
    "user_blocks_edit",
    "user_blocks_read",
    "user_follows_edit",
    "user_read",
];

/// Helix scopes the token is missing, in list order.
pub fn missing(granted: &[String]) -> Vec<String> {
    HELIX
        .iter()
        .filter(|s| !granted.iter().any(|g| g == *s))
        .map(|s| (*s).to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicates() {
        let mut sorted = HELIX.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), HELIX.len());
    }

    #[test]
    fn missing_lists_what_is_not_granted() {
        let granted: Vec<String> = HELIX.iter().skip(2).map(|s| (*s).to_owned()).collect();
        assert_eq!(missing(&granted), ["chat:edit", "chat:read"]);
        assert!(missing(&HELIX.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()).is_empty());
    }
}
