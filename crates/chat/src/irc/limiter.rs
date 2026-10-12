//! Sliding-window rate limits for JOINs and messages. Twitch allows 20 JOINs per 10 s and
//! 20 messages per 30 s (100 as broadcaster, mod or VIP); we stay a little under, like
//! Chatterino, so clock drift never trips them.

use std::collections::VecDeque;
use std::time::Duration;

use tokio::time::Instant;

#[derive(Debug)]
pub struct Window {
    period: Duration,
    sent: VecDeque<Instant>,
}

impl Window {
    pub const JOIN_LIMIT: usize = 18;
    pub const JOIN_PERIOD: Duration = Duration::from_secs(10);
    pub const SEND_LIMIT: usize = 19;
    /// Broadcaster, moderator or VIP.
    pub const SEND_LIMIT_FAST: usize = 99;
    pub const SEND_PERIOD: Duration = Duration::from_secs(32);

    pub fn new(period: Duration) -> Self {
        Self {
            period,
            sent: VecDeque::new(),
        }
    }

    pub fn joins() -> Self {
        Self::new(Self::JOIN_PERIOD)
    }

    pub fn sends() -> Self {
        Self::new(Self::SEND_PERIOD)
    }

    fn expire(&mut self, now: Instant) {
        while self.sent.front().is_some_and(|t| now.duration_since(*t) >= self.period) {
            self.sent.pop_front();
        }
    }

    /// Takes a slot if fewer than `limit` were used in the last period.
    pub fn try_take(&mut self, limit: usize, now: Instant) -> bool {
        self.expire(now);
        if self.sent.len() < limit {
            self.sent.push_back(now);
            true
        } else {
            false
        }
    }

    /// When the next slot frees up, or `None` if one is free now.
    pub fn wait(&mut self, limit: usize, now: Instant) -> Option<Duration> {
        self.expire(now);
        if self.sent.len() < limit {
            return None;
        }
        let oldest = self.sent[self.sent.len() - limit];
        Some(self.period.saturating_sub(now.duration_since(oldest)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_limit_then_waits_for_the_oldest() {
        let t0 = Instant::now();
        let mut w = Window::joins();
        for i in 0..20 {
            assert!(w.try_take(20, t0 + Duration::from_millis(i)));
        }
        assert!(!w.try_take(20, t0 + Duration::from_secs(1)));
        assert_eq!(w.wait(20, t0 + Duration::from_secs(1)), Some(Duration::from_secs(9)));
        assert!(w.try_take(20, t0 + Duration::from_secs(10)));
        assert_eq!(w.wait(20, t0 + Duration::from_secs(10)), Some(Duration::from_millis(1)));
    }

    #[test]
    fn a_higher_limit_frees_slots() {
        let t0 = Instant::now();
        let mut w = Window::sends();
        for _ in 0..20 {
            assert!(w.try_take(20, t0));
        }
        assert!(!w.try_take(20, t0));
        assert!(w.try_take(100, t0));
    }
}
