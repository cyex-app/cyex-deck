//! V107-N2c What a Helix refusal says, read as the GQL code it matches, so both paths
//! give the same short line ("bob isn’t banned." over "The user in the user_id query
//! parameter is not banned."). `None` when the words aren't known.

/// The GQL-style code for a Helix error message.
pub fn code(message: &str) -> Option<&'static str> {
    let m = message.to_ascii_lowercase();
    let has = |s: &str| m.contains(s);
    Some(if has("not banned") {
        "TARGET_NOT_BANNED"
    } else if has("already banned") {
        "TARGET_ALREADY_BANNED"
    } else if has("not a vip") {
        "REVOKEE_NOT_VIP"
    } else if has("already a vip") {
        "GRANTEE_ALREADY_VIP"
    } else if has("not a mod") {
        "TARGET_NOT_MOD"
    } else if has("already a mod") {
        "TARGET_ALREADY_MOD"
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twitchs_words_read_as_codes() {
        for (m, c) in [
            ("The user in the user_id query parameter is not banned.", "TARGET_NOT_BANNED"),
            ("The user specified in the user_id field is already banned.", "TARGET_ALREADY_BANNED"),
            ("The specified user is not a VIP in the broadcaster’s channel.", "REVOKEE_NOT_VIP"),
            ("The user in the user_id query parameter is already a VIP", "GRANTEE_ALREADY_VIP"),
            ("The user in the user_id query parameter is not a moderator", "TARGET_NOT_MOD"),
            ("The user in the user_id query parameter is already a moderator", "TARGET_ALREADY_MOD"),
            // Seen live 2026/10/21.
            ("User is already a mod.", "TARGET_ALREADY_MOD"),
            ("User is not a mod.", "TARGET_NOT_MOD"),
        ] {
            assert_eq!(code(m), Some(c), "{m}");
        }
        assert_eq!(code("Missing required parameter"), None);
        assert_eq!(code(""), None);
    }
}
