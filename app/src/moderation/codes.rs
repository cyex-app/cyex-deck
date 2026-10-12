//! Turns Twitch's error codes (GQL enums and Helix messages) into short lines people get.

/// A GQL mod error code to friendly text. `who` is the target's name for the message.
pub fn gql(code: &str, who: &str) -> String {
    match code {
        "TARGET_IS_MOD" | "TARGET_IS_MODERATOR" => format!("{who} is a moderator."),
        "TARGET_IS_VIP" => format!("{who} is a VIP."),
        "TARGET_IS_BROADCASTER" => "That's the broadcaster.".into(),
        "TARGET_IS_STAFF" | "TARGET_IS_ADMIN" | "TARGET_IS_GLOBAL_MOD" => format!("{who} is Twitch staff."),
        "TARGET_IS_SELF" => "That's you.".into(),
        "TARGET_NOT_FOUND" | "TARGET_IS_ANONYMOUS" => format!("No user called {who}."),
        "TARGET_ALREADY_BANNED" => format!("{who} is already banned."),
        "TARGET_NOT_BANNED" => format!("{who} isn't banned."),
        "DURATION_INVALID" => "That timeout length isn't allowed.".into(),
        "FORBIDDEN" | "UNAUTHORIZED" | "NO_PERMISSIONS" => "You can't do that here.".into(),
        "TARGET_ALREADY_MOD" => format!("{who} is already a mod."),
        "TARGET_NOT_MOD" => format!("{who} isn't a mod."),
        "GRANTEE_ALREADY_VIP" => format!("{who} is already a VIP."),
        "REVOKEE_NOT_VIP" => format!("{who} isn't a VIP."),
        "MAX_VIPS_REACHED" => "No VIP slots left.".into(),
        "VIP_ACHIEVEMENT_INCOMPLETE" => "This channel can't add VIPs yet.".into(),
        "REQUEST_NOT_FOUND" => "That request is gone.".into(),
        "RATE_LIMITED" => "Too many pins too quickly. Try again in a moment.".into(),
        other => format!("Twitch refused it ({other})."),
    }
}

/// A Helix error message (already human from Twitch) trimmed for chat, by status.
pub fn helix(status: u16, message: &str) -> String {
    match status {
        401 => "This account isn't allowed to; sign in again in Settings → Accounts.".into(),
        403 => "You can't do that here.".into(),
        429 => "Twitch is rate-limiting this. Try again in a moment.".into(),
        _ if !message.is_empty() => message.chars().take(200).collect(),
        _ => "Twitch refused it.".into(),
    }
}
