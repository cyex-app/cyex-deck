//! 1.48, 1.55 Pacing for mass actions over Helix: steady requests while the token's
//! points bucket has room, a wait for the bucket to refill when it runs low, and the
//! reset time after a 429. A few points stay free so other mod actions still work.

use cyex_twitch::RateLimit;

/// Points left alone for the user's own clicks meanwhile.
const RESERVE: u32 = 20;
/// At most 10 requests a second, even with a full bucket.
const MIN_GAP_MS: i64 = 100;
/// After a 429 without a reset time.
const BLIND_WAIT_MS: i64 = 5_000;

#[derive(Debug, Default)]
pub struct Pacer {
    last: Option<RateLimit>,
}

impl Pacer {
    /// How long to wait before the next request, given the last answer's limit.
    pub fn after_ok(&mut self, limit: Option<RateLimit>, now_ms: i64) -> i64 {
        if limit.is_some() {
            self.last = limit;
        }
        match self.last {
            Some(l) if l.remaining <= RESERVE => until_reset(l, now_ms),
            _ => MIN_GAP_MS,
        }
    }

    /// Twitch said 429: wait for the bucket to refill.
    pub fn after_limited(&mut self, limit: Option<RateLimit>, now_ms: i64) -> i64 {
        if limit.is_some() {
            self.last = limit;
        }
        match self.last {
            Some(l) => until_reset(l, now_ms).max(1_000),
            None => BLIND_WAIT_MS,
        }
    }
}

fn until_reset(l: RateLimit, now_ms: i64) -> i64 {
    (l.reset * 1000 - now_ms + 500).clamp(MIN_GAP_MS, 61_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(remaining: u32, reset: i64) -> Option<RateLimit> {
        Some(RateLimit {
            limit: 800,
            remaining,
            reset,
        })
    }

    #[test]
    fn steady_while_there_is_room() {
        let mut p = Pacer::default();
        assert_eq!(p.after_ok(limit(700, 100), 50_000), MIN_GAP_MS);
        // No headers this time: the last known bucket still counts.
        assert_eq!(p.after_ok(None, 50_100), MIN_GAP_MS);
    }

    #[test]
    fn waits_for_the_refill_when_low_or_limited() {
        let mut p = Pacer::default();
        assert_eq!(p.after_ok(limit(10, 60), 50_000), 10_500);
        assert_eq!(p.after_limited(limit(0, 60), 59_800), 1_000);
        let mut blind = Pacer::default();
        assert_eq!(blind.after_limited(None, 0), BLIND_WAIT_MS);
    }
}
