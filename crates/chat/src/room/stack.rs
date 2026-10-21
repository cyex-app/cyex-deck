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
/// The same feed line for a user again this soon is a second delivery of it: the channel
/// is open under two mod accounts and each one's feed brings it.
const DUP_MS: i64 = 400;
/// Stacks kept per user: the open one and older ones still waiting for late echoes.
const MAX_STACKS: usize = 4;

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

    /// `src` counted fewer than the other source: its next one may be a late echo.
    fn behind(&self, src: Source) -> bool {
        match src {
            Source::Irc => self.irc < self.feed,
            Source::Feed => self.feed < self.irc,
        }
    }

    /// The late echo of one already counted from the other source.
    fn echo(&self, src: Source, now: i64) -> bool {
        self.behind(src) && now - self.last < WINDOW_MS
    }

    /// Still inside its window.
    fn open(&self, now: i64) -> bool {
        now - self.start < WINDOW_MS
    }
}

/// One user's stacks, oldest first, and the feed's last line for them.
#[derive(Default)]
struct User {
    stacks: Vec<Stack>,
    feed: Option<(String, i64)>,
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
pub struct Stacks(HashMap<String, User>);

impl Stacks {
    /// A timeout of `user` from `src`, shown as `text`, received at `now`: local time, one
    /// clock for both sources (IRC's `tmi-sent-ts` is the server's). `new_id` is the line's
    /// id if this starts a stack; `alive` says whether a stack's line is still in chat.
    /// `None`: a second delivery of the feed line just counted, to be dropped.
    pub fn hit(
        &mut self,
        user: &str,
        src: Source,
        text: &str,
        now: i64,
        new_id: &str,
        alive: impl Fn(&str) -> bool,
    ) -> Option<Hit> {
        self.0.retain(|_, u| {
            u.stacks.retain(|s| now - s.last < KEEP_MS);
            !u.stacks.is_empty()
        });
        let u = self.0.entry(user.to_owned()).or_default();
        if src == Source::Feed {
            if u.feed.as_ref().is_some_and(|(t, at)| t == text && (now - at).abs() < DUP_MS) {
                return None;
            }
            u.feed = Some((text.to_owned(), now));
        }
        // A stack whose line is gone (scrolled out or hidden) takes nothing more.
        u.stacks.retain(|s| alive(&s.line_id));
        // Echoes go to the oldest stack still behind, anything else to the open one.
        let at = u.stacks.iter().position(|s| s.echo(src, now)).or_else(|| {
            let last = u.stacks.len().checked_sub(1)?;
            u.stacks[last].open(now).then_some(last)
        });
        if let Some(i) = at {
            let s = &mut u.stacks[i];
            *s.of(src) += 1;
            s.last = now;
            // IRC's plain line doesn't replace the feed's.
            if src == Source::Feed || s.feed == 0 {
                s.text = text.to_owned();
            }
            return Some(Hit { id: s.line_id.clone(), update: true, text: counted(&s.text, s.count()) });
        }
        // Older stacks stay only while they wait for echoes.
        u.stacks.retain(|s| s.irc != s.feed);
        if u.stacks.len() >= MAX_STACKS {
            u.stacks.remove(0);
        }
        let mut s = Stack { line_id: new_id.to_owned(), start: now, last: now, irc: 0, feed: 0, text: text.to_owned() };
        *s.of(src) = 1;
        u.stacks.push(s);
        Some(Hit { id: new_id.to_owned(), update: false, text: text.to_owned() })
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
    use crate::irc::Line;
    use crate::model::RoomEvent;
    use crate::tokenize::NoEmotes;

    const IRC: &str = "caressz was timed out for 1m.";
    const FEED: &str = "cyexbot timed out caressz for 1m.";

    /// A hit with every line still in chat.
    fn hit(s: &mut Stacks, user: &str, src: Source, text: &str, now: i64, id: &str) -> Hit {
        s.hit(user, src, text, now, id, |_| true).expect("not a duplicate")
    }

    #[test]
    fn v107_m7_own_timeout_and_its_echoes_count_once() {
        let mut s = Stacks::default();
        let a = hit(&mut s, "9", Source::Irc, IRC, 0, "l1");
        assert_eq!(a, Hit { id: "l1".into(), update: false, text: IRC.into() });
        // The feed's copy of the same timeout: same line, its text, still once.
        let b = hit(&mut s, "9", Source::Feed, FEED, 300, "l2");
        assert_eq!(b, Hit { id: "l1".into(), update: true, text: FEED.into() });
    }

    #[test]
    fn v107_m7_repeats_count_and_the_last_length_shows() {
        let mut s = Stacks::default();
        hit(&mut s, "9", Source::Feed, FEED, 0, "l1");
        hit(&mut s, "9", Source::Irc, IRC, 100, "x");
        let h = hit(&mut s, "9", Source::Irc, "caressz was timed out for 30s.", 2_000, "x");
        assert_eq!(h.text, "cyexbot timed out caressz for 1m (2 times).", "IRC keeps the feed's text");
        let h = hit(&mut s, "9", Source::Feed, "cyexbot timed out caressz for 30s.", 2_100, "x");
        assert_eq!(h.text, "cyexbot timed out caressz for 30s (2 times).");
        let h = hit(&mut s, "9", Source::Feed, "cyexbot timed out caressz for 10m: spam", 3_000, "x");
        assert_eq!((h.id.as_str(), h.update, h.text.as_str()), ("l1", true, "cyexbot timed out caressz for 10m: spam (3 times)"));
    }

    #[test]
    fn v107_m7_a_new_stack_every_ten_seconds() {
        let mut s = Stacks::default();
        hit(&mut s, "9", Source::Irc, IRC, 0, "l1");
        assert!(hit(&mut s, "9", Source::Irc, IRC, WINDOW_MS - 1, "l2").update);
        let h = hit(&mut s, "9", Source::Irc, IRC, WINDOW_MS, "l3");
        assert_eq!(h, Hit { id: "l3".into(), update: false, text: IRC.into() }, "counts from the first one");
        // Another user has their own stack.
        assert!(!hit(&mut s, "8", Source::Irc, IRC, WINDOW_MS + 1, "l4").update);
    }

    #[test]
    fn v107_m7_a_late_echo_joins_its_stack() {
        let mut s = Stacks::default();
        hit(&mut s, "9", Source::Irc, IRC, 0, "l1");
        hit(&mut s, "9", Source::Irc, IRC, 9_500, "x");
        // The feed's two copies come late: they belong to l1, not a new line.
        assert_eq!(hit(&mut s, "9", Source::Feed, FEED, 10_200, "x").id, "l1");
        let h = hit(&mut s, "9", Source::Feed, FEED, 10_700, "x");
        assert_eq!((h.id.as_str(), h.text.as_str()), ("l1", "cyexbot timed out caressz for 1m (2 times)."));
        // Caught up: the next one is a new stack.
        assert!(!hit(&mut s, "9", Source::Feed, FEED, 11_200, "l5").update);
    }

    #[test]
    fn v107_m7_echoes_at_the_window_edge_go_to_the_stack_behind() {
        let mut s = Stacks::default();
        hit(&mut s, "9", Source::Irc, IRC, 0, "l1");
        hit(&mut s, "9", Source::Irc, IRC, 9_500, "x");
        // A new timeout just past l1's window, before l1's feed copies came.
        assert!(!hit(&mut s, "9", Source::Irc, IRC, 10_100, "l2").update);
        // l1's two late feed copies: l1, not l2.
        let h = hit(&mut s, "9", Source::Feed, FEED, 10_200, "x");
        assert_eq!(h.id, "l1");
        let h = hit(&mut s, "9", Source::Feed, FEED, 10_700, "x");
        assert_eq!((h.id.as_str(), h.text.as_str()), ("l1", "cyexbot timed out caressz for 1m (2 times)."));
        // l2's own copy: l2, once.
        let h = hit(&mut s, "9", Source::Feed, "cyexbot timed out caressz for 1m: spam", 10_900, "x");
        assert_eq!((h.id.as_str(), h.text.as_str()), ("l2", "cyexbot timed out caressz for 1m: spam"));
    }

    #[test]
    fn v107_m7_a_second_feed_delivery_is_dropped() {
        let mut s = Stacks::default();
        hit(&mut s, "9", Source::Feed, FEED, 0, "l1");
        // The same action from the second mod account's feed.
        assert_eq!(s.hit("9", Source::Feed, FEED, 120, "l2", |_| true), None);
        let h = hit(&mut s, "9", Source::Irc, IRC, 200, "x");
        assert_eq!((h.id.as_str(), h.text.as_str()), ("l1", FEED), "still once");
        // Another length, or later on, counts.
        assert_eq!(hit(&mut s, "9", Source::Feed, "cyexbot timed out caressz for 5m.", 300, "x").text, "cyexbot timed out caressz for 5m (2 times).");
        assert!(hit(&mut s, "9", Source::Feed, "cyexbot timed out caressz for 5m.", 900, "x").update);
    }

    #[test]
    fn v107_m7_a_gone_line_starts_over() {
        let mut s = Stacks::default();
        hit(&mut s, "9", Source::Irc, IRC, 0, "l1");
        let h = s.hit("9", Source::Irc, IRC, 10, "l2", |id| id != "l1").unwrap();
        assert_eq!((h.id.as_str(), h.update), ("l2", false));
        assert_eq!(counted("a", 2), "a (2 times)");
        assert_eq!(counted("a.", 1), "a.");
    }

    fn clear(secs: u32, ts: i64) -> String {
        format!("@ban-duration={secs};target-user-id=9;tmi-sent-ts={ts} :tmi.twitch.tv CLEARCHAT #chan :caressz")
    }

    #[test]
    fn v107_m7_room_updates_one_line_in_place() {
        let mut r = super::super::Room::new("chan", None);
        let ev = r.handle_line(&Line::parse(&clear(60, 1_000)).unwrap(), 1_000, &NoEmotes);
        let RoomEvent::Message { message: first } = ev.last().unwrap() else { panic!("{ev:?}") };
        // The feed's copy of it, then a 30 s one from IRC and the feed.
        assert!(matches!(r.timeout_line("9", "cyexbot timed out caressz for 1m.", 1_200), Some(RoomEvent::Update { .. })));
        r.handle_line(&Line::parse(&clear(30, 3_000)).unwrap(), 3_000, &NoEmotes);
        let Some(RoomEvent::Update { message }) = r.timeout_line("9", "cyexbot timed out caressz for 30s.", 3_100) else { panic!() };
        assert_eq!(message.id, first.id);
        assert_eq!(message.notice.as_ref().unwrap().text, "cyexbot timed out caressz for 30s (2 times).");
        let lines = r.snapshot().messages.iter().filter(|m| m.notice.is_some()).count();
        assert_eq!(lines, 1);
        // Ten seconds after the first: a new line.
        let ev = r.handle_line(&Line::parse(&clear(600, 11_000)).unwrap(), 11_000, &NoEmotes);
        assert!(matches!(ev.last().unwrap(), RoomEvent::Message { message } if message.id != first.id));
    }

    #[test]
    fn v107_m7_a_skewed_pc_clock_still_stacks() {
        // The PC runs an hour ahead of Twitch: IRC's tmi-sent-ts and the local time differ.
        const SKEW: i64 = 3_600_000;
        let mut r = super::super::Room::new("chan", None);
        let ev = r.handle_line(&Line::parse(&clear(60, 1_000)).unwrap(), 1_000 + SKEW, &NoEmotes);
        let RoomEvent::Message { message: first } = ev.last().unwrap() else { panic!("{ev:?}") };
        assert_eq!(first.ts, 1_000, "the line keeps Twitch's time");
        let Some(RoomEvent::Update { message }) = r.timeout_line("9", "cyexbot timed out caressz for 1m.", 1_300 + SKEW) else { panic!() };
        assert_eq!(message.id, first.id, "one line, not two");
        // Twelve seconds later by Twitch but two by the PC (it caught up): still one stack
        // per the local clock, which both sources share.
        let ev = r.handle_line(&Line::parse(&clear(60, 13_000)).unwrap(), 3_000 + SKEW, &NoEmotes);
        assert!(matches!(ev.last().unwrap(), RoomEvent::Update { message } if message.id == first.id), "{ev:?}");
    }

    #[test]
    fn v107_m7_two_mod_accounts_count_a_feed_action_once() {
        let mut r = super::super::Room::new("chan", None);
        let text = "cyexbot timed out caressz for 1m.";
        assert!(matches!(r.timeout_line("9", text, 1_000), Some(RoomEvent::Message { .. })));
        assert!(r.timeout_line("9", text, 1_050).is_none(), "the second account's copy");
        let lines: Vec<String> = r.snapshot().messages.iter().filter_map(|m| m.notice.as_ref().map(|n| n.text.clone())).collect();
        assert_eq!(lines, [text]);
    }
}
