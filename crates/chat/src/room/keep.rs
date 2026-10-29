//! V107-M1, V108-C01 How much a room keeps. Shown or hidden, a room keeps the user's
//! scrollback; only the line budget (`budget`) shrinks a hidden one to `BACKGROUND`.
//! Shown again, it keeps the scrollback, and the history service brings back the older
//! lines it dropped (`backfill`).

use std::sync::Arc;

use super::Room;
use crate::build::Me;
use crate::model::{ChatMessage, ConnStatus, Flag};
use crate::tokenize::Emotes;

/// Lines a hidden room keeps once the line budget shrank it.
pub const BACKGROUND: usize = 150;

impl Room {
    /// Most lines kept right now.
    pub fn limit(&self) -> usize {
        if self.background { BACKGROUND.min(self.keep) } else { self.keep }
    }

    /// Shrunk to `BACKGROUND` by the line budget.
    pub fn background(&self) -> bool {
        self.background
    }

    /// `true`: the line budget drops the room to `BACKGROUND` lines. Shown again
    /// (`false`), it may grow back to the scrollback.
    pub fn set_background(&mut self, on: bool) {
        self.background = on;
        self.trim();
    }

    /// V108-C01 A parked room comes back: it joins anew, and what was said since it was
    /// parked (`parked_at`) is a gap to fill once the JOIN is in (`take_gap`).
    /// A room that held live lines fills the gap too when it wasn't joined when parked
    /// (its sign-in failed, say).
    pub fn resume(&mut self, parked_at: i64, now_ms: i64) {
        self.lost(parked_at);
        if self.lost_at.is_none() && self.live_from.is_some() {
            self.lost_at = Some(parked_at);
        }
        self.set_status(ConnStatus::Connecting, now_ms);
    }

    /// V108-C01 A gap fill that never landed (the room was parked meanwhile): the next
    /// fill starts from its start, or from an earlier point if one is already open.
    pub fn unfilled(&mut self, after: i64) {
        self.lost_at = Some(self.lost_at.map_or(after, |t| t.min(after)));
    }

    /// V108-C01 The account reading it, again: a restored room reads with today's names.
    pub fn set_me(&mut self, me: Option<Me>) {
        self.me = me;
    }

    /// Lines the buffer could still take.
    pub fn missing(&self) -> usize {
        self.limit().saturating_sub(self.buffer.len())
    }

    /// History lines older than everything here, to fill the buffer up again after the
    /// line budget shrank it. Lines that were seen live keep their normal look.
    pub fn backfill(&mut self, lines: &[String], now_ms: i64, emotes: &dyn Emotes) -> Vec<Arc<ChatMessage>> {
        let first = self.buffer.iter().find(|m| m.sender.is_some()).or(self.buffer.front());
        let before = first.map(|m| m.ts);
        self.older(lines, before, now_ms, emotes)
    }

    /// Lines in the buffer now.
    pub fn kept(&self) -> usize {
        self.buffer.len()
    }

    /// Lines were dropped since the last refill (`trim`), or the first history came into a
    /// shrunk room and was cut short: only then is a refill worth asking for.
    pub fn trimmed(&self) -> bool {
        self.trimmed
    }

    pub fn set_trimmed(&mut self, on: bool) {
        self.trimmed = on;
    }

    /// How many history lines a refill asks for: the user's history lines above what's
    /// kept, never more than the buffer takes.
    pub fn refill_want(&self, history_lines: usize) -> usize {
        (history_lines + self.kept()).min(self.limit())
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
    fn only_a_trimmed_room_counts_as_trimmed() {
        let mut r = room_with(100);
        r.set_background(true);
        assert!(!r.trimmed(), "under the background cap: nothing dropped");
        r.set_background(false);
        let mut r = room_with(300);
        r.set_background(true);
        assert!(r.trimmed());
        r.set_trimmed(false);
        feed(&mut r, "x", 9_000);
        assert!(r.trimmed(), "a live line pushed one out");
    }

    #[test]
    fn v108_01_history_into_a_shrunk_room_counts_as_trimmed() {
        let mut r = room_with(0);
        r.set_background(true);
        let lines: Vec<String> = (0..400).map(|i| line(&format!("h{i}"), 1_000 + i)).collect();
        assert_eq!(r.history(&lines, 9_000, &NoEmotes).len(), BACKGROUND);
        assert!(r.trimmed(), "shown, it refills");
        let mut r = room_with(0);
        assert_eq!(r.history(&lines, 9_000, &NoEmotes).len(), 400);
        assert!(!r.trimmed(), "full size: nothing left out");
    }

    #[test]
    fn v108_01_a_resumed_room_fills_from_when_it_was_parked() {
        let mut r = room_with(0);
        r.set_status(ConnStatus::Joined, 500);
        feed(&mut r, "a", 1_000);
        r.resume(2_000, 60_000);
        assert_eq!(r.status, ConnStatus::Connecting);
        r.set_status(ConnStatus::Joined, 61_000);
        assert_eq!(r.take_gap(61_000), Some((2_000, 61_000, 600)));
        // Never joined: nothing to fill.
        let mut r = room_with(0);
        r.resume(2_000, 60_000);
        assert_eq!(r.take_gap(61_000), None);
    }

    #[test]
    fn v108_01_a_room_parked_while_not_joined_still_fills_if_it_had_live_lines() {
        let mut r = room_with(0);
        r.set_status(ConnStatus::Joined, 500);
        feed(&mut r, "a", 1_000);
        r.set_status(ConnStatus::AuthFailed, 1_500);
        r.resume(2_000, 60_000);
        r.set_status(ConnStatus::Joined, 61_000);
        assert_eq!(r.take_gap(61_000).map(|g| g.0), Some(2_000));
    }

    #[test]
    fn v108_01_an_unfilled_gap_keeps_its_earliest_start() {
        let mut r = room_with(0);
        r.set_status(ConnStatus::Joined, 500);
        feed(&mut r, "a", 1_000);
        r.lost(2_000);
        let (after, ..) = r.take_gap(3_000).unwrap();
        // Parked while the fill was out: it comes back, later than the gap began.
        r.unfilled(after);
        r.resume(9_000, 60_000);
        r.set_status(ConnStatus::Joined, 61_000);
        assert_eq!(r.take_gap(61_000).map(|g| g.0), Some(2_000));
        r.unfilled(5_000);
        r.unfilled(4_000);
        assert_eq!(r.take_gap(61_000).map(|g| g.0), Some(4_000), "the earlier one wins");
    }

    #[test]
    fn v108_01_set_me_replaces_the_reader() {
        let mut r = Room::new("chan", Some(Me::new("1", "old", "Old")));
        r.set_me(Some(Me::new("1", "new", "New")));
        assert_eq!(r.me().map(|m| m.login.as_str()), Some("new"));
    }

    #[test]
    fn a_refill_asks_for_the_history_lines_above_what_is_kept() {
        let mut r = room_with(300);
        r.set_background(true);
        r.set_background(false);
        // 150 kept, 500 history lines: 650, under the 1000 scrollback.
        assert_eq!(r.refill_want(500), 650);
        assert_eq!(r.refill_want(50), 200);
        r.set_keep(500);
        assert_eq!(r.refill_want(500), 500, "never more than the buffer takes");
    }
}
