//! V112-24 One set of emote commands for 7TV and cyex.tv: `/add`, `/remove`, `/rename`, `/steal`
//! (`/yoink`) and `/whoadded` go to the service the emote word names. An id or link says it
//! itself (cyex.tv's are snowflakes, all digits; 7TV's are ULIDs or old 24-hex ObjectIds); a name
//! goes to the service whose set in that chat's channel has it. While `cyex_core::CYEXTV_READY`
//! is false, cyex.tv's answer is one "coming soon" line and the rest is 0.1.10's 7TV commands.

use super::seventv_edit::split_channel;

/// The commands this routes (`builtin::run` sends them here).
pub(super) const VERBS: [&str; 6] = ["add", "remove", "rename", "steal", "yoink", "whoadded"];

/// The service an emote command goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Service {
    SevenTv,
    Cyextv,
}

/// A name both services have in that channel, once cyex.tv is ready.
pub(super) const BOTH: &str = "7TV and cyex.tv both have {name} here. Use its id or link.";

/// Snowflake lengths taken as cyex.tv ids.
const SNOWFLAKE: std::ops::RangeInclusive<usize> = 15..=20;

/// What service an id or link is of; `None` for anything else (a name, junk).
pub(super) fn classify(word: &str) -> Option<Service> {
    let lower = word.to_ascii_lowercase();
    if let Some(id) = link_id(&lower, "cyex.tv/emotes/") {
        return (!id.is_empty()).then_some(Service::Cyextv);
    }
    if let Some(id) = link_id(&lower, "7tv.app/emotes/") {
        return (!id.is_empty()).then_some(Service::SevenTv);
    }
    let digits = word.bytes().all(|b| b.is_ascii_digit());
    if cyex_emotes::seventv::is_id(&word.to_ascii_uppercase()) || is_object_id(word) {
        return Some(Service::SevenTv);
    }
    (digits && SNOWFLAKE.contains(&word.len())).then_some(Service::Cyextv)
}

/// The id part of `https://[www.|old.]<mark><id>[/?#…]` (lower-cased `word`), or `None` when it
/// isn't such a link.
fn link_id<'a>(lower: &'a str, mark: &str) -> Option<&'a str> {
    let at = lower.find(mark)?;
    let head = &lower[..at];
    let head = head.strip_prefix("https://").or_else(|| head.strip_prefix("http://")).unwrap_or(head);
    let head = head.strip_prefix("www.").or_else(|| head.strip_prefix("old.")).unwrap_or(head);
    if !head.is_empty() {
        return None;
    }
    let tail = &lower[at + mark.len()..];
    Some(&tail[..tail.find(['/', '?', '#']).unwrap_or(tail.len())])
}

