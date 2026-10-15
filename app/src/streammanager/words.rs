//! What the stream manager says when GQL turns an action down (D30). The codes come from
//! Twitch's 2023 schema and client code; an unknown one shows as it is.

use cyex_twitch::Error;
use cyex_twitch::gql::dashboard::ads::RETRY_AFTER;

pub fn gql(e: &Error) -> String {
    match e {
        Error::Refused(code) => refused(code),
        Error::Api { status: 401, .. } => "Sign in again in Settings → Accounts.".into(),
        e if e.is_schema_error() => "Twitch changed this on their side. It needs an app update.".into(),
        Error::Network(_) => "Couldn’t reach Twitch.".into(),
        _ => "Twitch had a problem. Try again.".into(),
    }
}

fn refused(code: &str) -> String {
    if let Some(secs) = code.strip_prefix(RETRY_AFTER).and_then(|s| s.parse::<u32>().ok()) {
        return format!("You can run the next ad in {}:{:02}.", secs / 60, secs % 60);
    }
    let line = match code {
        "FORBIDDEN" | "USER_NOT_AUTHORIZED" | "USER_UNAUTHORIZED" | "AUTH_FAILURE" => {
            "Twitch didn’t allow that."
        }
        "RATE_LIMITED" | "REQUESTS_THROTTLED" => "Too fast. Try again in a moment.",
        "STATUS_TOO_LONG" => "The title is too long.",
        "STATUS_USES_BANNED_WORDS" | "AUTOMOD_FAILED" => "Twitch blocked some words in that.",
        "LANGUAGE_NOT_VALID" => "Twitch doesn’t know that language.",
        "GAME_NAME_TOO_LONG" => "That category name is too long.",
        "BROADCASTER_NOT_LIVE" => "Markers only work while you’re live.",
        "ARCHIVES_DISABLED" => "Turn on past broadcasts to place markers.",
        "MAX_DESCRIPTION_LENGTH_EXCEEDED" => "The marker note is too long.",
        "ALREADY_RAIDING" => "A raid is already counting down.",
        "CANNOT_RAID_YOURSELF" | "CANT_RAID_YOURSELF" => "You can’t raid yourself.",
        "CANNOT_RAID_THIS_CHANNEL" | "TARGET_SETTINGS_DO_NOT_ALLOW" => {
            "That channel doesn’t take raids from you."
        }
        "TOO_MANY_VIEWERS_TO_RAID" => "Too many viewers to raid that channel.",
        "INVALID_CHANNEL" => "No such channel.",
        "NO_ACTIVE_RAID" | "RAID_DOES_NOT_EXIST" | "NO_RAID" => "No raid is running.",
        "POLL_ALREADY_ACTIVE" => "A poll is already running.",
        "CHANNEL_NOT_BITS_ENABLED" => "Bits voting needs Bits turned on.",
        "EVENT_ALREADY_ACTIVE" => "A prediction is already running.",
        "CHANNEL_POINTS_NOT_ENABLED" => "Predictions need channel points turned on.",
        "INVALID_PREDICTION_WINDOW" => "Betting can stay open 30 seconds to 30 minutes.",
        "NOT_ENOUGH_OUTCOMES" => "A prediction needs at least two outcomes.",
        "TOO_MANY_OUTCOMES" => "A prediction can have at most ten outcomes.",
        "EVENT_NOT_ACTIVE" | "EVENT_ENDED" => "That prediction already ended.",
        "NO_SUB_PRODUCT" => "Sub-only chat needs Affiliate or Partner.",
        "SNOOZE_FAILED" => "Couldn’t snooze the ad.",
        _ => return format!("Twitch turned that down ({code})."),
    };
    line.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_as_lines() {
        let r = |c: &str| gql(&Error::Refused(c.into()));
        assert_eq!(r("ALREADY_RAIDING"), "A raid is already counting down.");
        assert_eq!(r("RETRY_AFTER 95"), "You can run the next ad in 1:35.");
        assert_eq!(r("SOMETHING_NEW"), "Twitch turned that down (SOMETHING_NEW).");
        let stale = Error::Api {
            status: 200,
            message: r#"Cannot query field "startAd" on type "Mutation"."#.into(),
        };
        assert!(gql(&stale).contains("app update"));
    }
}
