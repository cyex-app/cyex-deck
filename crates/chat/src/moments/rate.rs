//! V108-N25, V108-N10 Messages per second: one bucket per wall-clock second, kept for the
//! baseline window (five minutes) and a little more. Feeds both the chat header's meter
//! and the spike check.

use std::collections::VecDeque;

/// Seconds of history the buckets keep.
pub const KEEP_SECS: i64 = 330;

#[derive(Debug, Default)]
pub struct Buckets {
    /// (second, messages), oldest first, seconds strictly rising.
    slots: VecDeque<(i64, u32)>,
    /// The first second anything was counted: how long we have known this room.
    first: Option<i64>,
}

impl Buckets {
    pub fn record(&mut self, now_ms: i64) {
        let sec = now_ms.div_euclid(1000);
        self.first.get_or_insert(sec);
        match self.slots.back_mut() {
            // Same second, or the clock stepped back: count into the newest bucket.
            Some((s, n)) if *s >= sec => *n += 1,
            _ => self.slots.push_back((sec, 1)),
        }
        while self.slots.front().is_some_and(|(s, _)| *s < sec - KEEP_SECS) {
            self.slots.pop_front();
        }
    }

    /// Messages in the `secs` seconds ending at the one containing `now_ms`, with the `skip`
    /// newest seconds left out.
    pub fn count(&self, now_ms: i64, secs: i64, skip: i64) -> u32 {
        let end = now_ms.div_euclid(1000) - skip;
        let start = end - secs + 1;
        self.slots
            .iter()
            .filter(|(s, _)| (start..=end).contains(s))
            .map(|(_, n)| *n)
            .sum()
    }

    /// Messages per second over the last `secs` seconds.
    pub fn per_sec(&self, now_ms: i64, secs: i64) -> f32 {
        self.count(now_ms, secs, 0) as f32 / secs.max(1) as f32
    }

    /// The busiest single second in the last `secs`.
    pub fn peak(&self, now_ms: i64, secs: i64) -> u32 {
        let end = now_ms.div_euclid(1000);
        self.slots
            .iter()
            .filter(|(s, _)| (end - secs + 1..=end).contains(s))
            .map(|(_, n)| *n)
            .max()
            .unwrap_or(0)
    }

    /// Seconds since the first message was counted.
    pub fn known_secs(&self, now_ms: i64) -> i64 {
        self.first.map_or(0, |f| now_ms.div_euclid(1000) - f)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_per_second_and_windows() {
        let mut b = Buckets::default();
        for _ in 0..10 {
            b.record(5_000);
        }
        b.record(6_100);
        assert_eq!(b.count(6_500, 1, 0), 1);
        assert_eq!(b.count(6_500, 2, 0), 11);
        assert_eq!(b.count(6_500, 1, 1), 10);
        assert_eq!(b.peak(6_500, 10), 10);
        assert!((b.per_sec(6_500, 2) - 5.5).abs() < 0.01);
    }

    #[test]
    fn old_seconds_fall_out() {
        let mut b = Buckets::default();
        b.record(0);
        b.record((KEEP_SECS + 10) * 1000);
        assert_eq!(b.len(), 1);
        assert_eq!(b.count((KEEP_SECS + 10) * 1000, 400, 0), 1);
    }

    #[test]
    fn empty_is_zero() {
        let b = Buckets::default();
        assert_eq!(b.per_sec(1_000, 5), 0.0);
        assert_eq!(b.peak(1_000, 5), 0);
        assert_eq!(b.known_secs(1_000), 0);
    }

    #[test]
    fn a_clock_step_back_does_not_reorder() {
        let mut b = Buckets::default();
        b.record(10_000);
        b.record(4_000);
        assert_eq!(b.len(), 1);
    }
}
