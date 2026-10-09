//! Tag-given ranges in a message: Twitch emotes (`emotes`) and chat GIFs (`gifs`, 1.43).
//! Positions are Unicode code point indices, end inclusive.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RangeKind<'a> {
    Emote { id: &'a str },
    Gif { id: &'a str, url: &'a str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Range<'a> {
    /// Code point indices, end exclusive.
    pub start: usize,
    pub end: usize,
    pub kind: RangeKind<'a>,
}

/// `25:0-4,12-16/emotesv2_abc:6-10`
pub fn emotes(tag: &str) -> Vec<Range<'_>> {
    let mut out = Vec::new();
    for entry in tag.split('/').filter(|e| !e.is_empty()) {
        let Some((id, spans)) = entry.split_once(':') else { continue };
        for span in spans.split(',') {
            if let Some((start, end)) = span_of(span) {
                out.push(Range {
                    start,
                    end,
                    kind: RangeKind::Emote { id },
                });
            }
        }
    }
    out
}

/// `0-9|gifId|https://…`, several joined by `,`. A comma inside a URL stays with it.
pub fn gifs(tag: &str) -> Vec<Range<'_>> {
    let mut out = Vec::new();
    let mut rest = tag;
    while !rest.is_empty() {
        let next = find_item_start(rest, 1).unwrap_or(rest.len());
        let item = rest[..next].trim_end_matches(',');
        rest = &rest[next..];
        let mut parts = item.splitn(3, '|');
        let (Some(span), Some(id), Some(url)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if let Some((start, end)) = span_of(span) {
            out.push(Range {
                start,
                end,
                kind: RangeKind::Gif { id, url },
            });
        }
    }
    out
}

/// Byte index of the next `,<digits>-<digits>|` after `from`, pointing past the comma.
fn find_item_start(s: &str, from: usize) -> Option<usize> {
    s.char_indices().skip(from).find_map(|(i, c)| {
        if c != ',' {
            return None;
        }
        let after = &s[i + 1..];
        let head = after.split('|').next()?;
        span_of(head).map(|_| i + 1)
    })
}

/// `3-7` → (3, 8). Rejects reversed or malformed spans.
fn span_of(span: &str) -> Option<(usize, usize)> {
    let (a, b) = span.split_once('-')?;
    let (a, b) = (a.parse::<usize>().ok()?, b.parse::<usize>().ok()?);
    (a <= b).then_some((a, b + 1))
}

/// Sorts by start and drops ranges that overlap an earlier one or run past `len`.
pub fn clean(mut ranges: Vec<Range<'_>>, len: usize) -> Vec<Range<'_>> {
    ranges.sort_by_key(|r| r.start);
    let mut end = 0;
    ranges.retain(|r| {
        let ok = r.start >= end && r.end <= len;
        if ok {
            end = r.end;
        }
        ok
    });
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emote_tag() {
        let r = emotes("25:0-4,12-16/emotesv2_ab:6-10");
        assert_eq!(r.len(), 3);
        assert_eq!((r[1].start, r[1].end), (12, 17));
        assert_eq!(r[2].kind, RangeKind::Emote { id: "emotesv2_ab" });
        assert!(emotes("").is_empty());
        assert!(emotes("25:4-0/x").is_empty());
    }

    #[test]
    fn gif_tag() {
        let r = gifs("0-5|g1|https://a.b/c,d.gif,7-9|g2|https://x.y/z");
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0].kind,
            RangeKind::Gif {
                id: "g1",
                url: "https://a.b/c,d.gif"
            }
        );
        assert_eq!((r[1].start, r[1].end), (7, 10));
    }

    #[test]
    fn overlapping_and_out_of_bounds_are_dropped() {
        let r = clean(emotes("1:0-4,3-6,8-20/2:6-7"), 10);
        assert_eq!(r.iter().map(|r| r.start).collect::<Vec<_>>(), [0, 6]);
    }
}