/// An older 7TV id: 24 hex digits (a MongoDB ObjectId).
fn is_object_id(word: &str) -> bool {
    word.len() == 24 && word.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Where `verb args` goes. `has(service, channel, name)` says whether that channel's loaded set
/// of the service has an emote of that name (up to case). Anything that doesn't name an emote
/// (no words, a bad `#channel`, an `/add` without an id) goes to 7TV, whose usage lines answer
/// it as 0.1.10 did. `ready` is `CYEXTV_READY`.
pub(super) fn route(
    verb: &str,
    args: &str,
    here: &str,
    ready: bool,
    has: impl Fn(Service, &str, &str) -> bool,
) -> Result<Service, String> {
    let Ok((words, channel)) = split_channel(args) else {
        return Ok(Service::SevenTv);
    };
    let Some(first) = words.first() else {
        return Ok(Service::SevenTv);
    };
    if let Some(s) = classify(first) {
        return Ok(s);
    }
    if verb == "add" {
        return Ok(Service::SevenTv);
    }
    let channel = channel.as_deref().unwrap_or(here);
    match (has(Service::SevenTv, channel, first), has(Service::Cyextv, channel, first)) {
        (false, true) => Ok(Service::Cyextv),
        // Both: 7TV's while cyex.tv is coming soon.
        (true, true) if ready => Err(BOTH.replace("{name}", first)),
        _ => Ok(Service::SevenTv),
    }
}

#[cfg(test)]
mod tests {
    use super::Service::{Cyextv, SevenTv};
    use super::{classify, route};

    #[test]
    fn ids_and_links_name_their_service() {
        // cyex.tv snowflakes and links.
        assert_eq!(classify("1234567890123456789"), Some(Cyextv));
        assert_eq!(classify("123456789012345"), Some(Cyextv));
        assert_eq!(classify("12345678901234567890"), Some(Cyextv));
        assert_eq!(classify("https://cyex.tv/emotes/1234567890123456789"), Some(Cyextv));
        assert_eq!(classify("cyex.tv/emotes/1234567890123456789?x=1"), Some(Cyextv));
        assert_eq!(classify("https://www.CYEX.tv/emotes/01HELLO0000000000000000000"), Some(Cyextv));
        // 7TV ULIDs (any case), 24-hex ObjectIds and links.
        assert_eq!(classify("01HZX5V4Q9000A1B2C3D4E5F6G"), Some(SevenTv));
        assert_eq!(classify("01hzx5v4q9000a1b2c3d4e5f6g"), Some(SevenTv));
        assert_eq!(classify("60ae958e229664e8667aea38"), Some(SevenTv));
        assert_eq!(classify("https://7tv.app/emotes/01HZX5V4Q9000A1B2C3D4E5F6G"), Some(SevenTv));
        assert_eq!(classify("old.7tv.app/emotes/60ae958e229664e8667aea38"), Some(SevenTv));
        // 26 digits is a ULID, not a snowflake.
        assert_eq!(classify("01234567890123456789012345"), Some(SevenTv));
        // Junk and names.
        for w in [
            "", "KEKW", "12345", "12345678901234", "123456789012345678901", "1234567890123456789x",
            "01HZX5V4Q9000A1B2C3D4E5F6", "01HZX5V4Q9000A1B2C3D4E5FIL", "60ae958e229664e8667aea3g",
            "https://cyex.tv/emotes/", "https://evil.com/7tv.app/emotes/01HZX5V4Q9000A1B2C3D4E5F6G",
            "https://7tv.app/users/01HZX5V4Q9000A1B2C3D4E5F6G",
        ] {
            assert_eq!(classify(w), None, "{w}");
        }
    }

    fn sets<'a>(seven: &'a [&'a str], tv: &'a [&'a str]) -> impl Fn(super::Service, &str, &str) -> bool + 'a {
        move |s, ch, name| {
            ch == "here" && match s {
                SevenTv => seven.iter().any(|n| n.eq_ignore_ascii_case(name)),
                Cyextv => tv.iter().any(|n| n.eq_ignore_ascii_case(name)),
            }
        }
    }

    #[test]
    fn add_goes_by_the_id() {
        let none = sets(&[], &[]);
        assert_eq!(route("add", "1234567890123456789", "here", false, &none), Ok(Cyextv));
        assert_eq!(route("add", "https://cyex.tv/emotes/1234567890123456789 hi #other", "here", false, &none), Ok(Cyextv));
        assert_eq!(route("add", "01HZX5V4Q9000A1B2C3D4E5F6G hi", "here", false, &none), Ok(SevenTv));
        assert_eq!(route("add", "60ae958e229664e8667aea38", "here", true, &none), Ok(SevenTv));
        // Not an id: 7TV's own usage lines, as in 0.1.10.
        for args in ["", "KEKW", "#bad#", "#a #b"] {
            assert_eq!(route("add", args, "here", true, &none), Ok(SevenTv), "{args}");
        }
    }

    #[test]
    fn names_go_to_the_set_that_has_them() {
        let both = sets(&["KEKW", "Both"], &["tvOnly", "both"]);
        for verb in ["remove", "rename", "whoadded", "steal", "yoink"] {
            assert_eq!(route(verb, "KEKW", "here", true, &both), Ok(SevenTv), "{verb}");
            assert_eq!(route(verb, "tvonly", "here", true, &both), Ok(Cyextv), "{verb}");
            assert_eq!(route(verb, "nobody", "here", true, &both), Ok(SevenTv), "{verb}");
            // Both have it: 7TV's while cyex.tv is coming soon, else ambiguous.
            assert_eq!(route(verb, "Both", "here", false, &both), Ok(SevenTv), "{verb}");
            let err = route(verb, "Both", "here", true, &both).unwrap_err();
            assert!(err.contains("Both") && err.contains("id or link"), "{err}");
            // An id says it itself.
            assert_eq!(route(verb, "1234567890123456789", "here", false, &both), Ok(Cyextv), "{verb}");
        }
        // The `#channel` named is the one looked in.
        assert_eq!(route("remove", "tvOnly #here", "elsewhere", true, &both), Ok(Cyextv));
        assert_eq!(route("steal", "tvOnly #other", "here", true, &both), Ok(SevenTv));
        // With several names the first decides.
        assert_eq!(route("remove", "tvOnly KEKW", "here", true, &both), Ok(Cyextv));
    }
}
