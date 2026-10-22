//! V107-Bn `sendCheer`'s `validationError.code`s as short lines. Rare ones share a line.

/// Twitch's own "it didn't say": no answer, or a thrown error (research "Cheering").
pub const UNKNOWN: &str = "UNKNOWN";

/// What to show for a refusal code.
pub fn text(code: &str) -> &'static str {
    match code {
        UNKNOWN => "Not sure it went out. Sending it again won’t pay twice.",
        "AMOUNT_BELOW_MIN_BITS" => "That’s below this channel’s minimum cheer.",
        "EMOTE_AMOUNT_BELOW_MIN_BITS" => "A cheer emote is below this channel’s minimum.",
        "AMOUNT_BELOW_MIN_BITS_TO_PIN" | "PINNED_CHEERS_DISABLED" => "Pinned cheers aren’t available here.",
        "ANONYMOUS_CHEERMOTE_NOT_ALLOWED" | "INVALID_ANONYMOUS_MESSAGE" => "Anonymous cheers are off.",
        "AUTO_MOD_MESSAGE" | "AUTO_MOD_PENDING" => "AutoMod stopped this cheer. Reword it and try again.",
        "CHANNEL_BLOCKED_TERMS" => "That has a term this channel blocks.",
        "ZALGO_MESSAGE" => "Too many stacked characters.",
        "CHANNEL_INELIGIBLE" => "This channel can’t take cheers.",
        "INSUFFICIENT_BALANCE" => "Not enough Bits.",
        "MESSAGE_LENGTH_EXCEEDED" => "Too long for a cheer.",
        "REQUEST_THROTTLED" => "Too many cheers at once. Wait a moment.",
        "TOO_LARGE_BITS_EMOTE" => "100.000 is the largest cheer emote.",
        "TOO_LARGE_CHEER" => "That cheer is too big.",
        "USER_BANNED" => "You’re banned in this channel.",
        "USER_HAS_CONSEQUENCE" | "USER_INELIGIBLE" | "USER_SUSPENDED" => "Your account can’t cheer right now.",
        "INVALID_BITS_AMOUNT" | "INVALID_BITS_MESSAGE" | "INVALID_EVENT_ID" => {
            "Twitch didn’t take that cheer. Check it and try again."
        }
        _ => "Twitch couldn’t take the cheer. Try again later.",
    }
}

/// Twitch may or may not have spent the Bits: a retry must reuse the same id.
pub fn unsure(code: &str) -> bool {
    matches!(code, UNKNOWN | "REQUEST_THROTTLED")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [&str; 24] = [
        "AMOUNT_BELOW_MIN_BITS",
        "AMOUNT_BELOW_MIN_BITS_TO_PIN",
        "ANONYMOUS_CHEERMOTE_NOT_ALLOWED",
        "AUTO_MOD_MESSAGE",
        "AUTO_MOD_PENDING",
        "CHANNEL_BLOCKED_TERMS",
        "CHANNEL_INELIGIBLE",
        "EMOTE_AMOUNT_BELOW_MIN_BITS",
        "INSUFFICIENT_BALANCE",
        "INVALID_ANONYMOUS_MESSAGE",
        "INVALID_BITS_AMOUNT",
        "INVALID_BITS_MESSAGE",
        "INVALID_EVENT_ID",
        "MESSAGE_LENGTH_EXCEEDED",
        "PINNED_CHEERS_DISABLED",
        "REQUEST_THROTTLED",
        "TOO_LARGE_BITS_EMOTE",
        "TOO_LARGE_CHEER",
        "UNKNOWN",
        "USER_BANNED",
        "USER_HAS_CONSEQUENCE",
        "USER_INELIGIBLE",
        "USER_SUSPENDED",
        "ZALGO_MESSAGE",
    ];

    #[test]
    fn v107_bn_every_code_has_a_short_line() {
        let fallback = text("SOMETHING_NEW");
        for c in ALL {
            let t = text(c);
            assert!(t.len() < 60 && t.ends_with('.'), "{c}: {t}");
            assert_ne!(t, fallback, "{c} falls through");
        }
        assert_eq!(text("INSUFFICIENT_BALANCE"), "Not enough Bits.");
        assert!(unsure("UNKNOWN") && !unsure("INSUFFICIENT_BALANCE"));
    }
}
