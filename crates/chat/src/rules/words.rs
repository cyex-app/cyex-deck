//! V110-C57 Matched words of a "colour only the matched words" rule, moved from the message's
//! plain text onto its text fragments, where the page draws them.

use crate::model::{Fragment, WordMark};

/// `spans` are char ranges in `plain(body)`. Each becomes a mark inside the text fragment
/// it falls in (cut at the fragment's edges), counted in UTF-16 units like the page does.
/// Parts over an emote, link or other fragment are left out.
pub fn marks(body: &[Fragment], spans: &[(usize, usize)]) -> Vec<WordMark> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, f) in body.iter().enumerate() {
        let len = f.source().chars().count();
        let end = start + len;
        if let Fragment::Text { text } = f
            && let Ok(frag) = u16::try_from(i)
        {
            for &(s, e) in spans {
                let (s, e) = (s.max(start), e.min(end));
                if s >= e {
                    continue;
                }
                let utf16 = |chars: usize| text.chars().take(chars).map(char::len_utf16).sum::<usize>();
                let (from, to) = (utf16(s - start), utf16(e - start));
                if let (Ok(from), Ok(to)) = (u32::try_from(from), u32::try_from(to)) {
                    out.push(WordMark { frag, from, to });
                }
            }
        }
        start = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Fragment {
        Fragment::Text { text: s.into() }
    }

    #[test]
    fn inside_one_fragment() {
        let body = [text("hello cyex world")];
        assert_eq!(marks(&body, &[(6, 10)]), [WordMark { frag: 0, from: 6, to: 10 }]);
    }

    #[test]
    fn later_fragments_count_from_their_own_start() {
        let body = [text("one "), text("two three")];
        // "three" is chars 8..13 of the plain text, 4..9 of the second fragment.
        assert_eq!(marks(&body, &[(8, 13)]), [WordMark { frag: 1, from: 4, to: 9 }]);
    }

    #[test]
    fn astral_chars_count_as_two_units() {
        let body = [text("😀 cyex")];
        // Chars 2..6, but the emoji is two UTF-16 units.
        assert_eq!(marks(&body, &[(2, 6)]), [WordMark { frag: 0, from: 3, to: 7 }]);
    }

    #[test]
    fn a_match_over_a_non_text_part_keeps_only_the_text() {
        let body = [text("a "), Fragment::Break, text("b")];
        // Chars 0..4 cover "a ", the break and "b".
        assert_eq!(
            marks(&body, &[(0, 4)]),
            [WordMark { frag: 0, from: 0, to: 2 }, WordMark { frag: 2, from: 0, to: 1 }]
        );
    }
}
