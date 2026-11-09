//! V110-F02 The watchdog's decisions, in plain values: no windows, no clocks of its own.

use std::time::{Duration, Instant};

/// No frame for this long while shown: the window may have stopped painting.
pub const STALL: Duration = Duration::from_secs(3);
/// After a nudge, frames get this long to come back before the page reloads.
pub const RETRY: Duration = Duration::from_secs(3);
/// At most one watchdog reload per window in this time.
pub const RELOAD_GAP: Duration = Duration::from_secs(120);
/// A (re)loading page gets this long before its first ping counts as late.
pub(super) const LOAD_GRACE: Duration = Duration::from_secs(10);

/// What the watchdog does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    None,
    Nudge,
    Reload,
}

/// What a page told the watchdog.
#[derive(Clone, Copy, Debug)]
pub struct Page {
    /// Its last frame ping, or when it was last given a fresh start (shown, loaded, dragged,
    /// resumed). May lie ahead (load grace).
    pub frame: Instant,
    /// `document.visibilityState` at its last ping.
    pub visible: bool,
    /// When it last answered a script.
    pub answer: Option<Instant>,
    /// When the watchdog last nudged it.
    pub nudged: Option<Instant>,
    /// It was visible when nudged. The nudge hides and shows the webview, so the page may
    /// say "hidden" right after; a stall that began visible still ends in a reload.
    pub nudged_visible: bool,
}

impl Page {
    pub(super) fn fresh(at: Instant, visible: bool) -> Self {
        Self {
            frame: at,
            visible,
            answer: None,
            nudged: None,
            nudged_visible: false,
        }
    }
}

/// What Windows says about the window and the system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gate {
    /// Minimized, hidden or in the tray.
    pub minimized: bool,
    /// Asleep, monitor off or session locked: nothing paints anyway.
    pub suspended: bool,
    /// A window drag or resize runs.
    pub dragging: bool,
    /// The window the user works in.
    pub foreground: bool,
}

/// The page has sent no frame for `STALL` where it should have: shown, the system awake,
/// no drag, and its page visible (or hidden although its window is in front).
pub fn stalled(p: &Page, g: &Gate, now: Instant) -> bool {
    !g.minimized
        && !g.suspended
        && !g.dragging
        && (p.visible || g.foreground)
        && now.saturating_duration_since(p.frame) >= STALL
}

/// V110-F02 The watchdog's step for one page. Nothing unless it stalled and answered a
/// script since the stall began (a page that doesn't answer is hung, F01's case). Then a
/// nudge; a visible page still without frames `RETRY` after it reloads. A hidden page is
/// only nudged, once per stall.
pub fn decide(p: &Page, g: &Gate, now: Instant) -> Step {
    if !stalled(p, g, now) {
        return Step::None;
    }
    let since = p.frame.checked_add(STALL).unwrap_or(p.frame);
    if !p.answer.is_some_and(|a| a >= since) {
        return Step::None;
    }
    match p.nudged.filter(|&n| n >= p.frame) {
        None => Step::Nudge,
        Some(n) if (p.visible || p.nudged_visible) && now.saturating_duration_since(n) >= RETRY => {
            Step::Reload
        }
        Some(_) => Step::None,
    }
}

/// V110-F16 The step when the reload was refused: a nudge, but only once per `RELOAD_GAP`.
pub(super) fn skipped_reload(nudges: &mut RateLimit, at: Instant) -> Step {
    if nudges.take(at) { Step::Nudge } else { Step::None }
}

/// At most one go per `gap`.
#[derive(Clone, Copy, Debug)]
pub struct RateLimit {
    gap: Duration,
    last: Option<Instant>,
}

impl RateLimit {
    pub const fn new(gap: Duration) -> Self {
        Self { gap, last: None }
    }

