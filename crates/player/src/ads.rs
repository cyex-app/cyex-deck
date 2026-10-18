//! Ads stitched into the stream (1.2). A segment is an ad when its title isn't `live`,
//! or its time falls in a `stitched-ad` date range (streamlink's and vaft's rules).
//! Breaks Twitch's own player fetches on the side only leave markers in the playlist,
//! every segment still `live`: this player never asks for those ads, so they need nothing.

use crate::hls::media::{DateRange, Media, Segment, parse_time};

/// Which segments are ads, in order.
pub fn marks(media: &Media) -> Vec<bool> {
    let ranges: Vec<(i64, i64)> = media.dateranges.iter().filter_map(ad_range).collect();
    media
        .segments
        .iter()
        .map(|s| is_ad(s) || s.time.is_some_and(|t| ranges.iter().any(|&(a, b)| t >= a && t < b)))
        .collect()
}

/// An ad is playing at the live edge right now.
pub fn in_break(media: &Media) -> bool {
    marks(media).last().copied().unwrap_or(false)
}

/// h4 A stitched ad runs on past the live edge: the segments still being made may be ads.
/// No times to go by: any announced ad counts.
pub fn ahead(media: &Media) -> bool {
    let mut ranges = media.dateranges.iter().filter_map(ad_range);
    match media.segments.last().and_then(|s| s.time.map(|t| t + (s.duration * 1000.0) as i64)) {
        // A quarter second of slack: a range may end a hair after its last segment.
        Some(edge) => ranges.any(|(_, end)| end > edge + 250),
        None => ranges.next().is_some(),
    }
}

/// The stream's own segments, ads left out.
pub fn live_segments(media: &Media) -> impl Iterator<Item = &Segment> {
    media.segments.iter().zip(marks(media)).filter(|(_, ad)| !ad).map(|(s, _)| s)
}

fn is_ad(s: &Segment) -> bool {
    !s.title.starts_with("live")
}

/// `[start, end)` of a stitched ad, in ms.
fn ad_range(d: &DateRange) -> Option<(i64, i64)> {
    if d.class != "twitch-stitched-ad" && !d.id.starts_with("stitched-ad-") {
        return None;
    }
    let start = parse_time(d.attr("START-DATE")?)?;
    if let Some(end) = d.attr("END-DATE").and_then(parse_time) {
        return Some((start, end));
    }
    let seconds = ["DURATION", "PLANNED-DURATION"]
        .iter()
        .filter_map(|k| d.attr(k)?.parse::<f64>().ok())
        .find(|s| *s > 0.0);
    Some((start, seconds.map_or(i64::MAX, |s| start + (s * 1000.0) as i64)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hls::media::parse;

    /// A midroll starting at the third segment: Amazon-titled segments.
    pub const MIDROLL: &str = r#"#EXTM3U
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:200
#EXT-X-DATERANGE:ID="stitched-ad-1760011204-30",CLASS="twitch-stitched-ad",START-DATE="2026-10-09T12:00:04.000Z",DURATION=30.0,X-TV-TWITCH-AD-ROLL-TYPE="MIDROLL"
#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:00.000Z
#EXTINF:2.000,live
https://edge.example/200.ts
#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:02.000Z
#EXTINF:2.000,live
https://edge.example/201.ts
#EXT-X-DISCONTINUITY
#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:04.000Z
#EXTINF:2.002,Amazon|8675309
https://edge.example/ad-0.ts
#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:06.002Z
#EXTINF:2.002,Amazon|8675309
https://edge.example/ad-1.ts
#EXT-X-TWITCH-PREFETCH:https://edge.example/ad-2.ts
"#;

    #[test]
    fn stitched_ads_by_title_and_range() {
        let m = parse(MIDROLL);
        assert_eq!(marks(&m), [false, false, true, true]);
        assert!(in_break(&m));
        assert_eq!(live_segments(&m).count(), 2);

        // A live-titled segment inside the ad's range is still an ad.
        let sneaky = MIDROLL.replace("Amazon|8675309", "live");
        assert_eq!(marks(&parse(&sneaky)), [false, false, true, true]);
    }

    #[test]
    fn an_ad_ahead_of_the_edge() {
        // h4 Announced before its first segment: the edge is still stream.
        let before = MIDROLL.split("#EXT-X-DISCONTINUITY").next().unwrap();
        let m = parse(before);
        assert!(!in_break(&m) && ahead(&m));
        // Over: the range ends where the stream goes on.
        let after = before.replace("DURATION=30.0", "DURATION=2.0").replace("12:00:04", "12:00:00");
        assert!(!ahead(&parse(&after)));
        // Side markers aren't ads.
        assert!(!ahead(&parse(&before.replace("twitch-stitched-ad", "twitch-ad-quartile").replace("stitched-ad-", "x-"))));
    }

    #[test]
    fn markers_alone_are_no_break() {
        // Ads fetched on the side: markers, but the playlist is all stream.
        let side = "#EXTM3U\n#EXT-X-DATERANGE:ID=\"x\",CLASS=\"twitch-ad-quartile\",START-DATE=\"2026-10-09T12:00:00Z\"\n\
                    #EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:00.000Z\n#EXTINF:2.000,live\na.ts\n";
        let m = parse(side);
        assert!(!in_break(&m));
        assert_eq!(marks(&m), [false]);
    }
}
