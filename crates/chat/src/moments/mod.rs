//! V108-N25 Moments: the points of a stream worth going back to, found in the chat as it
//! goes by (a spike in messages, a raid, a big gift bomb, an announcement, a burst of clip
//! links), plus ones the app reports (hype train, title or category change). Kept in memory
//! per room, newest `KEEP`. V108-N10 The same per-second counts give the chat header its
//! messages-per-second meter. Pure: the room feeds it live messages, the app reads it.
//!
//! The wall time of a moment is when it was noticed. A spike is noticed a few seconds after
//! it started, so a clip from one starts a little before it (the app adds that lead).

mod events;
mod label;
mod rate;
mod spike;

use std::collections::{HashMap, VecDeque};

use serde::Serialize;
use ts_rs::TS;

use crate::model::{ChatMessage, Flag};

/// Moments kept per room.
pub const KEEP: usize = 200;
/// Seconds the header meter averages over.
pub const METER_SECS: i64 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MomentKind {
    /// Chat got much busier.
    Spike,
    Raid,
    /// A big gift bomb.
    Gift,
    Announcement,
    /// A hype train began or levelled up.
    Hype,
    Title,
    Category,
    /// A burst of clip links.
    Clips,
}

impl MomentKind {
    /// Least time between two moments of a kind, ms: a bot repeating an announcement or a
    /// raid shown twice is one moment.
    fn gap_ms(self) -> i64 {
        match self {
            MomentKind::Spike => 0,
            MomentKind::Gift => 5_000,
            MomentKind::Hype => 0,
            MomentKind::Raid => 30_000,
            MomentKind::Announcement => 20_000,
            MomentKind::Title | MomentKind::Category => 30_000,
            MomentKind::Clips => 120_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Moment {
    /// Counts up per room, never reused.
    pub id: u32,
    /// Unix ms, when it was noticed.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// Seconds into the broadcast then; the app fills it from the stream's start time.
    #[ts(type = "number | null")]
    pub stream_secs: Option<i64>,
    pub kind: MomentKind,
    /// What it was about: the repeated emote or word, the notice text. May be empty.
    pub label: String,
    /// How strong: a spike's times-the-usual rate in tenths (30 = 3x); 0 for the rest.
    pub score: u32,
}

/// What the header meter shows.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Rate {
    /// Messages per second over the last few seconds.
    pub per_sec: f32,
    /// The busiest single second in the last minute.
    pub peak: u32,
}

#[derive(Debug, Default)]
pub struct Moments {
    buckets: rate::Buckets,
    detector: spike::Detector,
    words: label::Recent,
    list: VecDeque<Moment>,
    fresh: Vec<Moment>,
    next_id: u32,
    last: HashMap<MomentKind, i64>,
    clip_links: VecDeque<i64>,
}

impl Moments {
    /// A live message arrived (`now_ms`: when). History and gap lines don't come here.
    pub fn feed(&mut self, m: &ChatMessage, now_ms: i64) {
        if m.has(Flag::History) {
            return;
        }
        if let Some(n) = &m.notice {
            if let Some((kind, text)) = events::from_notice(n.kind, &n.text) {
                self.event(kind, &text, 0, now_ms);
            }
            return;
        }
        if m.sender.is_none() {
            return;
        }
        self.buckets.record(now_ms);
        self.words.record(now_ms, &m.body);
        if events::has_clip_link(&m.body) {
            self.clip_links.push_back(now_ms);
            while self
                .clip_links
                .front()
                .is_some_and(|t| *t < now_ms - events::CLIP_SPAN_MS)
            {
                self.clip_links.pop_front();
            }
            if self.clip_links.len() >= events::CLIP_BURST {
                self.clip_links.clear();
                self.event(MomentKind::Clips, "", 0, now_ms);
            }
        }
        if let Some(score) = self.detector.check(&self.buckets, now_ms) {
            let label = self.words.top(now_ms).unwrap_or_default();
            self.event(MomentKind::Spike, &label, score, now_ms);
        }
    }

    /// A moment from outside the chat lines (hype train, title or category change).
    /// `false`: too soon after one of its kind.
    pub fn event(&mut self, kind: MomentKind, label: &str, score: u32, now_ms: i64) -> bool {
        if self.last.get(&kind).is_some_and(|t| now_ms - t < kind.gap_ms()) {
            return false;
        }
        self.last.insert(kind, now_ms);
        self.next_id += 1;
        let m = Moment {
            id: self.next_id,
            at_ms: now_ms,
            stream_secs: None,
            kind,
            label: label.to_owned(),
            score,
        };
        self.list.push_back(m.clone());
        while self.list.len() > KEEP {
            self.list.pop_front();
        }
        self.fresh.push(m);
        true
    }

    /// Moments found since the last call.
    pub fn take_fresh(&mut self) -> Vec<Moment> {
        std::mem::take(&mut self.fresh)
    }

    /// All kept moments, oldest first.
    pub fn list(&self) -> Vec<Moment> {
        self.list.iter().cloned().collect()
    }

    pub fn rate(&self, now_ms: i64) -> Rate {
        Rate {
            per_sec: self.buckets.per_sec(now_ms, METER_SECS),
            peak: self.buckets.peak(now_ms, 60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_keep_the_newest_200_with_rising_ids() {
        let mut m = Moments::default();
        for i in 0..250 {
            assert!(m.event(MomentKind::Hype, "x", 0, i));
        }
        let l = m.list();
        assert_eq!(l.len(), KEEP);
        assert_eq!(l[0].id, 51);
        assert_eq!(l.last().unwrap().id, 250);
        assert_eq!(m.take_fresh().len(), 250);
        assert!(m.take_fresh().is_empty());
    }

    #[test]
    fn a_kind_does_not_repeat_inside_its_gap() {
        let mut m = Moments::default();
        assert!(m.event(MomentKind::Raid, "a", 0, 1_000));
        assert!(!m.event(MomentKind::Raid, "a", 0, 20_000));
        assert!(m.event(MomentKind::Raid, "b", 0, 31_001));
        // Another kind is not held back by it.
        assert!(m.event(MomentKind::Title, "t", 0, 31_002));
    }

    #[test]
    fn the_meter_reads_the_last_seconds() {
        let mut m = Moments::default();
        for _ in 0..20 {
            m.buckets.record(10_000);
        }
        let r = m.rate(10_500);
        assert!((r.per_sec - 4.0).abs() < 0.01);
        assert_eq!(r.peak, 20);
        assert_eq!(m.rate(60_000).per_sec, 0.0);
    }
}
