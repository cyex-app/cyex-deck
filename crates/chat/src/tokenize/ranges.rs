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
        // V110-F87 Only Giphy's own pictures: the tag also comes with history lines from
        // another service, and the window loads the address as it is.
        if let Some((start, end)) = span_of(span)
            && giphy(url)
        {
            out.push(Range {
                start,
                end,
                kind: RangeKind::Gif { id, url },
            });
        }
    }
    out
}

/// An `https` address on giphy.com or a host under it.
fn giphy(address: &str) -> bool {
    url::Url::parse(address).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str().is_some_and(|h| h == "giphy.com" || h.ends_with(".giphy.com"))
            && u.username().is_empty()
            && u.password().is_none()
    })
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
    // No wrap on a huge end (history and logs lines come from other services).
    (a <= b).then_some((a, b.checked_add(1)?))
}

/// Sorts by start and drops ranges that overlap an earlier one or run past `len`.
pub fn clean(mut ranges: Vec<Range<'_>>, len: usize) -> Vec<Range<'_>> {
    ranges.sort_by_key(|r| r.start);
    let mut end = 0;
    ranges.retain(|r| {
        let ok = r.start >= end && r.start < r.end && r.end <= len;
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
        // An end of usize::MAX would wrap to 0 and slice backwards.
        assert!(emotes("25:5-18446744073709551615").is_empty());
        assert_eq!(clean(emotes("25:3-3"), 10).len(), 1);
    }

    #[test]
    fn gif_tag() {
        let r = gifs("0-5|g1|https://media.giphy.com/c,d.gif,7-9|g2|https://giphy.com/z");
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0].kind,
            RangeKind::Gif {
                id: "g1",
                url: "https://media.giphy.com/c,d.gif"
            }
        );
        assert_eq!((r[1].start, r[1].end), (7, 10));
    }

    #[test]
    fn gifs_come_from_giphy_only() {
        // V110-F87 Anything else is dropped, so the words stay text.
        for ok in [
            "https://media.giphy.com/media/abc/giphy.gif",
            "https://media4.giphy.com/media/v1.Y2lk/abc/200.webp?cid=1&rid=200.webp",
            "https://giphy.com/x.gif",
            "HTTPS://MEDIA.GIPHY.COM/x.gif",
        ] {
            assert_eq!(gifs(&format!("0-1|g|{ok}")).len(), 1, "{ok}");
        }
        for bad in [
            "https://a.b/c.gif",
            "http://media.giphy.com/x.gif",
            "https://evilgiphy.com/x.gif",
            "https://giphy.com.evil.example/x.gif",
            "https://evil.example/media.giphy.com/x.gif",
            "https://media.giphy.com@evil.example/x.gif",
            "https://evil.example#.giphy.com",
            "https://evil.example?.giphy.com",
            "https://user:pw@media.giphy.com/x.gif",
            r"https://evil.example\@media.giphy.com/x.gif",
            r"\\media.giphy.com\x.gif",
            "//media.giphy.com/x.gif",
            "media.giphy.com/x.gif",
            "javascript:alert(1)//.giphy.com",
            "data:image/gif;base64,AAAA",
            "file:///C:/x.gif",
            "https://127.0.0.1/x.gif",
            "",
        ] {
            assert!(gifs(&format!("0-1|g|{bad}")).is_empty(), "{bad}");
        }
        // A bad one doesn't take the good one after it along.
        let r = gifs("0-1|a|https://evil.example/a.gif,3-4|b|https://media.giphy.com/b.gif");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].start, 3);
    }

    #[test]
    fn overlapping_and_out_of_bounds_are_dropped() {
        let r = clean(emotes("1:0-4,3-6,8-20/2:6-7"), 10);
        assert_eq!(r.iter().map(|r| r.start).collect::<Vec<_>>(), [0, 6]);
    }
}
