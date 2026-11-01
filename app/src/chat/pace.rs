//! V108-C15 Pacing and duplicate dodging for sends. Twitch lets a normal chatter post about one
//! message a second in a channel and refuses the same text twice in a row; instead of letting
//! those come back as "Not sent", a send waits its turn and repeats change invisibly.
//! Pure, with the time passed in, so the rules are testable.

use tokio::time::{Duration, Instant};

/// Space between two sends of one chatter in one channel (not mod, VIP or broadcaster).
pub const GAP: Duration = Duration::from_millis(1100);
/// Sends one room holds back at once; more are refused.
pub const MAX_WAITING: usize = 10;
/// Twitch rejects the same text twice within this window.
pub const DUPLICATE_MS: i64 = 30_000;
/// Wordings of one text. Three, so a single refused send in between still leaves the next one
/// different from the last one Twitch took.
const VARIANTS: u8 = 3;

/// When the next send in a room may go out.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pacer {
    next: Option<Instant>,
}

impl Pacer {
    /// The time this send goes out, or `None` when [`MAX_WAITING`] are already waiting.
    pub fn take(&mut self, now: Instant) -> Option<Instant> {
        let next = self.next.filter(|n| *n > now);
        let waiting = next.map_or(0, |n| (n - now).as_millis().div_ceil(GAP.as_millis()) as usize);
        if waiting >= MAX_WAITING {
            return None;
        }
        let slot = next.unwrap_or(now);
        self.next = Some(slot + GAP);
        Some(slot)
    }
}

/// The last text sent in a room: what the user typed, which wording went out, and when.
#[derive(Clone, Debug)]
pub struct LastSent {
    pub body: String,
    pub variant: u8,
    pub at_ms: i64,
}

impl LastSent {
    /// Which wording the next send of `body` at `at_ms` takes: plain, or the one after the
    /// last when it repeats the last text inside Twitch's duplicate window.
    pub fn next_variant(last: Option<&LastSent>, body: &str, at_ms: i64) -> u8 {
        match last {
            Some(l) if l.body == body && at_ms - l.at_ms < DUPLICATE_MS => (l.variant + 1) % VARIANTS,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last(variant: u8, at_ms: i64) -> LastSent {
        LastSent {
            body: "A".into(),
            variant,
            at_ms,
        }
    }

    #[test]
    fn repeats_cycle_through_wordings() {
        let mut l = None;
        let mut seen = vec![];
        for i in 0..6 {
            let v = LastSent::next_variant(l.as_ref(), "A", i * 1100);
            seen.push(v);
            l = Some(last(v, i * 1100));
        }
        assert_eq!(seen, [0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn a_different_or_old_text_goes_out_plain() {
        assert_eq!(LastSent::next_variant(Some(&last(1, 0)), "B", 1000), 0);
        assert_eq!(LastSent::next_variant(Some(&last(1, 0)), "A", DUPLICATE_MS), 0);
        assert_eq!(LastSent::next_variant(None, "A", 0), 0);
    }

    #[test]
    fn two_in_a_row_never_match() {
        for a in 0..VARIANTS {
            assert_ne!(a, LastSent::next_variant(Some(&last(a, 0)), "A", 1));
        }
    }

    #[test]
    fn sends_are_spaced_by_the_gap() {
        let t0 = Instant::now();
        let mut p = Pacer::default();
        assert_eq!(p.take(t0), Some(t0));
        assert_eq!(p.take(t0), Some(t0 + GAP));
        assert_eq!(p.take(t0 + Duration::from_millis(200)), Some(t0 + GAP * 2));
        // After a pause the next one goes at once.
        let later = t0 + Duration::from_secs(60);
        assert_eq!(p.take(later), Some(later));
    }

    #[test]
    fn a_full_queue_refuses_and_frees_with_time() {
        let t0 = Instant::now();
        let mut p = Pacer::default();
        for _ in 0..MAX_WAITING {
            assert!(p.take(t0).is_some());
        }
        assert!(p.take(t0).is_none());
        assert!(p.take(t0 + GAP).is_some());
    }
}
