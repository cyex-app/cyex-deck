//! V107-M7 Timeouts of one user in a row share one chat line with a counter:
//! "cyexbot timed out caressz for 5m (3 times)." A stack lasts `WINDOW_MS` from its first
//! timeout; the next one after that starts a new line. Each timeout reaches chat up to
//! twice, as IRC's CLEARCHAT and as the mod-action feed (EventSub or Hermes), so each
//! source counts on its own and the line shows the larger count. The feed's text (it names
//! the mod and the reason) wins over IRC's.

use std::collections::HashMap;

/// How long a stack takes new timeouts, from its first one.
pub const WINDOW_MS: i64 = 10_000;
/// Stacks this old are dropped.
const KEEP_MS: i64 = 60_000;

/// Where a timeout line came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Irc,
    Feed,
}

struct Stack {
    line_id: String,
    start: i64,
    last: i64,
    irc: u32,
    feed: u32,
    /// The line's text without the counter.
    text: String,
}

impl Stack {
    fn count(&self) -> u32 {
        self.irc.max(self.feed)
    }

    fn of(&mut self, src: Source) -> &mut u32 {
        match src {
            Source::Irc => &mut self.irc,
            Source::Feed => &mut self.feed,
        }
    }

    /// Whether a timeout from `src` at `now` belongs here: inside the window, or the late
    /// echo of one already counted from the other source.
    fn takes(&self, src: Source, now: i64) -> bool {
        let behind = match src {
            Source::Irc => self.irc < self.feed,
            Source::Feed => self.feed < self.irc,
        };
        now - self.start < WINDOW_MS || (behind && now - self.last < WINDOW_MS)
    }
}

/// What to show for one timeout.
#[derive(Debug, PartialEq, Eq)]
pub struct Hit {
    /// The line's id: the stack's line, or the new one.
    pub id: String,
    /// `true`: update line `id` in place; `false`: it's a new line.
    pub update: bool,
    pub text: String,
}

/// Open stacks per user id, for one room.
#[derive(Default)]
pub struct Stacks(HashMap<String, Stack>);

impl Stacks {
    /// A timeout of `user` from `src`, shown as `text`. `new_id` is the line's id if this
    /// starts a stack.
    pub fn hit(&mut self, user: &str, src: Source, text: &str, now: i64, new_id: &str) -> Hit {
        self.0.retain(|_, s| now - s.last < KEEP_MS);
        if let Some(s) = self.0.get_mut(user).filter(|s| s.takes(src, now)) {
            *s.of(src) += 1;
            s.last = now;
            // IRC's plain line doesn't replace the feed's.
            if src == Source::Feed || s.feed == 0 {
                s.text = text.to_owned();
            }
            return Hit { id: s.line_id.clone(), update: true, text: counted(&s.text, s.count()) };
        }
        let mut s = Stack { line_id: new_id.to_owned(), start: now, last: now, irc: 0, feed: 0, text: text.to_owned() };
        *s.of(src) = 1;
        self.0.insert(user.to_owned(), s);
        Hit { id: new_id.to_owned(), update: false, text: text.to_owned() }
    }

    /// The stack's line is gone (scrolled out or hidden): the next timeout starts over.
    pub fn forget(&mut self, user: &str) {
        self.0.remove(user);
    }
}

