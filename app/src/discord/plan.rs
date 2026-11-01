//! V108-N26 When to talk to Discord: it rate-limits activity updates, so changes are
//! coalesced (only the newest wanted state is sent) and spaced; a missing Discord is retried
//! quietly. Pure: the worker passes in the time.

use std::time::{Duration, Instant};

use super::activity::Watch;

/// Between two updates.
pub const SPACING: Duration = Duration::from_secs(15);
/// Between two tries to reach a Discord that isn't there.
pub const RETRY: Duration = Duration::from_secs(20);

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Nothing to do until a change comes in.
    Idle,
    /// Look again after this long.
    Wait(Duration),
    Connect,
    /// Closing the pipe clears the activity at once.
    Disconnect,
    Send(Watch),
}

#[derive(Default)]
pub struct Plan {
    wanted: Option<Watch>,
    sent: Option<Watch>,
    connected: bool,
    last_send: Option<Instant>,
    retry_at: Option<Instant>,
}

impl Plan {
    /// The newest wanted state replaces any earlier one that wasn't sent.
    pub fn want(&mut self, w: Option<Watch>) {
        self.wanted = w;
    }

    pub fn step(&self, now: Instant) -> Step {
        let Some(want) = &self.wanted else {
            return if self.connected { Step::Disconnect } else { Step::Idle };
        };
        if !self.connected {
            return match self.retry_at {
                Some(t) if t > now => Step::Wait(t - now),
                _ => Step::Connect,
            };
        }
        if self.sent.as_ref() == Some(want) {
            return Step::Idle;
        }
        match self.last_send {
            Some(t) if now < t + SPACING => Step::Wait(t + SPACING - now),
            _ => Step::Send(want.clone()),
        }
    }

    pub fn connected(&mut self) {
        self.connected = true;
        self.retry_at = None;
        self.sent = None;
        self.last_send = None;
    }

    pub fn connect_failed(&mut self, now: Instant) {
        self.retry_at = Some(now + RETRY);
    }

    /// The pipe closed on its own (Discord quit): try again after a pause.
    pub fn lost(&mut self, now: Instant) {
        self.connected = false;
        self.sent = None;
        self.retry_at = Some(now + RETRY);
    }

    pub fn disconnected(&mut self) {
        self.connected = false;
        self.sent = None;
        self.retry_at = None;
    }

    pub fn sent(&mut self, w: Watch, now: Instant) {
        self.sent = Some(w);
        self.last_send = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(login: &str) -> Watch {
        Watch { login: login.into(), name: login.into(), avatar: None }
    }

    #[test]
    fn idle_without_a_want() {
        let p = Plan::default();
        assert_eq!(p.step(Instant::now()), Step::Idle);
    }

    #[test]
    fn connects_then_sends_at_once() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        assert_eq!(p.step(t0), Step::Connect);
        p.connected();
        assert_eq!(p.step(t0), Step::Send(w("a")));
        p.sent(w("a"), t0);
        assert_eq!(p.step(t0 + Duration::from_secs(60)), Step::Idle);
    }

    #[test]
    fn changes_are_spaced_and_coalesced() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected();
        p.sent(w("a"), t0);
        p.want(Some(w("b")));
        p.want(Some(w("c")));
        assert_eq!(p.step(t0 + Duration::from_secs(5)), Step::Wait(Duration::from_secs(10)));
        assert_eq!(p.step(t0 + SPACING), Step::Send(w("c")));
    }

    #[test]
    fn switching_back_before_the_send_sends_nothing() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected();
        p.sent(w("a"), t0);
        p.want(Some(w("b")));
        p.want(Some(w("a")));
        assert_eq!(p.step(t0 + Duration::from_secs(1)), Step::Idle);
    }

    #[test]
    fn nothing_wanted_closes_at_once() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected();
        p.sent(w("a"), t0);
        p.want(None);
        assert_eq!(p.step(t0), Step::Disconnect);
        p.disconnected();
        assert_eq!(p.step(t0), Step::Idle);
        // A new start is not held back by the earlier send.
        p.want(Some(w("b")));
        assert_eq!(p.step(t0), Step::Connect);
        p.connected();
        assert_eq!(p.step(t0), Step::Send(w("b")));
    }

    #[test]
    fn a_missing_discord_is_retried_quietly() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        assert_eq!(p.step(t0), Step::Connect);
        p.connect_failed(t0);
        assert_eq!(p.step(t0 + Duration::from_secs(5)), Step::Wait(Duration::from_secs(15)));
        assert_eq!(p.step(t0 + RETRY), Step::Connect);
    }

    #[test]
    fn a_lost_pipe_reconnects_and_resends() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected();
        p.sent(w("a"), t0);
        p.lost(t0);
        assert_eq!(p.step(t0), Step::Wait(RETRY));
        p.connected();
        assert_eq!(p.step(t0 + RETRY), Step::Send(w("a")));
    }
}
