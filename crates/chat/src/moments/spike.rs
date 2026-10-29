//! V108-N25 When a chat is suddenly much busier than it was: the last 10 s against the
//! 5 minutes before them.

use super::rate::Buckets;

/// Window that is checked, seconds.
pub const WINDOW: i64 = 10;
/// Baseline window before it, seconds.
pub const BASELINE: i64 = 290;
/// A room has to be known this long before it can spike (every start would be one).
pub const WARMUP: i64 = 60;
/// Quiet time after a spike, ms.
pub const COOLDOWN_MS: i64 = 60_000;

/// A spike's strength: times the usual rate, in tenths (30 = 3x).
pub type Score = u32;

#[derive(Debug, Default)]
pub struct Detector {
    last_ms: Option<i64>,
    /// The second last looked at: a busy chat is checked once a second.
    checked: Option<i64>,
}

/// Messages in 10 s a spike needs at least, from the room's usual rate (`baseline`, per
/// second): 10 in a small room, more in a big one, never over 60.
pub fn floor(baseline: f32) -> u32 {
    (10.0 + baseline * 2.0).round().clamp(10.0, 60.0) as u32
}

impl Detector {
    pub fn check(&mut self, b: &Buckets, now_ms: i64) -> Option<Score> {
        let sec = now_ms.div_euclid(1000);
        if self.checked == Some(sec) {
            return None;
        }
        self.checked = Some(sec);
        if b.known_secs(now_ms) < WARMUP {
            return None;
        }
        if self.last_ms.is_some_and(|t| now_ms - t < COOLDOWN_MS) {
            return None;
        }
        let recent = b.count(now_ms, WINDOW, 0);
        let base = b.count(now_ms, BASELINE, WINDOW) as f32 / BASELINE as f32;
        let rate = recent as f32 / WINDOW as f32;
        if recent < floor(base) || rate < (3.0 * base).max(base + 2.0) {
            return None;
        }
        self.last_ms = Some(now_ms);
        Some((rate / base.max(0.1) * 10.0).round() as Score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `per_sec` messages in every second from `from` up to `to`.
    fn fill(b: &mut Buckets, from: i64, to: i64, per_sec: u32) {
        for s in from..to {
            for _ in 0..per_sec {
                b.record(s * 1000);
            }
        }
    }

    #[test]
    fn a_burst_over_a_calm_room_fires() {
        let mut b = Buckets::default();
        fill(&mut b, 0, 200, 1);
        fill(&mut b, 200, 210, 6);
        let score = Detector::default().check(&b, 209_500).expect("spike");
        assert!(score >= 30);
    }

    #[test]
    fn a_young_room_never_spikes() {
        let mut b = Buckets::default();
        fill(&mut b, 0, 20, 10);
        assert!(Detector::default().check(&b, 19_500).is_none());
    }

    #[test]
    fn a_steady_busy_room_does_not() {
        let mut b = Buckets::default();
        fill(&mut b, 0, 300, 8);
        assert!(Detector::default().check(&b, 299_500).is_none());
    }

    #[test]
    fn a_trickle_is_not_a_spike() {
        let mut b = Buckets::default();
        b.record(0);
        fill(&mut b, 200, 210, 1);
        assert!(Detector::default().check(&b, 209_500).is_none());
    }

    #[test]
    fn cooldown_holds_the_next_one_back_and_then_lets_it_through() {
        let mut b = Buckets::default();
        fill(&mut b, 0, 200, 1);
        fill(&mut b, 200, 210, 6);
        let mut d = Detector::default();
        assert!(d.check(&b, 209_500).is_some());
        fill(&mut b, 210, 220, 6);
        assert!(d.check(&b, 219_500).is_none());
        fill(&mut b, 220, 280, 1);
        fill(&mut b, 280, 290, 12);
        assert!(d.check(&b, 289_500).is_some());
    }

    #[test]
    fn floor_scales_with_the_room() {
        assert_eq!(floor(0.0), 10);
        assert_eq!(floor(10.0), 30);
        assert_eq!(floor(100.0), 60);
    }

    #[test]
    fn looks_once_a_second() {
        let mut b = Buckets::default();
        fill(&mut b, 0, 210, 1);
        let mut d = Detector::default();
        assert!(d.check(&b, 209_100).is_none());
        fill(&mut b, 209, 210, 80);
        assert!(d.check(&b, 209_900).is_none());
        assert!(d.check(&b, 210_100).is_some());
    }
}
