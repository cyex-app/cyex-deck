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

/// The last wording queued in a room and when it goes out.
#[derive(Clone, Debug)]
pub struct LastSent {
    pub sent_as: String,
    pub at_ms: i64,
}

/// Why no wording of a send can go out.
#[derive(Debug, PartialEq, Eq)]
pub enum NoWording {
    /// Even the plain one is over the limit once sent (`chars` with the command escape).
    TooLong { chars: usize },
    /// Every wording that fits repeats the last one: a text at the limit sent twice.
    Repeat,
}

/// Which wording goes out at `at_ms`: the first of the [`VARIANTS`] (`wording(0)` is the
/// user's own text) that `fits` and differs from the last one queued and, V111-30, the last one
/// Twitch took (`taken`), each only inside the duplicate window. A refused send in between
/// doesn't count: Twitch compares with what it took.
pub fn pick(
    last: Option<&LastSent>,
    taken: Option<&str>,
    at_ms: i64,
    wording: impl Fn(u8) -> String,
    fits: impl Fn(&str) -> Result<(), usize>,
) -> Result<(u8, String), NoWording> {
    let queued = last.filter(|l| at_ms - l.at_ms < DUPLICATE_MS).map(|l| l.sent_as.as_str());
    let plain = wording(0);
    if let Err(chars) = fits(&plain) {
        return Err(NoWording::TooLong { chars });
    }
    (0..VARIANTS)
        .map(|v| (v, if v == 0 { plain.clone() } else { wording(v) }))
        .find(|(_, w)| fits(w).is_ok() && Some(w.as_str()) != queued && Some(w.as_str()) != taken)
        .ok_or(NoWording::Repeat)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(v: u8) -> String {
        format!("A{}", "+".repeat(v as usize))
    }

    fn ok(_: &str) -> Result<(), usize> {
        Ok(())
    }

    fn last(sent_as: &str, at_ms: i64) -> LastSent {
        LastSent { sent_as: sent_as.into(), at_ms }
    }

    #[test]
    fn repeats_never_match_the_last_queued() {
        let mut l: Option<LastSent> = None;
        let mut seen = vec![];
        for i in 0..4 {
            let (v, w) = pick(l.as_ref(), None, i * 1100, word, ok).unwrap();
            seen.push(v);
            l = Some(last(&w, i * 1100));
        }
        assert_eq!(seen, [0, 1, 0, 1]);
    }

    /// V111-30 "A" taken, "A+" refused: the next "A" can't be plain (Twitch still has "A").
    #[test]
    fn a_refused_send_in_between_is_skipped() {
        let (v, _) = pick(Some(&last("A+", 1000)), Some("A"), 2000, word, ok).unwrap();
        assert_eq!(v, 2);
        // A different text goes out plain.
        let (v, _) = pick(Some(&last("A+", 1000)), Some("A"), 2000, |_| "B".into(), ok).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn an_old_text_goes_out_plain() {
        assert_eq!(pick(Some(&last("A", 0)), None, DUPLICATE_MS, word, ok).unwrap().0, 0);
        assert_eq!(pick(None, None, 0, word, ok).unwrap().0, 0);
    }

    #[test]
    fn wordings_over_the_limit_are_skipped() {
        let fits = |w: &str| if w.len() <= 2 { Ok(()) } else { Err(w.len()) };
        assert_eq!(pick(Some(&last("A", 0)), None, 1, word, fits).unwrap().0, 1);
        assert_eq!(pick(Some(&last("A+", 0)), Some("A"), 1, word, fits), Err(NoWording::Repeat));
        let fits = |w: &str| if w.is_empty() { Ok(()) } else { Err(w.len()) };
        assert_eq!(pick(None, None, 1, word, fits), Err(NoWording::TooLong { chars: 1 }));
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

    /// V111-30 Lines typed in a burst go out in the order typed, each its own gap after the last.
    #[test]
    fn a_burst_keeps_its_order() {
        let t0 = Instant::now();
        let mut p = Pacer::default();
        let slots: Vec<_> = (0..5).map(|i| p.take(t0 + Duration::from_millis(i * 50)).unwrap()).collect();
        assert!(slots.windows(2).all(|w| w[1] - w[0] == GAP), "{slots:?}");
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
