//! V108-N24 Replay what you watched: the segments this player was already served, kept for
//! a few minutes, so a stream without Twitch's rewind can still go back, like a TV box. Never
//! the archive VOD (V1061-03 stays: no way around Twitch's perk). Ads were cut by the stitcher,
//! so there are none in here.
//!
//! One track for video, one for audio only: they can't share a playlist. Each is served as a
//! live playlist with a date per segment (not EVENT: the cap trims its front); it grows while
//! live and ends when the stream does.
//! About 150 B a segment: 30 min is under 200 KB.

use std::collections::VecDeque;
use std::fmt::Write;
use std::sync::{Arc, Mutex};

use crate::hls::media::Segment;
use crate::stitch::format_time;

/// The setting's top (`player.replayMinutes`).
pub const MAX_MINUTES: u8 = 30;

/// Measured 2026/10/29 (signed out, lowest quality): a segment that had left Twitch's live
/// playlist still loaded, same bytes, 8 min later. A missing segment younger than this is a
/// hole; an older one says how long Twitch keeps them, and everything older goes.
pub const KNOWN_LIFETIME_MS: i64 = 8 * 60_000;

/// V108-N24 How far outside a track's range a start may ask and still be placed at its edge.
pub const START_SLACK_MS: i64 = 30_000;

/// One ring per player, shared by its live sessions (a reopen keeps what was recorded).
pub type Shared = Arc<Mutex<Replay>>;

#[derive(Debug)]
pub struct Replay {
    cap_ms: i64,
    /// Video, audio only.
    tracks: [Track; 2],
    ended: bool,
    /// Learned from a missing segment older than `KNOWN_LIFETIME_MS`.
    lifetime_ms: Option<i64>,
}

#[derive(Debug, Default)]
struct Track {
    entries: VecDeque<Entry>,
    next_seq: u64,
    /// `EXT-X-DISCONTINUITY-SEQUENCE`: marks that left the front.
    gone_marks: u64,
    target: u32,
}

#[derive(Debug)]
struct Entry {
    seq: u64,
    /// Program date time, ms since 1970.
    time: i64,
    duration: f32,
    uri: String,
    map: Option<String>,
    discontinuity: bool,
    variant: usize,
}

impl Entry {
    fn end(&self) -> i64 {
        self.time + (f64::from(self.duration) * 1000.0) as i64
    }
}

/// What a track holds: program date times in ms, `to` is the end of the newest segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub from: i64,
    pub to: i64,
}

impl Replay {
    pub fn new(minutes: u8) -> Self {
        Self {
            cap_ms: cap(minutes),
            tracks: [Track::default(), Track::default()],
            ended: false,
            lifetime_ms: None,
        }
    }

    pub fn shared(minutes: u8) -> Shared {
        Arc::new(Mutex::new(Self::new(minutes)))
    }

    /// The setting changed: a shorter one trims now.
    pub fn set_minutes(&mut self, minutes: u8) {
        self.cap_ms = cap(minutes);
        let (cap, life) = (self.cap_ms, self.lifetime_ms);
        for t in &mut self.tracks {
            t.trim(cap, life);
        }
    }

    /// What the stitcher just served for `variant` (`audio`: an audio only one), with its
    /// discontinuity marks. Repeats by time are left out; a new quality, a mark or a hole in
    /// time starts a new piece.
    pub fn record<'a>(
        &mut self,
        variant: usize,
        audio: bool,
        target: u32,
        segments: impl IntoIterator<Item = (&'a Segment, bool)>,
    ) {
        let (cap, life) = (self.cap_ms, self.lifetime_ms);
        let t = &mut self.tracks[usize::from(audio)];
        let mut added = false;
        for (s, mark) in segments {
            added |= t.push(variant, s, mark);
        }
        if added {
            t.target = t.target.max(target);
            t.trim(cap, life);
            // A stream that was over came back (a restart): it grows again.
            self.ended = false;
        }
    }

    /// The stream is over: the playlists end.
    pub fn end(&mut self) {
        self.ended = true;
    }

    pub fn ended(&self) -> bool {
        self.ended
    }

    /// V109-F72 How long a track keeps, ms: the setting, or less once Twitch was seen to drop
    /// segments sooner. The bar of a full replay is this long.
    pub fn keep_ms(&self) -> i64 {
        self.lifetime_ms.map_or(self.cap_ms, |l| l.min(self.cap_ms))
    }

