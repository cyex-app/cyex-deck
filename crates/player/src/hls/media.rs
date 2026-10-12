//! Media playlists: the segments of one quality. Live ones also carry Twitch's
//! `EXT-X-TWITCH-PREFETCH` (segments still being made, for low latency) and the
//! `EXT-X-DATERANGE` lines that mark ads.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{attr, attrs};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Media {
    pub target_duration: u32,
    /// `EXT-X-MEDIA-SEQUENCE`: the first segment's number.
    pub sequence: u64,
    pub segments: Vec<Segment>,
    /// Next segments' addresses, in order.
    pub prefetch: Vec<String>,
    pub dateranges: Vec<DateRange>,
    /// `EXT-X-ENDLIST`: a VOD, or a stream that ended.
    pub ended: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub seq: u64,
    pub duration: f32,
    /// `EXTINF` title. Twitch's stream segments say `live`.
    pub title: String,
    pub uri: String,
    /// `EXT-X-PROGRAM-DATE-TIME` in ms since 1970, or counted on from the last one.
    pub time: Option<i64>,
    /// `EXT-X-DISCONTINUITY` before it.
    pub discontinuity: bool,
    /// `EXT-X-MAP` in effect: the init segment of fMP4 streams.
    pub map: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DateRange {
    pub id: String,
    pub class: String,
    /// The whole attribute list, for Twitch's `X-TV-TWITCH-*` attributes.
    pub attrs: String,
}

impl DateRange {
    pub fn attr(&self, key: &str) -> Option<&str> {
        attr(&self.attrs, key)
    }
}

pub fn parse(text: &str) -> Media {
    let mut m = Media::default();
    let mut duration = 0.0;
    let mut title = String::new();
    let mut time = None;
    let mut discontinuity = false;
    let mut map: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if let Some(v) = line.strip_prefix("#EXTINF:") {
            let (d, t) = v.split_once(',').unwrap_or((v, ""));
            duration = d.trim().parse().unwrap_or(0.0);
            title = t.trim().to_owned();
        } else if let Some(v) = line.strip_prefix("#EXT-X-PROGRAM-DATE-TIME:") {
            time = parse_time(v);
        } else if line == "#EXT-X-DISCONTINUITY" {
            discontinuity = true;
        } else if let Some(v) = line.strip_prefix("#EXT-X-MAP:") {
            map = attr(v, "URI").map(str::to_owned);
        } else if let Some(v) = line.strip_prefix("#EXT-X-TWITCH-PREFETCH:") {
            m.prefetch.push(v.to_owned());
        } else if let Some(v) = line.strip_prefix("#EXT-X-DATERANGE:") {
            let mut d = DateRange {
                attrs: v.to_owned(),
                ..DateRange::default()
            };
            for (key, value) in attrs(v) {
                match key {
                    "ID" => d.id = value.to_owned(),
                    "CLASS" => d.class = value.to_owned(),
                    _ => {}
                }
            }
            m.dateranges.push(d);
        } else if let Some(v) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            m.target_duration = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            m.sequence = v.trim().parse().unwrap_or(0);
        } else if line == "#EXT-X-ENDLIST" {
            m.ended = true;
        } else if !line.starts_with('#') {
            let time = time.take().or_else(|| {
                let last = m.segments.last()?;
                Some(last.time? + (last.duration * 1000.0) as i64)
            });
            m.segments.push(Segment {
                seq: m.sequence + m.segments.len() as u64,
                duration,
                title: std::mem::take(&mut title),
                uri: line.to_owned(),
                time,
                discontinuity: std::mem::take(&mut discontinuity),
                map: map.clone(),
            });
        }
    }
    m
}

/// RFC 3339 → ms since 1970.
pub(crate) fn parse_time(s: &str) -> Option<i64> {
    let t = OffsetDateTime::parse(s.trim(), &Rfc3339).ok()?;
    i64::try_from(t.unix_timestamp_nanos() / 1_000_000).ok()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A live playlist as Twitch sends it, two segments and two prefetched.
    pub const LIVE: &str = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:100
#EXT-X-TWITCH-LIVE-SEQUENCE:100
#EXT-X-DATERANGE:ID="playlist-creation-1",CLASS="timestamp",START-DATE="2026-10-09T12:00:00.000Z",END-ON-NEXT=YES,X-SERVER-TIME="1760011200.00"
#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:00.000Z
#EXTINF:2.000,live
https://edge.example/v1/segment/100.ts
#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:02.000Z
#EXTINF:2.000,live
https://edge.example/v1/segment/101.ts
#EXT-X-TWITCH-PREFETCH:https://edge.example/v1/segment/102.ts
#EXT-X-TWITCH-PREFETCH:https://edge.example/v1/segment/103.ts
"#;

    #[test]
    fn live_playlist() {
        let m = parse(LIVE);
        assert_eq!((m.target_duration, m.sequence, m.ended), (6, 100, false));
        assert_eq!(m.segments.len(), 2);
        let s = &m.segments[1];
        assert_eq!((s.seq, s.duration, s.title.as_str()), (101, 2.0, "live"));
        assert_eq!(s.uri, "https://edge.example/v1/segment/101.ts");
        assert_eq!(s.time, Some(1_791_547_202_000));
        assert_eq!(m.prefetch.len(), 2);
        assert_eq!(m.dateranges[0].class, "timestamp");
        assert_eq!(m.dateranges[0].attr("X-SERVER-TIME"), Some("1760011200.00"));
    }

    #[test]
    fn times_count_on_and_maps_carry_over() {
        let m = parse(
            "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:00.500+00:00\n\
             #EXTINF:2.5,live\na.mp4\n#EXT-X-DISCONTINUITY\n#EXTINF:2,\nb.mp4\n#EXT-X-ENDLIST\n",
        );
        assert_eq!(m.segments[1].time, Some(m.segments[0].time.unwrap() + 2500));
        assert!(m.segments[1].discontinuity && !m.segments[0].discontinuity);
        assert_eq!(m.segments[1].map.as_deref(), Some("init.mp4"));
        assert_eq!(m.segments[1].title, "");
        assert!(m.ended);
    }
}
