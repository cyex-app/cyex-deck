//! V108-N25 Moments the room already shows as lines: raids, big gift bombs, announcements,
//! and a burst of clip links. (Hype trains, title and category changes come from outside
//! and go in through `Moments::event`.)

use super::MomentKind;
use crate::model::{Fragment, LinkTarget, NoticeKind};

/// Gifts in one bomb that count as a moment.
pub const BIG_GIFT: u32 = 10;
/// Clip links in `CLIP_SPAN_MS` that count as a moment.
pub const CLIP_BURST: usize = 5;
pub const CLIP_SPAN_MS: i64 = 20_000;
/// Longest label kept, in characters.
const LABEL_MAX: usize = 80;

/// What a system line is a moment for, with its label. `text` is the line as shown.
pub fn from_notice(kind: NoticeKind, text: &str) -> Option<(MomentKind, String)> {
    let text = text.trim();
    match kind {
        NoticeKind::Raid if !text.to_lowercase().contains("unraid") => {
            Some((MomentKind::Raid, shorten(text)))
        }
        NoticeKind::Gift => (gift_count(text)? >= BIG_GIFT).then(|| (MomentKind::Gift, shorten(text))),
        NoticeKind::Announcement if !text.is_empty() => Some((MomentKind::Announcement, shorten(text))),
        _ => None,
    }
}

/// Whether a message has a Twitch clip link.
pub fn has_clip_link(body: &[Fragment]) -> bool {
    body.iter().any(|f| {
        matches!(
            f,
            Fragment::Link {
                target: Some(LinkTarget::Clip { .. }),
                ..
            }
        )
    })
}

/// V108-N25 The size of a gift bomb: the number right after "gifting" ("Foo is gifting 20
/// Tier 1 Subs"), so a number in a name (xqc123) doesn't count. The line is Twitch's
/// `system-msg`, which carries `msg-param-mass-gift-count` as that number.
fn gift_count(s: &str) -> Option<u32> {
    let lower = s.to_lowercase();
    lower.match_indices("gifting ").find_map(|(i, w)| {
        let digits: String = lower[i + w.len()..].chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

fn shorten(s: &str) -> String {
    if s.chars().count() <= LABEL_MAX {
        return s.to_owned();
    }
    let cut: String = s.chars().take(LABEL_MAX - 1).collect();
    format!("{}\u{2026}", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raids_and_announcements_are_moments() {
        let (k, l) = from_notice(NoticeKind::Raid, "42 raiders from Foo have joined!").unwrap();
        assert_eq!(k, MomentKind::Raid);
        assert!(l.contains("raiders"));
        assert_eq!(
            from_notice(NoticeKind::Announcement, "Giveaway at 8").unwrap().0,
            MomentKind::Announcement
        );
        assert!(from_notice(NoticeKind::Announcement, "  ").is_none());
    }

    #[test]
    fn only_big_gift_bombs_count() {
        assert!(
            from_notice(
                NoticeKind::Gift,
                "Foo is gifting 20 Tier 1 Subs to Bar's community!"
            )
            .is_some()
        );
        assert!(
            from_notice(
                NoticeKind::Gift,
                "Foo is gifting 5 Tier 1 Subs to Bar's community!"
            )
            .is_none()
        );
        assert!(from_notice(NoticeKind::Gift, "Foo gifted a Tier 1 sub to Baz").is_none());
        assert!(from_notice(NoticeKind::Gift, "Foo gifted a sub, 12 months in total").is_none());
        // A number in a name isn't the count.
        assert!(
            from_notice(
                NoticeKind::Gift,
                "xqc123 is gifting 2 Tier 1 Subs to Bar's community!"
            )
            .is_none()
        );
        assert!(
            from_notice(
                NoticeKind::Gift,
                "gifting99 is gifting 50 Tier 1 Subs to Bar's community!"
            )
            .is_some()
        );
    }

    #[test]
    fn other_kinds_and_unraid_are_not() {
        assert!(from_notice(NoticeKind::Sub, "Foo subscribed").is_none());
        assert!(from_notice(NoticeKind::Moderation, "Foo was banned").is_none());
        assert!(from_notice(NoticeKind::Raid, "The raid was cancelled (unraid)").is_none());
    }

    #[test]
    fn labels_are_cut_on_characters() {
        let long = "\u{00E9}".repeat(200);
        let (_, l) = from_notice(NoticeKind::Announcement, &long).unwrap();
        assert_eq!(l.chars().count(), LABEL_MAX);
        assert!(l.ends_with('\u{2026}'));
    }

    #[test]
    fn clip_links_are_found() {
        let clip = Fragment::Link {
            text: "t".into(),
            url: "https://clips.twitch.tv/x".into(),
            host: "clips.twitch.tv".into(),
            target: Some(LinkTarget::Clip { slug: "x".into() }),
            embed: true,
        };
        let other = Fragment::Link {
            text: "t".into(),
            url: "https://example.com".into(),
            host: "example.com".into(),
            target: None,
            embed: false,
        };
        assert!(has_clip_link(&[Fragment::Text { text: "a".into() }, clip]));
        assert!(!has_clip_link(&[other]));
    }
}