    pub fn range(&self, audio: bool) -> Option<Range> {
        let e = &self.tracks[usize::from(audio)].entries;
        Some(Range {
            from: e.front()?.time,
            to: e.back()?.end(),
        })
    }

    /// V108-N24 Whether the track holds `at` (ms), give or take `START_SLACK_MS`. The other
    /// track (audio only, or back to video) may not: it was only recorded while it played.
    pub fn covers(&self, audio: bool, at: i64) -> bool {
        self.range(audio)
            .is_some_and(|r| at >= r.from - START_SLACK_MS && at <= r.to + START_SLACK_MS)
    }

    /// Seconds into the playlist for `at` (ms). Before the first segment or in a hole: the
    /// next segment's start; past the end: the last one's start.
    pub fn offset(&self, audio: bool, at: i64) -> Option<f64> {
        let entries = &self.tracks[usize::from(audio)].entries;
        let last = entries.back()?;
        let mut acc = 0.0;
        for e in entries {
            if at < e.time {
                return Some(acc);
            }
            if at < e.end() {
                return Some(acc + (at - e.time) as f64 / 1000.0);
            }
            acc += f64::from(e.duration);
        }
        Some((acc - f64::from(last.duration)).max(0.0))
    }

    /// The track's playlist, None while it's empty.
    pub fn render(&self, audio: bool) -> Option<String> {
        let t = &self.tracks[usize::from(audio)];
        let first = t.entries.front()?;
        let fmp4 = t.entries.iter().any(|e| e.map.is_some());
        let longest = t
            .entries
            .iter()
            .map(|e| e.duration.ceil() as u32)
            .max()
            .unwrap_or(1);
        // V108-N24 No EXT-X-PLAYLIST-TYPE: an EVENT playlist may never lose its front, and the
        // cap trims it. A plain live playlist that slides, then ends with ENDLIST, is valid.
        // The first entry's own mark isn't written (nothing before it), so it counts here.
        let mut out = String::from("#EXTM3U\n");
        let _ = write!(
            out,
            "#EXT-X-VERSION:{}\n#EXT-X-TARGETDURATION:{}\n\
             #EXT-X-MEDIA-SEQUENCE:{}\n#EXT-X-DISCONTINUITY-SEQUENCE:{}\n",
            if fmp4 { 6 } else { 3 },
            t.target.max(longest).max(1),
            first.seq,
            t.gone_marks + u64::from(first.discontinuity),
        );
        let mut map: Option<&str> = None;
        for (i, e) in t.entries.iter().enumerate() {
            if e.discontinuity && i > 0 {
                out.push_str("#EXT-X-DISCONTINUITY\n");
            }
            let m = e.map.as_deref();
            if m.is_some() && (m != map || e.discontinuity || i == 0) {
                let _ = writeln!(out, "#EXT-X-MAP:URI=\"{}\"", m.unwrap_or_default());
            }
            map = m;
            if let Some(d) = format_time(e.time) {
                let _ = writeln!(out, "#EXT-X-PROGRAM-DATE-TIME:{d}");
            }
            let _ = write!(out, "#EXTINF:{:.3},live\n{}\n", e.duration, e.uri);
        }
        if self.ended {
            out.push_str("#EXT-X-ENDLIST\n");
        }
        Some(out)
    }

    /// The player couldn't load `uri` (404). Older than the known lifetime: Twitch keeps
    /// segments this long, so it and everything before it go, now and from now on. Younger:
    /// a hole, the player skips it. True when something was trimmed.
    pub fn gone(&mut self, uri: &str) -> bool {
        let Some((track, age)) = self.tracks.iter().enumerate().find_map(|(i, t)| {
            let newest = t.entries.back()?.time;
            t.entries
                .iter()
                .find(|e| e.uri == uri)
                .map(|e| (i, newest - e.time))
        }) else {
            return false;
        };
        if age < KNOWN_LIFETIME_MS {
            return false;
        }
        self.lifetime_ms = Some(self.lifetime_ms.map_or(age, |l| l.min(age)));
        let t = &mut self.tracks[track];
        while let Some(e) = t.entries.pop_front() {
            if e.discontinuity {
                t.gone_marks += 1;
            }
            if e.uri == uri {
                break;
            }
        }
        let (cap, life) = (self.cap_ms, self.lifetime_ms);
        for t in &mut self.tracks {
            t.trim(cap, life);
        }
        true
    }
}