    /// Whether a go is allowed now; takes it when so.
    pub fn take(&mut self, now: Instant) -> bool {
        if self
            .last
            .is_some_and(|l| now.saturating_duration_since(l) < self.gap)
        {
            return false;
        }
        self.last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn a_skipped_reload_nudges_once_per_gap() {
        let t0 = Instant::now();
        let mut l = RateLimit::new(RELOAD_GAP);
        assert_eq!(skipped_reload(&mut l, t0), Step::Nudge);
        assert_eq!(skipped_reload(&mut l, t0 + secs(3)), Step::None);
        assert_eq!(skipped_reload(&mut l, t0 + secs(119)), Step::None);
        assert_eq!(skipped_reload(&mut l, t0 + secs(120)), Step::Nudge);
    }

    fn shown() -> Gate {
        Gate {
            foreground: true,
            ..Gate::default()
        }
    }

    /// A page whose last frame was at `t0`, answering `answer` s after it.
    fn page(t0: Instant, answer: Option<u64>, nudged: Option<u64>) -> Page {
        Page {
            frame: t0,
            visible: true,
            answer: answer.map(|a| t0 + secs(a)),
            nudged: nudged.map(|n| t0 + secs(n)),
            nudged_visible: nudged.is_some(),
        }
    }

    #[test]
    fn frames_coming_means_nothing() {
        let t0 = Instant::now();
        assert_eq!(
            decide(&page(t0, Some(2), None), &shown(), t0 + secs(2)),
            Step::None
        );
        assert_eq!(
            decide(
                &page(t0, Some(1), None),
                &shown(),
                t0 + Duration::from_millis(2999)
            ),
            Step::None
        );
    }

    #[test]
    fn a_stall_with_an_answer_nudges_then_reloads() {
        let t0 = Instant::now();
        assert_eq!(
            decide(&page(t0, Some(3), None), &shown(), t0 + secs(3)),
            Step::Nudge
        );
        // Nudged at 3 s: nothing until 3 s later, then the reload.
        assert_eq!(
            decide(&page(t0, Some(5), Some(3)), &shown(), t0 + secs(5)),
            Step::None
        );
        assert_eq!(
            decide(&page(t0, Some(6), Some(3)), &shown(), t0 + secs(6)),
            Step::Reload
        );
    }

    #[test]
    fn no_answer_since_the_stall_is_not_ours() {
        let t0 = Instant::now();
        assert_eq!(decide(&page(t0, None, None), &shown(), t0 + secs(10)), Step::None);
        // An answer from before the stall doesn't count.
        assert_eq!(
            decide(&page(t0, Some(1), None), &shown(), t0 + secs(10)),
            Step::None
        );
    }

    #[test]
    fn minimized_asleep_or_dragged_waits() {
        let t0 = Instant::now();
        let p = page(t0, Some(9), None);
        let now = t0 + secs(10);
        for g in [
            Gate {
                minimized: true,
                ..shown()
            },
            Gate {
                suspended: true,
                ..shown()
            },
            Gate {
                dragging: true,
                ..shown()
            },
        ] {
            assert_eq!(decide(&p, &g, now), Step::None, "{g:?}");
        }
    }

    #[test]
    fn a_hidden_page_counts_only_in_the_front_window() {
        let t0 = Instant::now();
        let hidden = Page {
            visible: false,
            ..page(t0, Some(9), None)
        };
        assert!(!hidden.nudged_visible);
        let now = t0 + secs(10);
        // Covered by another window: Chromium hid it on purpose.
        assert_eq!(decide(&hidden, &Gate::default(), now), Step::None);
        assert_eq!(decide(&hidden, &shown(), now), Step::Nudge);
        let nudged = Page {
            nudged: Some(t0 + secs(4)),
            ..hidden
        };
        assert_eq!(decide(&nudged, &shown(), now), Step::None);
        // Visible when nudged, "hidden" after (the nudge hides and shows): still reloads.
        let flipped = Page {
            nudged_visible: true,
            ..nudged
        };
        assert_eq!(decide(&flipped, &shown(), now), Step::Reload);
    }

    #[test]
    fn a_frame_after_the_nudge_starts_a_new_stall() {
        let t0 = Instant::now();
        // Nudged at 3 s, frames back at 4 s, stalled again from 7 s.
        let p = Page {
            frame: t0 + secs(4),
            ..page(t0, Some(8), Some(3))
        };
        assert_eq!(decide(&p, &shown(), t0 + secs(8)), Step::Nudge);
    }

    #[test]
    fn load_grace_lies_ahead() {
        let t0 = Instant::now();
        let p = Page::fresh(t0 + LOAD_GRACE, true);
        assert!(!stalled(&p, &shown(), t0 + secs(12)));
        assert!(stalled(&p, &shown(), t0 + LOAD_GRACE + STALL));
    }

    #[test]
    fn reloads_at_most_every_two_minutes() {
        let t0 = Instant::now();
        let mut r = RateLimit::new(RELOAD_GAP);
        assert!(r.take(t0));
        assert!(!r.take(t0));
        assert!(!r.take(t0 + RELOAD_GAP - secs(1)));
        assert!(r.take(t0 + RELOAD_GAP));
        assert!(!r.take(t0 + RELOAD_GAP + secs(1)));
    }
}
