//! The live media playlist the player sees (1.2). Segments come from whichever source is
//! clean right now, the stream or a backup without ads. One numbering that only goes up,
//! a discontinuity at every switch, and each moment of the stream only once: hls.js
//! plays across the switches as if it were one stream.

use std::collections::VecDeque;
use std::fmt::Write;

use time::OffsetDateTime;
use time::macros::format_description;

use crate::hls::media::Segment;

/// Segments kept, about Twitch's own window (14 × 2 s).
const KEEP: usize = 15;

/// Where segments came from: 0 is the stream, others are backups.
pub type Source = u8;

#[derive(Debug, Default)]
pub struct Stitcher {
    window: VecDeque<Out>,
    next_seq: u64,
    /// `EXT-X-DISCONTINUITY-SEQUENCE`: switches that left the window.
    gone_switches: u64,
    last: Option<Last>,
    target: u32,
}

#[derive(Debug)]
struct Out {
    seq: u64,
    segment: Segment,
    discontinuity: bool,
}

#[derive(Debug)]
struct Last {
    source: Source,
    seq: u64,
    time: Option<i64>,
    duration: f32,
}

impl Stitcher {
    /// Adds what's new in `segments` from `source`. A switch of source is marked.
    pub fn push<'a>(&mut self, source: Source, target: u32, segments: impl IntoIterator<Item = &'a Segment>) {
        self.target = self.target.max(target);
        for s in segments {
            if !self.is_new(source, s) {
                continue;
            }
            // A switch, or a jump in time where a break was cut out: hls.js then lines the
            // next segment up again instead of shifting video and padding audio apart.
            let jump = self.last.as_ref().is_some_and(|l| {
                l.source != source
                    || matches!((l.time, s.time), (Some(lt), Some(t)) if t > lt + (l.duration * 1000.0) as i64 + 1000)
            });
            self.window.push_back(Out {
                seq: self.next_seq,
                segment: s.clone(),
                discontinuity: jump || (s.discontinuity && self.last.is_some()),
            });
            self.next_seq += 1;
            self.last = Some(Last {
                source,
                seq: s.seq,
                time: s.time,
                duration: s.duration,
            });
            while self.window.len() > KEEP {
                if self.window.pop_front().is_some_and(|o| o.discontinuity) {
                    self.gone_switches += 1;
                }
            }
        }
    }

    /// Later than the last segment added, by time when both have one.
    fn is_new(&self, source: Source, s: &Segment) -> bool {
        let Some(last) = &self.last else { return true };
        if self.window.iter().any(|o| o.segment.uri == s.uri) {
            return false;
        }
        match (last.time, s.time) {
            // Half a segment of slack: backups don't line up to the millisecond.
            (Some(lt), Some(t)) => t > lt + (last.duration * 500.0) as i64,
            _ => source == last.source && s.seq > last.seq,
        }
    }

    /// Next segments that are still being made (low latency): they're listed as normal
    /// ones, timed on from `after`, the last finished segment of the same playlist. The
    /// player's request waits until each is ready. Timed from that and not from the last
    /// one listed here: that's often last poll's prefetch, so each poll would put the
    /// stream 2 s further ahead and break it up at every segment (V103-03).
    pub fn push_prefetch(&mut self, source: Source, after: Option<&Segment>, uris: &[String]) {
        let (Some(last), Some(after)) = (&self.last, after) else { return };
        if last.source != source {
            return;
        }
        let (mut time, duration, mut seq) = (after.time, after.duration.max(1.0), after.seq);
        let map = after.map.clone();
        let next: Vec<Segment> = uris
            .iter()
            .map(|uri| {
                time = time.map(|t| t + (duration * 1000.0) as i64);
                seq += 1;
                Segment {
                    seq,
                    duration,
                    title: "live".into(),
                    uri: uri.clone(),
                    time,
                    discontinuity: false,
                    map: map.clone(),
                }
            })
            .collect();
        self.push(source, 0, &next);
    }

    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    pub fn render(&self) -> String {
        let fmp4 = self.window.iter().any(|o| o.segment.map.is_some());
        let first = self.window.front().map_or(self.next_seq, |o| o.seq);
        let mut out = String::from("#EXTM3U\n");
        let _ = write!(
            out,
            "#EXT-X-VERSION:{}\n#EXT-X-TARGETDURATION:{}\n#EXT-X-MEDIA-SEQUENCE:{first}\n\
             #EXT-X-DISCONTINUITY-SEQUENCE:{}\n",
            if fmp4 { 6 } else { 3 },
            self.target.max(1),
            self.gone_switches,
        );
        let mut map: Option<&str> = None;
        for o in &self.window {
            if o.discontinuity {
                out.push_str("#EXT-X-DISCONTINUITY\n");
            }
            let m = o.segment.map.as_deref();
            if m.is_some() && (m != map || o.discontinuity) {
                let _ = writeln!(out, "#EXT-X-MAP:URI=\"{}\"", m.unwrap_or_default());
            }
            map = m;
            if let Some(t) = o.segment.time.and_then(format_time) {
                let _ = writeln!(out, "#EXT-X-PROGRAM-DATE-TIME:{t}");
            }
            let _ = write!(out, "#EXTINF:{:.3},{}\n{}\n", o.segment.duration, o.segment.title, o.segment.uri);
        }
        out
    }
}