impl Track {
    fn push(&mut self, variant: usize, s: &Segment, mark: bool) -> bool {
        // Without a date it can't be placed (Twitch always sends one).
        let Some(time) = s.time else { return false };
        let last = self.entries.back();
        if let Some(l) = last
            && time <= l.time + (f64::from(l.duration) * 500.0) as i64
        {
            return false;
        }
        let discontinuity =
            last.is_some_and(|l| mark || l.variant != variant || time > l.end() + 1000 || s.discontinuity);
        self.entries.push_back(Entry {
            seq: self.next_seq,
            time,
            duration: s.duration,
            uri: s.uri.clone(),
            map: s.map.clone(),
            discontinuity,
            variant,
        });
        self.next_seq += 1;
        true
    }

    fn trim(&mut self, cap_ms: i64, lifetime_ms: Option<i64>) {
        let Some(newest) = self.entries.back().map(|e| e.time) else {
            return;
        };
        let keep = lifetime_ms.map_or(cap_ms, |l| l.min(cap_ms));
        while self.entries.front().is_some_and(|e| newest - e.time >= keep) {
            if self.entries.pop_front().is_some_and(|e| e.discontinuity) {
                self.gone_marks += 1;
            }
        }
    }
}

fn cap(minutes: u8) -> i64 {
    i64::from(minutes.min(MAX_MINUTES)) * 60_000
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_791_288_000_000; // 2026-10-06T12:00:00Z

    /// Segments `from..to`, 2 s each, at `T0 + n * 2 s`.
    fn segs(host: &str, from: u64, to: u64) -> Vec<Segment> {
        (from..to)
            .map(|n| Segment {
                seq: n,
                duration: 2.0,
                title: "live".into(),
                uri: format!("https://{host}/{n}.ts"),
                time: Some(T0 + n as i64 * 2000),
                discontinuity: false,
                map: None,
            })
            .collect()
    }

    fn rec(r: &mut Replay, variant: usize, audio: bool, s: &[Segment]) {
        r.record(variant, audio, 6, s.iter().map(|s| (s, false)));
    }

    fn uris(text: &str) -> Vec<&str> {
        text.lines().filter(|l| l.starts_with("https://")).collect()
    }

    #[test]
    fn empty_until_something_played() {
        let r = Replay::new(5);
        assert_eq!(
            (r.render(false), r.range(false), r.offset(false, T0)),
            (None, None, None)
        );
    }

    #[test]
    fn repeats_are_left_out_and_it_grows() {
        let mut r = Replay::new(5);
        rec(&mut r, 0, false, &segs("a", 0, 5));
        rec(&mut r, 0, false, &segs("a", 2, 8));
        let text = r.render(false).unwrap();
        assert_eq!(uris(&text).len(), 8);
        assert!(text.starts_with("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:0\n"));
        assert!(text.contains(
            "#EXT-X-PROGRAM-DATE-TIME:2026-10-06T12:00:00.000Z\n#EXTINF:2.000,live\nhttps://a/0.ts\n"
        ));
        assert!(!text.contains("DISCONTINUITY\n") && !text.contains("ENDLIST"));
        assert_eq!(
            r.range(false),
            Some(Range {
                from: T0,
                to: T0 + 16_000
            })
        );
    }

    #[test]
    fn the_cap_trims_the_front_and_numbers_stay() {
        let mut r = Replay::new(1);
        rec(&mut r, 0, false, &segs("a", 0, 40));
        let text = r.render(false).unwrap();
        // 60 s at 2 s a segment: 30 kept, the first numbered 10.
        assert_eq!(uris(&text).len(), 30);
        assert!(text.contains("#EXT-X-MEDIA-SEQUENCE:10\n"), "{text}");
        assert_eq!(uris(&text)[0], "https://a/10.ts");
        // Shorter setting: trimmed at once. 0 keeps nothing.
        r.set_minutes(0);
        assert_eq!(r.render(false), None);
    }

    #[test]
    fn a_quality_switch_or_a_mark_starts_a_new_piece() {
        let mut r = Replay::new(5);
        rec(&mut r, 0, false, &segs("hi", 0, 3));
        // The new quality lists what the old one already served: only what's after.
        rec(&mut r, 3, false, &segs("lo", 1, 5));
        let b = segs("bk", 5, 6);
        r.record(3, false, 6, b.iter().map(|s| (s, true)));
        let text = r.render(false).unwrap();
        assert_eq!(
            uris(&text),
            [
                "https://hi/0.ts",
                "https://hi/1.ts",
                "https://hi/2.ts",
                "https://lo/3.ts",
                "https://lo/4.ts",
                "https://bk/5.ts"
            ]
        );
        assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 2);
        assert!(text.contains("#EXT-X-DISCONTINUITY\n#EXT-X-PROGRAM-DATE-TIME:2026-10-06T12:00:06.000Z\n#EXTINF:2.000,live\nhttps://lo/3.ts"));
    }

    #[test]
    fn a_hole_in_time_is_marked_and_skipped_by_offset() {
        let mut r = Replay::new(30);
        rec(&mut r, 0, false, &segs("a", 0, 3));
        // Paused 10 min: nothing loaded in between.
        rec(&mut r, 0, false, &segs("a", 300, 302));
        let text = r.render(false).unwrap();
        assert_eq!(text.matches("#EXT-X-DISCONTINUITY\n").count(), 1);
        assert_eq!(r.offset(false, T0 + 1500), Some(1.5));
        assert_eq!(r.offset(false, T0 + 3000), Some(3.0));
        assert_eq!(
            r.offset(false, T0 + 100_000),
            Some(6.0),
            "in the hole: the next piece"
        );
        assert_eq!(r.offset(false, T0 - 5000), Some(0.0));
        assert_eq!(
            r.offset(false, T0 + 999_999),
            Some(8.0),
            "past the end: the last segment"
        );
    }

    #[test]
    fn marks_leaving_the_front_are_counted() {
        let mut r = Replay::new(1);
        rec(&mut r, 0, false, &segs("a", 0, 2));
        rec(&mut r, 1, false, &segs("b", 2, 4));
        rec(&mut r, 1, false, &segs("b", 4, 40));
        let text = r.render(false).unwrap();
        assert!(text.contains("#EXT-X-DISCONTINUITY-SEQUENCE:1\n"), "{text}");
        assert!(!text.contains("#EXT-X-DISCONTINUITY\n"));
    }

    #[test]
    fn a_marked_first_entry_counts_its_mark() {
        let mut r = Replay::new(1);
        rec(&mut r, 0, false, &segs("a", 0, 10));
        // Paused 20 s: the piece after the hole starts with a mark.
        rec(&mut r, 0, false, &segs("a", 20, 50));
        // The cap trimmed everything before the hole: the marked entry leads.
        let text = r.render(false).unwrap();
        assert_eq!(uris(&text)[0], "https://a/20.ts");
        assert!(text.contains("#EXT-X-DISCONTINUITY-SEQUENCE:1\n"), "{text}");
        assert!(!text.contains("#EXT-X-DISCONTINUITY\n"));
        // One more leaves: the count stays.
        rec(&mut r, 0, false, &segs("a", 50, 51));
        assert!(r.render(false).unwrap().contains("#EXT-X-DISCONTINUITY-SEQUENCE:1\n"));
    }

    #[test]
    fn no_playlist_type_while_it_slides_and_endlist_at_the_end() {
        let mut r = Replay::new(1);
        rec(&mut r, 0, false, &segs("a", 0, 40));
        let text = r.render(false).unwrap();
        assert!(!text.contains("PLAYLIST-TYPE"), "{text}");
        r.end();
        let text = r.render(false).unwrap();
        assert!(!text.contains("PLAYLIST-TYPE") && text.ends_with("#EXT-X-ENDLIST\n"));
    }

    #[test]
    fn covers_only_what_the_track_recorded() {
        let mut r = Replay::new(30);
        rec(&mut r, 0, false, &segs("v", 0, 300));
        rec(&mut r, 5, true, &segs("au", 250, 300));
        // 200 s back: video has it, audio only started 100 s back.
        let at = T0 + 400_000;
        assert!(r.covers(false, at));
        assert!(!r.covers(true, at));
        assert!(r.covers(true, T0 + 500_000 - START_SLACK_MS + 1000));
        assert!(!r.covers(true, T0 + 700_000));
        assert!(!Replay::new(5).covers(false, T0));
    }

    #[test]
    fn video_and_audio_are_kept_apart() {
        let mut r = Replay::new(5);
        rec(&mut r, 0, false, &segs("v", 0, 3));
        rec(&mut r, 5, true, &segs("au", 3, 5));
        assert_eq!(uris(&r.render(false).unwrap()).len(), 3);
        assert_eq!(
            uris(&r.render(true).unwrap()),
            ["https://au/3.ts", "https://au/4.ts"]
        );
    }

    #[test]
    fn stream_end_ends_the_list_and_a_restart_reopens_it() {
        let mut r = Replay::new(5);
        rec(&mut r, 0, false, &segs("a", 0, 3));
        r.end();
        assert!(
            r.render(false)
                .unwrap()
                .ends_with("https://a/2.ts\n#EXT-X-ENDLIST\n")
        );
        rec(&mut r, 0, false, &segs("a", 0, 3));
        assert!(r.ended(), "repeats don't reopen it");
        rec(&mut r, 0, false, &segs("a", 50, 51));
        assert!(!r.ended());
    }

    #[test]
    fn fmp4_maps_are_listed_at_the_start_and_on_change() {
        let mut r = Replay::new(5);
        let mut s = segs("a", 0, 3);
        for x in &mut s {
            x.map = Some("https://a/init.mp4".into());
        }
        rec(&mut r, 0, false, &s);
        let text = r.render(false).unwrap();
        assert!(text.starts_with("#EXTM3U\n#EXT-X-VERSION:6\n"));
        assert_eq!(text.matches("#EXT-X-MAP:URI=\"https://a/init.mp4\"\n").count(), 1);
    }

    #[test]
    fn a_young_missing_segment_is_a_hole() {
        let mut r = Replay::new(30);
        rec(&mut r, 0, false, &segs("a", 0, 100));
        assert!(!r.gone("https://a/50.ts"));
        assert!(!r.gone("https://a/nope.ts"));
        assert_eq!(uris(&r.render(false).unwrap()).len(), 100);
    }

    #[test]
    fn an_old_missing_segment_sets_the_lifetime() {
        let mut r = Replay::new(30);
        // 20 min of segments; the one 10 min back is gone.
        rec(&mut r, 0, false, &segs("a", 0, 600));
        assert!(r.gone("https://a/299.ts"));
        let text = r.render(false).unwrap();
        assert_eq!(uris(&text)[0], "https://a/300.ts");
        assert!(text.contains("#EXT-X-MEDIA-SEQUENCE:300\n"));
        // From now on nothing older than that stays.
        rec(&mut r, 0, false, &segs("a", 600, 700));
        let first = uris(&r.render(false).unwrap())[0].to_owned();
        assert_eq!(first, "https://a/400.ts");
    }

    #[test]
    fn keep_is_the_setting_until_a_shorter_lifetime_is_seen() {
        // V109-F72 The bar of a full replay is this long.
        let mut r = Replay::new(30);
        assert_eq!(r.keep_ms(), 30 * 60_000);
        rec(&mut r, 0, false, &segs("a", 0, 600));
        assert!(r.gone("https://a/299.ts"));
        let keep = r.keep_ms();
        assert!((KNOWN_LIFETIME_MS..30 * 60_000).contains(&keep), "{keep}");
        // What it holds stays inside it.
        let range = r.range(false).unwrap();
        assert!(range.to - range.from <= keep + 2000, "{range:?}");
        r.set_minutes(5);
        assert_eq!(r.keep_ms(), 5 * 60_000);
    }

    #[test]
    fn segments_without_a_date_are_skipped() {
        let mut r = Replay::new(5);
        let mut s = segs("a", 0, 2);
        s[0].time = None;
        rec(&mut r, 0, false, &s);
        assert_eq!(uris(&r.render(false).unwrap()), ["https://a/1.ts"]);
    }

    #[test]
    fn the_setting_is_capped() {
        let mut r = Replay::new(200);
        rec(&mut r, 0, false, &segs("a", 0, 1000));
        // 30 min = 900 segments.
        assert_eq!(uris(&r.render(false).unwrap()).len(), 900);
    }
}
