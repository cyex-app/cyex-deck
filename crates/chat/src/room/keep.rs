//! V107-M1 How much a room keeps. A room no view shows keeps `BACKGROUND` lines; once
//! shown again it keeps the user's scrollback, and the history service brings back the
//! older lines it dropped (`backfill`).

use std::collections::HashMap;
use std::sync::Arc;

use super::Room;
use crate::model::{ChatMessage, Flag};
use crate::tokenize::Emotes;

/// Lines a room keeps while no view shows it.
pub const BACKGROUND: usize = 150;
/// History lines a room loads when it's joined in the background.
pub const BACKGROUND_HISTORY: u32 = 100;

impl Room {
    /// Most lines kept right now.
    pub fn limit(&self) -> usize {
        if self.background { BACKGROUND.min(self.keep) } else { self.keep }
    }

    pub fn background(&self) -> bool {
        self.background
    }

    /// No view shows the room (`true`): it drops to `BACKGROUND` lines. Shown again, it
    /// may grow back to the scrollback.
    pub fn set_background(&mut self, on: bool) {
        self.background = on;
        self.trim();
    }

    /// Lines the buffer could still take.
    pub fn missing(&self) -> usize {
        self.limit().saturating_sub(self.buffer.len())
    }

    /// History lines older than everything here, to fill the buffer up again after it was
    /// shown in the background. Lines that were seen live keep their normal look.
    pub fn backfill(&mut self, lines: &[String], now_ms: i64, emotes: &dyn Emotes) -> Vec<Arc<ChatMessage>> {
        let first = self.buffer.iter().find(|m| m.sender.is_some()).or(self.buffer.front());
        let before = first.map(|m| m.ts);
        self.older(lines, before, now_ms, emotes)
    }

    /// The current copies of these lines, oldest first; gone ones are left out. A refill
    /// sends its lines bit by bit, and meanwhile they may have been deleted or changed.
    pub fn current(&self, ids: &[String]) -> Vec<Arc<ChatMessage>> {
        let by_id: HashMap<&str, &Arc<ChatMessage>> = self.buffer.iter().map(|m| (m.id.as_str(), m)).collect();
        ids.iter().filter_map(|id| by_id.get(id.as_str()).map(|m| Arc::clone(m))).collect()
    }

    /// History, dimmed, unless a refill brings back what was said after the room went live.
    pub(super) fn mark_old(&self, m: &mut ChatMessage, refill: bool) {
        if !refill || self.live_from.is_none_or(|t| m.ts < t) {
            m.set(Flag::History);
        } else {
            m.flags.retain(|f| *f != Flag::History);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irc::Line;
    use crate::tokenize::NoEmotes;

    fn line(id: &str, ts: i64) -> String {
        format!("@badges=;color=;display-name=A;emotes=;id={id};room-id=1;tmi-sent-ts={ts};user-id=7 :a!a@a PRIVMSG #chan :hi {id}")
    }

    fn feed(r: &mut Room, id: &str, ts: i64) {
        r.handle_line(&Line::parse(&line(id, ts)).unwrap(), ts, &NoEmotes);
    }

    fn ids(r: &Room) -> Vec<String> {
        r.snapshot().messages.iter().map(|m| m.id.clone()).collect()
    }

    fn room_with(n: usize) -> Room {
        let mut r = Room::new("chan", None);
        r.set_keep(1000);
        for i in 0..n {
            feed(&mut r, &format!("m{i}"), 1_000 + i as i64);
        }
        r
    }

    #[test]
    fn background_keeps_few_and_shown_keeps_the_scrollback() {
        let mut r = room_with(800);
        assert_eq!(r.limit(), 1000);
        r.set_background(true);
        assert_eq!((r.limit(), ids(&r).len()), (BACKGROUND, BACKGROUND));
        assert_eq!(ids(&r)[0], "m650", "the newest stay");
        feed(&mut r, "x", 5_000);
        assert_eq!(ids(&r).len(), BACKGROUND, "still capped");
        r.set_background(false);
        assert_eq!((ids(&r).len(), r.missing()), (BACKGROUND, 1000 - BACKGROUND));
    }

    #[test]
    fn background_never_keeps_more_than_the_scrollback() {
        let mut r = room_with(10);
        r.set_keep(0);
        r.set_background(true);
        assert_eq!(r.limit(), BACKGROUND);
        assert_eq!(r.keep(), crate::room::BUFFER);
    }

    #[test]
    fn backfill_adds_only_older_lines_without_duplicates() {
        let mut r = room_with(300);
        r.set_background(true);
        r.set_background(false);
        // The service has m0..m299: the kept ones (m150..) come again and are skipped.
        let lines: Vec<String> = (0..300).map(|i| line(&format!("m{i}"), 1_000 + i)).collect();
        let added = r.backfill(&lines, 9_000, &NoEmotes);
        assert_eq!(added.len(), 150);
        assert_eq!((added[0].id.as_str(), added[149].id.as_str()), ("m0", "m149"));
        let all = ids(&r);
        assert_eq!(all.len(), 300);
        assert!(all.iter().enumerate().all(|(i, id)| *id == format!("m{i}")), "in order");
        assert!(!added[0].has(Flag::History), "seen live before: not dimmed");
    }

    #[test]
    fn backfill_skips_lines_newer_than_the_oldest_kept() {
        let mut r = room_with(200);
        r.set_background(true);
        r.set_background(false);
        // A line the room never kept (hidden live, say) that's newer than the oldest kept.
        let mut lines: Vec<String> = (0..50).map(|i| line(&format!("m{i}"), 1_000 + i)).collect();
        lines.push(line("late", 1_190));
        let added = r.backfill(&lines, 9_000, &NoEmotes);
        assert_eq!(added.len(), 50);
        assert!(added.iter().all(|m| m.id != "late"));
    }

    #[test]
    fn backfill_fills_up_to_the_limit_newest_first() {
        let mut r = room_with(0);
        r.set_keep(500);
        feed(&mut r, "live", 10_000);
        let lines: Vec<String> = (0..800).map(|i| line(&format!("h{i}"), 1_000 + i)).collect();
        let added = r.backfill(&lines, 20_000, &NoEmotes);
        assert_eq!(added.len(), 499);
        assert_eq!(added[0].id, "h301", "the oldest are left out");
        assert!(added[0].has(Flag::History), "before the room went live: dimmed");
    }

    #[test]
    fn current_copies_follow_changes() {
        let mut r = room_with(5);
        r.handle_line(&Line::parse("@target-msg-id=m2 :tmi.twitch.tv CLEARMSG #chan :x").unwrap(), 9, &NoEmotes);
        let got = r.current(&["m1".into(), "m2".into(), "gone".into()]);
        assert_eq!(got.len(), 2);
        assert!(got[1].has(Flag::Deleted));
    }
}