/// "x was timed out for 5m." with " (3 times)" before the closing period.
fn counted(text: &str, n: u32) -> String {
    if n < 2 {
        return text.to_owned();
    }
    match text.strip_suffix('.') {
        Some(head) => format!("{head} ({n} times)."),
        None => format!("{text} ({n} times)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IRC: &str = "caressz was timed out for 1m.";
    const FEED: &str = "cyexbot timed out caressz for 1m.";

    #[test]
    fn v107_m7_own_timeout_and_its_echoes_count_once() {
        let mut s = Stacks::default();
        let a = s.hit("9", Source::Irc, IRC, 0, "l1");
        assert_eq!(a, Hit { id: "l1".into(), update: false, text: IRC.into() });
        // The feed's copy of the same timeout: same line, its text, still once.
        let b = s.hit("9", Source::Feed, FEED, 300, "l2");
        assert_eq!(b, Hit { id: "l1".into(), update: true, text: FEED.into() });
    }

    #[test]
    fn v107_m7_repeats_count_and_the_last_length_shows() {
        let mut s = Stacks::default();
        s.hit("9", Source::Feed, FEED, 0, "l1");
        s.hit("9", Source::Irc, IRC, 100, "x");
        let h = s.hit("9", Source::Irc, "caressz was timed out for 30s.", 2_000, "x");
        assert_eq!(h.text, "cyexbot timed out caressz for 1m (2 times).", "IRC keeps the feed's text");
        let h = s.hit("9", Source::Feed, "cyexbot timed out caressz for 30s.", 2_100, "x");
        assert_eq!(h.text, "cyexbot timed out caressz for 30s (2 times).");
        let h = s.hit("9", Source::Feed, "cyexbot timed out caressz for 10m: spam", 3_000, "x");
        assert_eq!((h.id.as_str(), h.update, h.text.as_str()), ("l1", true, "cyexbot timed out caressz for 10m: spam (3 times)"));
    }

    #[test]
    fn v107_m7_a_new_stack_every_ten_seconds() {
        let mut s = Stacks::default();
        s.hit("9", Source::Irc, IRC, 0, "l1");
        assert!(s.hit("9", Source::Irc, IRC, WINDOW_MS - 1, "l2").update);
        let h = s.hit("9", Source::Irc, IRC, WINDOW_MS, "l3");
        assert_eq!(h, Hit { id: "l3".into(), update: false, text: IRC.into() }, "counts from the first one");
        // Another user has their own stack.
        assert!(!s.hit("8", Source::Irc, IRC, WINDOW_MS + 1, "l4").update);
    }

    #[test]
    fn v107_m7_a_late_echo_joins_its_stack() {
        let mut s = Stacks::default();
        s.hit("9", Source::Irc, IRC, 0, "l1");
        s.hit("9", Source::Irc, IRC, 9_500, "x");
        // The feed's two copies come late: they belong to l1, not a new line.
        assert_eq!(s.hit("9", Source::Feed, FEED, 10_200, "x").id, "l1");
        let h = s.hit("9", Source::Feed, FEED, 10_400, "x");
        assert_eq!((h.id.as_str(), h.text.as_str()), ("l1", "cyexbot timed out caressz for 1m (2 times)."));
        // Caught up: the next one is a new stack.
        assert!(!s.hit("9", Source::Feed, FEED, 10_600, "l5").update);
    }

    #[test]
    fn v107_m7_room_updates_one_line_in_place() {
        use crate::irc::Line;
        use crate::model::RoomEvent;
        use crate::tokenize::NoEmotes;
        let mut r = super::super::Room::new("chan", None);
        let clear = |secs: u32, ts: i64| {
            format!("@ban-duration={secs};target-user-id=9;tmi-sent-ts={ts} :tmi.twitch.tv CLEARCHAT #chan :caressz")
        };
        let ev = r.handle_line(&Line::parse(&clear(60, 1_000)).unwrap(), 1_000, &NoEmotes);
        let RoomEvent::Message { message: first } = &ev[1] else { panic!("{ev:?}") };
        // The feed's copy of it, then a 30 s one from IRC and the feed.
        assert!(matches!(r.timeout_line("9", "cyexbot timed out caressz for 1m.", 1_200), RoomEvent::Update { .. }));
        r.handle_line(&Line::parse(&clear(30, 3_000)).unwrap(), 3_000, &NoEmotes);
        let RoomEvent::Update { message } = r.timeout_line("9", "cyexbot timed out caressz for 30s.", 3_100) else { panic!() };
        assert_eq!(message.id, first.id);
        assert_eq!(message.notice.as_ref().unwrap().text, "cyexbot timed out caressz for 30s (2 times).");
        let lines = r.snapshot().messages.iter().filter(|m| m.notice.is_some()).count();
        assert_eq!(lines, 1);
        // Ten seconds after the first: a new line.
        let ev = r.handle_line(&Line::parse(&clear(600, 11_000)).unwrap(), 11_000, &NoEmotes);
        assert!(matches!(&ev[1], RoomEvent::Message { message } if message.id != first.id));
    }

    #[test]
    fn v107_m7_forget_starts_over() {
        let mut s = Stacks::default();
        s.hit("9", Source::Irc, IRC, 0, "l1");
        s.forget("9");
        assert!(!s.hit("9", Source::Irc, IRC, 10, "l2").update);
        assert_eq!(counted("a", 2), "a (2 times)");
        assert_eq!(counted("a.", 1), "a.");
    }
}
