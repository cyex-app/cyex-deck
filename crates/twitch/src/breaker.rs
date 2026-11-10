//! V110-F96 A small circuit breaker for a service that can hang (the link resolvers): after
//! `TRIP` failures in a row (timeouts, no connection) the service is skipped for `PAUSE`, so a
//! run of link cards doesn't each wait out the timeout. Then one ask goes through to probe it:
//! a success closes the breaker, a failure pauses it again. The clock is passed in.

use std::sync::Mutex;
use std::time::{Duration, Instant};

const TRIP: u32 = 2;
pub const PAUSE: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Breaker(Mutex<State>);

#[derive(Default)]
struct State {
    /// Failures in a row.
    fails: u32,
    /// Skipped until then (set once tripped; a probe moves it on so only one goes through).
    until: Option<Instant>,
}

impl Breaker {
    /// Whether to ask now. Tripped and past its pause: yes, once (the next pause starts here,
    /// so a probe that never reports back doesn't hold the door open).
    pub fn allow(&self, now: Instant) -> bool {
        let mut s = self.0.lock().unwrap_or_else(|p| p.into_inner());
        match s.until {
            Some(until) if now < until => false,
            Some(_) => {
                s.until = Some(now + PAUSE);
                true
            }
            None => true,
        }
    }

    /// The ask got an answer (any status) or failed to connect or time out (`ok` false).
    pub fn report(&self, ok: bool, now: Instant) {
        let mut s = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if ok {
            *s = State::default();
            return;
        }
        s.fails = s.fails.saturating_add(1);
        if s.fails >= TRIP {
            s.until = Some(now + PAUSE);
        }
    }
}

/// The breakers of one HTTP client.
#[derive(Default)]
pub struct Breakers {
    pub embed: Breaker,
    pub resolver: Breaker,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v110_f96_trips_after_two_pauses_then_probes_once() {
        let b = Breaker::default();
        let t = Instant::now();
        let s = Duration::from_secs;
        assert!(b.allow(t));
        b.report(false, t);
        // One failure does not trip it.
        assert!(b.allow(t));
        b.report(false, t);
        assert!(!b.allow(t + s(1)));
        assert!(!b.allow(t + s(59)));
        // Past the pause one ask probes; the rest still skip.
        assert!(b.allow(t + s(61)));
        assert!(!b.allow(t + s(62)));
        // The probe fails: paused again from its report.
        b.report(false, t + s(63));
        assert!(!b.allow(t + s(100)));
        assert!(b.allow(t + s(124)));
        // The probe works: closed, everyone asks.
        b.report(true, t + s(125));
        assert!(b.allow(t + s(125)) && b.allow(t + s(125)));
    }

    #[test]
    fn v110_f96_a_success_resets_the_count() {
        let b = Breaker::default();
        let t = Instant::now();
        b.report(false, t);
        b.report(true, t);
        b.report(false, t);
        assert!(b.allow(t));
        // A probe that never reports back: the door opens again after another pause.
        b.report(false, t);
        assert!(b.allow(t + PAUSE));
        assert!(!b.allow(t + PAUSE + Duration::from_secs(1)));
        assert!(b.allow(t + PAUSE * 2));
    }
}
