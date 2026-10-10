//! What a moderator's session follows for one channel. Every type's condition names the
//! channel and the moderator, so they cost nothing against Twitch's limit.

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
}
