//! V110-A21 One reconnect backoff for the IRC client, EventSub and the 7TV/BTTV sockets: the
//! first wait is `first` plus a random part of `first_spread` (so clients that dropped together
//! don't come back in the same second), later waits double up to `cap` with up to a quarter
//! more at random.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

/// How long to wait before the next connection try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
    pub first: Duration,
    pub first_spread: Duration,
    pub cap: Duration,
}

impl Backoff {
    pub const fn new(first: Duration, first_spread: Duration, cap: Duration) -> Self {
        Self { first, first_spread, cap }
    }

    /// The wait after `prev` (zero: nothing failed yet), with a fresh random part.
    pub fn next(&self, prev: Duration) -> Duration {
        // V110-N1 Not the clock: clients that dropped in the same millisecond got the same wait.
        // `RandomState` keys differ per call.
        let r = (RandomState::new().build_hasher().finish() % 1000) as u32;
        self.next_with(prev, r)
    }

    /// [`Backoff::next`] with the random part given, `r` in 0..1000 (thousandths).
    pub fn next_with(&self, prev: Duration, r: u32) -> Duration {
        let r = r.min(999);
        if prev.is_zero() {
            self.first + self.first_spread * r / 1000
        } else {
            let base = prev.saturating_mul(2).min(self.cap);
            base + base / 4 * r / 1000
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn v110_a21_the_first_wait_is_spread_and_later_ones_double_to_the_cap() {
        let b = Backoff::new(S(1), S(2), S(60));
        assert_eq!(b.next_with(Duration::ZERO, 0), S(1));
        assert_eq!(b.next_with(Duration::ZERO, 999), Duration::from_millis(2998));
        assert_eq!(b.next_with(S(4), 0), S(8));
        assert!(b.next_with(S(4), 999) < S(10));
        assert_eq!(b.next_with(S(600), 0), S(60));
        assert!(b.next_with(S(600), 999) < S(75));
        assert_eq!(b.next_with(Duration::MAX, 0), S(60), "no overflow");
        assert!((1000..3000).contains(&b.next(Duration::ZERO).as_millis()));
    }

    /// V110-N1 (A1-7) Calls in the same instant don't all get the same wait.
    #[test]
    fn v110_n1_the_random_part_does_not_follow_the_clock() {
        let b = Backoff::new(S(1), S(2), S(60));
        let waits: std::collections::BTreeSet<_> = (0..50).map(|_| b.next(Duration::ZERO)).collect();
        assert!(waits.len() > 10, "{}", waits.len());
    }
}