fn format_time(ms: i64) -> Option<String> {
    let t = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000).ok()?;
    t.format(format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
    ))
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hls::media::{self, tests::LIVE};

    /// Live segments `from..to`, 2 s each, from a source whose clock is `offset_ms` off.
    fn segs(name: &str, from: u64, to: u64, offset_ms: i64) -> Vec<Segment> {
        (from..to)
            .map(|n| Segment {
                seq: n,
                duration: 2.0,
                title: "live".into(),
                uri: format!("https://{name}/{n}.ts"),
                time: Some(1_000_000 + n as i64 * 2000 + offset_ms),
                discontinuity: false,
                map: None,
            })
            .collect()
    }

    fn uris(s: &Stitcher) -> Vec<&str> {
        s.window.iter().map(|o| o.segment.uri.as_str()).collect()
    }

    #[test]
    fn repeats_are_left_out_and_numbers_go_up() {
        let mut s = Stitcher::default();
        s.push(0, 6, &segs("a", 10, 13, 0));
        s.push(0, 6, &segs("a", 11, 15, 0));
        assert_eq!(uris(&s), ["https://a/10.ts", "https://a/11.ts", "https://a/12.ts", "https://a/13.ts", "https://a/14.ts"]);
        let text = s.render();
        assert!(text.contains("#EXT-X-MEDIA-SEQUENCE:0\n"));
        assert!(text.contains("#EXT-X-TARGETDURATION:6\n"));
        assert!(!text.contains("DISCONTINUITY\n"));
    }

    #[test]
    fn switching_sources_picks_up_where_it_left_off() {
        let mut s = Stitcher::default();
        s.push(0, 6, &segs("main", 10, 13, 0));
        // The backup runs 300 ms behind and still lists what was already played.
        s.push(1, 6, &segs("backup", 8, 15, -300));
        assert_eq!(uris(&s)[3..], ["https://backup/13.ts", "https://backup/14.ts"]);
        assert!(s.window[3].discontinuity && !s.window[4].discontinuity);
        // Back to the stream: only what's after the backup's last segment.
        s.push(0, 6, &segs("main", 12, 17, 0));
        assert_eq!(uris(&s)[5..], ["https://main/15.ts", "https://main/16.ts"]);
        assert!(s.window[5].discontinuity);
        let text = s.render();
        assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 2);
    }

    #[test]
    fn switches_leaving_the_window_are_counted() {
        let mut s = Stitcher::default();
        s.push(0, 6, &segs("main", 0, 2, 0));
        s.push(1, 6, &segs("backup", 2, 4, 0));
        s.push(0, 6, &segs("main", 4, 4 + KEEP as u64, 0));
        let text = s.render();
        assert!(text.contains("#EXT-X-DISCONTINUITY-SEQUENCE:1\n"), "{text}");
        assert!(text.contains("#EXT-X-MEDIA-SEQUENCE:4\n"));
    }

    #[test]
    fn prefetched_segments_are_listed_once() {
        let mut s = Stitcher::default();
        let m = media::parse(LIVE);
        s.push(0, m.target_duration, &m.segments);
        s.push_prefetch(0, m.segments.last(), &m.prefetch);
        assert_eq!(s.window.len(), 4);
        assert_eq!(s.window[3].segment.time, Some(m.segments[1].time.unwrap() + 4000));
        // Next poll: the prefetched ones are real now, plus one new.
        let next = segs("edge.example/v1/segment", 102, 105, 0)
            .into_iter()
            .map(|mut x| {
                x.time = Some(m.segments[1].time.unwrap() + (x.seq as i64 - 101) * 2000);
                x
            })
            .collect::<Vec<_>>();
        s.push(0, 6, &next);
        assert_eq!(s.window.len(), 5);
        assert!(s.render().contains("#EXT-X-PROGRAM-DATE-TIME:2026-10-09T12:00:08.000Z\n#EXTINF:2.000,live\nhttps://edge.example/v1/segment/104.ts"));
    }

    #[test]
    fn prefetch_listed_again_keeps_the_stream_in_step() {
        // V103-03 Each poll lists the next two as prefetch; the second was prefetch last
        // time too. The stream stays 2 s a segment, in one piece.
        let mut s = Stitcher::default();
        let all = segs("edge", 0, 20, 0);
        let uri = |n: usize| all[n].uri.clone();
        for edge in 4..12 {
            s.push(0, 6, &all[edge - 4..=edge]);
            s.push_prefetch(0, Some(&all[edge]), &[uri(edge + 1), uri(edge + 2)]);
        }
        let times: Vec<i64> = s.window.iter().filter_map(|o| o.segment.time).collect();
        assert!(times.windows(2).all(|w| w[1] - w[0] == 2000), "{times:?}");
        assert!(s.window.iter().all(|o| !o.discontinuity));
        assert_eq!(s.window.back().map(|o| o.segment.time), Some(all[13].time));
    }

    #[test]
    fn fmp4_init_segments_follow_switches() {
        let mut s = Stitcher::default();
        let mut a = segs("a", 0, 2, 0);
        for x in &mut a {
            x.map = Some("https://a/init.mp4".into());
        }
        let mut b = segs("b", 2, 3, 0);
        b[0].map = Some("https://b/init.mp4".into());
        s.push(0, 6, &a);
        s.push(1, 6, &b);
        let text = s.render();
        assert!(text.starts_with("#EXTM3U\n#EXT-X-VERSION:6\n"));
        assert_eq!(text.matches("#EXT-X-MAP:").count(), 2);
    }
}
