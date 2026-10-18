//! 7TV (1.37): emote sets, paints, badges, personal emotes (C9), live set changes (C10),
//! animated avatars (C23). Research: `research/providers.md`.

pub mod account;
pub mod api;
pub mod events;
pub mod paint;
pub mod socket;

/// R15 A 7TV id: a 26-character ULID (Crockford base32, upper case).
pub fn is_id(t: &str) -> bool {
    t.len() == 26
        && t
            .bytes()
            .all(|b| b.is_ascii_digit() || (b.is_ascii_uppercase() && !matches!(b, b'I' | b'L' | b'O' | b'U')))
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_id_is_an_upper_case_ulid() {
        assert!(super::is_id("01F6MZGCNG000255K4X1K96HXJ"));
        assert!(!super::is_id("01f6mzgcng000255k4x1k96hxj"));
        assert!(!super::is_id("01F6MZGCNG000255K4X1K96HX"));
        assert!(!super::is_id("01F6MZGCNG000255K4X1K96HXI"));
        assert!(!super::is_id("emotesv2_123"));
    }
}
