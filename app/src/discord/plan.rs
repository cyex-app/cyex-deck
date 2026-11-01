//! V108-N26 When to talk to Discord: it rate-limits activity updates, so changes are
//! coalesced (only the newest wanted state is sent) and spaced, across reconnects too; a
//! missing Discord is retried quietly. Nothing to show clears the activity and keeps the
//! pipe for a while, so toggling doesn't reconnect each time. Pure: the worker passes in the time.

use std::time::{Duration, Instant};

use super::activity::Watch;

/// Between two updates.
pub const SPACING: Duration = Duration::from_secs(15);
/// Between two tries to reach a Discord that isn't there.
pub const RETRY: Duration = Duration::from_secs(20);
/// How long a pipe showing nothing stays open.
pub const IDLE: Duration = Duration::from_secs(60);

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Nothing to do until a change comes in.
    Idle,
    /// Look again after this long.
    Wait(Duration),
    Connect,
    /// Closing the pipe clears the activity at once.
    Disconnect,
    /// An activity, or `None` to clear it.
    Send(Option<Watch>),
}

#[derive(Default)]
pub struct Plan {
    wanted: Option<Watch>,
    /// What Discord shows over this pipe (a new pipe shows nothing).
    shown: Option<Watch>,
    connected: bool,
    /// Kept across reconnects: a new pipe doesn't reset Discord's limit.
    last_send: Option<Instant>,
    retry_at: Option<Instant>,
    /// Since when the pipe has shown nothing.
    blank_since: Option<Instant>,
}

impl Plan {
    /// The newest wanted state replaces any earlier one that wasn't sent.
    pub fn want(&mut self, w: Option<Watch>) {
        self.wanted = w;
    }

    pub fn step(&self, now: Instant) -> Step {
        if !self.connected {
            if self.wanted.is_none() {
                return Step::Idle;
            }
            return match self.retry_at {
                Some(t) if t > now => Step::Wait(t - now),
                _ => Step::Connect,
            };
        }
        if self.shown == self.wanted {
            if self.wanted.is_some() {
                return Step::Idle;
            }
            let close_at = self.blank_since.unwrap_or(now) + IDLE;
            return if now >= close_at { Step::Disconnect } else { Step::Wait(close_at - now) };
        }
        match self.last_send.map(|t| t + SPACING).filter(|&t| t > now) {
            None => Step::Send(self.wanted.clone()),
            // A clear that would have to wait: closing the pipe clears at once.
            Some(_) if self.wanted.is_none() => Step::Disconnect,
            Some(t) => Step::Wait(t - now),
        }
    }

    pub fn connected(&mut self, now: Instant) {
        self.connected = true;
        self.retry_at = None;
        self.shown = None;
        self.blank_since = Some(now);
    }

    pub fn connect_failed(&mut self, now: Instant) {
        self.retry_at = Some(now + RETRY);
    }

    /// The pipe closed on its own (Discord quit) or stopped answering: try again after a pause.
    pub fn lost(&mut self, now: Instant) {
        self.connected = false;
        self.shown = None;
        self.retry_at = Some(now + RETRY);
    }

    pub fn disconnected(&mut self) {
        self.connected = false;
        self.shown = None;
        self.retry_at = None;
    }

    pub fn sent(&mut self, w: Option<Watch>, now: Instant) {
        self.blank_since = if w.is_none() { Some(now) } else { None };
        self.shown = w;
        self.last_send = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(login: &str) -> Watch {
        Watch { login: login.into(), name: login.into(), avatar: None }
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
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
        p.connected(t0);
        assert_eq!(p.step(t0), Step::Send(Some(w("a"))));
        p.sent(Some(w("a")), t0);
        assert_eq!(p.step(t0 + secs(60)), Step::Idle);
    }

    #[test]
    fn changes_are_spaced_and_coalesced() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected(t0);
        p.sent(Some(w("a")), t0);
        p.want(Some(w("b")));
        p.want(Some(w("c")));
        assert_eq!(p.step(t0 + secs(5)), Step::Wait(secs(10)));
        assert_eq!(p.step(t0 + SPACING), Step::Send(Some(w("c"))));
    }

    #[test]
    fn switching_back_before_the_send_sends_nothing() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected(t0);
        p.sent(Some(w("a")), t0);
        p.want(Some(w("b")));
        p.want(Some(w("a")));
        assert_eq!(p.step(t0 + secs(1)), Step::Idle);
    }

    #[test]
    fn nothing_wanted_clears_and_keeps_the_pipe_a_while() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected(t0);
        p.sent(Some(w("a")), t0);
        let t1 = t0 + SPACING;
        p.want(None);
        assert_eq!(p.step(t1), Step::Send(None));
        p.sent(None, t1);
        assert_eq!(p.step(t1 + secs(10)), Step::Wait(IDLE - secs(10)));
        // Back on within the minute: same pipe, spaced from the clear.
        p.want(Some(w("a")));
        assert_eq!(p.step(t1 + secs(10)), Step::Wait(secs(5)));
        assert_eq!(p.step(t1 + SPACING), Step::Send(Some(w("a"))));
        p.sent(Some(w("a")), t1 + SPACING);
        p.want(None);
        let t2 = t1 + SPACING + SPACING;
        assert_eq!(p.step(t2), Step::Send(None));
        p.sent(None, t2);
        assert_eq!(p.step(t2 + IDLE), Step::Disconnect);
        p.disconnected();
        assert_eq!(p.step(t2 + IDLE), Step::Idle);
    }

    #[test]
    fn a_clear_inside_the_spacing_closes_and_the_next_start_is_spaced() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected(t0);
        p.sent(Some(w("a")), t0);
        p.want(None);
        assert_eq!(p.step(t0 + secs(2)), Step::Disconnect);
        p.disconnected();
        p.want(Some(w("b")));
        assert_eq!(p.step(t0 + secs(3)), Step::Connect);
        p.connected(t0 + secs(3));
        assert_eq!(p.step(t0 + secs(3)), Step::Wait(secs(12)));
        assert_eq!(p.step(t0 + SPACING), Step::Send(Some(w("b"))));
    }

    #[test]
    fn a_missing_discord_is_retried_quietly() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        assert_eq!(p.step(t0), Step::Connect);
        p.connect_failed(t0);
        assert_eq!(p.step(t0 + secs(5)), Step::Wait(secs(15)));
        assert_eq!(p.step(t0 + RETRY), Step::Connect);
    }

    #[test]
    fn a_lost_pipe_reconnects_and_resends() {
        let t0 = Instant::now();
        let mut p = Plan::default();
        p.want(Some(w("a")));
        p.connected(t0);
        p.sent(Some(w("a")), t0);
        p.lost(t0);
        assert_eq!(p.step(t0), Step::Wait(RETRY));
        p.connected(t0 + RETRY);
        assert_eq!(p.step(t0 + RETRY), Step::Send(Some(w("a"))));
    }
}
