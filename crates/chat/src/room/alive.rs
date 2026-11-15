//! V111-08 Whether a joined room still hears its channel. Twitch stops sending a channel's
//! lines to an account banned there while it stays joined: no PART, no NOTICE, the
//! connection stays up. A room notices it by our own line coming back only from the writer
//! (never from the reader), or by another room of the channel hearing what this one doesn't
//! (the app compares, `app/src/chat/alive.rs`). A forced rejoin (PART and JOIN) gets it in
//! again, or makes Twitch say `msg_banned`, and what was missed fills from history.

use super::Room;
use crate::model::{ConnStatus, RoomEvent};

/// How long the reader gets to bring back a line Twitch took from the writer.
pub const ECHO_WAIT_MS: i64 = 15_000;

#[derive(Default)]
pub(super) struct Alive {
    /// When the last live line from someone else came (ms).
    heard_at: Option<i64>,
    /// When the room last got in (ms).
    joined_at: Option<i64>,
    /// Our line Twitch took, not yet back on the reader: its id and when it was taken.
    echo: Option<(String, i64)>,
    /// A forced rejoin is out: its answer adds no "Joined chat" line.
    quiet: bool,
}

impl Alive {
    pub(super) fn heard(&mut self, now_ms: i64) {
        self.heard_at = Some(now_ms);
    }

    pub(super) fn echoed(&mut self, id: &str) {
        if self.echo.as_ref().is_some_and(|(e, _)| e == id) {
            self.echo = None;
        }
    }

    /// The first one waiting stays: a dead room shows on it soonest.
    pub(super) fn expect_echo(&mut self, id: &str, now_ms: i64) {
        self.echo.get_or_insert_with(|| (id.to_owned(), now_ms));
    }

    /// The status changes to `status`. True: the line for it stays out (a forced rejoin).
    pub(super) fn settle(&mut self, status: ConnStatus, now_ms: i64) -> bool {
        if status == ConnStatus::Joined {
            self.joined_at = Some(now_ms);
        }
        if matches!(status, ConnStatus::Connecting | ConnStatus::Reconnecting) {
            return false;
        }
        // Any answer ends the wait; only getting in is said by the app instead.
        self.echo = None;
        std::mem::take(&mut self.quiet) && status == ConnStatus::Joined
    }
}

impl Room {
    /// V111-08 A forced rejoin goes out (F5, a ban, a dead room): the room waits for Twitch's
    /// answer, and what was said since it last heard anyone is a gap to fill once it's in.
    pub fn rejoin(&mut self, now_ms: i64) -> Vec<RoomEvent> {
        self.deaf_since_heard(now_ms);
        self.alive.echo = None;
        self.alive.quiet = true;
        self.set_status(ConnStatus::Connecting, now_ms)
    }

    /// V111-40 What was said since the room last heard anyone is a gap to fill later; an
    /// earlier mark stays. Set the moment we're banned, before the anonymous read starts
    /// (its lines count as heard).
    pub fn deaf_since_heard(&mut self, now_ms: i64) {
        let from = self.alive.heard_at.or(self.alive.joined_at).unwrap_or(now_ms).min(now_ms);
        self.lost_at = Some(self.lost_at.map_or(from, |t| t.min(from)));
    }

    /// V111-08 Joined, and our line went out on the writer `ECHO_WAIT_MS` ago without coming
    /// back on the reader: Twitch isn't sending this chat to us.
    pub fn stalled(&self, now_ms: i64) -> bool {
        self.status == ConnStatus::Joined && self.alive.echo.as_ref().is_some_and(|(_, at)| now_ms - at >= ECHO_WAIT_MS)
    }

    /// V111-08 When this room last heard its channel: someone's live line, else getting in.
    pub fn heard(&self) -> Option<i64> {
        match (self.alive.heard_at, self.alive.joined_at) {
            (Some(h), Some(j)) => Some(h.max(j)),
            (h, j) => h.or(j),
        }
    }

    /// V111-08 When someone else's live line last came, joined or not.
    pub fn heard_others(&self) -> Option<i64> {
        self.alive.heard_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::Me;
    use crate::irc::Line;
    use crate::model::{Flag, NoticeKind, Restriction};
    use crate::tokenize::NoEmotes;

    const ME: &str = "1";

    fn send(text: &str, nonce: &str, at_ms: i64) -> crate::room::PendingSend {
        crate::room::PendingSend { text: text.into(), at_ms, nonce: Some(nonce.into()), written_ms: Some(at_ms), ..Default::default() }
    }

    fn room() -> Room {
        let mut r = Room::new("chan", Some(Me::new(ME, "me", "Me")));
        r.set_status(ConnStatus::Joined, 0);
        r
    }

    fn feed(r: &mut Room, raw: &str, now: i64) -> Vec<RoomEvent> {
        r.handle_line(&Line::parse(raw).unwrap(), now, &NoEmotes)
    }

    fn say(r: &mut Room, id: &str, user: &str, now: i64) -> Vec<RoomEvent> {
        feed(r, &format!("@id={id};tmi-sent-ts={now};user-id={user} :{user}!{user}@{user} PRIVMSG #chan :hi"), now)
    }

    fn lines(events: &[RoomEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|e| match e {
                RoomEvent::Message { message } if message.notice.as_ref().is_some_and(|n| n.kind == NoticeKind::Info) => {
                    Some(message.notice.as_ref().unwrap().text.clone())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn v111_08_a_forced_rejoin_fills_from_the_last_line_heard() {
        let mut r = room();
        say(&mut r, "a", "7", 1_000);
        let ev = r.rejoin(60_000);
        assert_eq!(r.status, ConnStatus::Connecting);
        assert!(lines(&ev).is_empty(), "going out says nothing");
        // Twitch lets us in: no "Joined chat", the gap since the last line is asked for.
        let ev = r.set_status(ConnStatus::Joined, 61_000);
        assert!(lines(&ev).is_empty(), "{:?}", lines(&ev));
        let (after, before, limit) = r.take_gap(61_000).expect("a gap");
        assert_eq!((after, before, limit), (1_000, 61_000, 610));
        // A later plain drop and return says so again.
        r.lost(70_000);
        r.set_status(ConnStatus::Reconnecting, 70_000);
        assert_eq!(lines(&r.set_status(ConnStatus::Joined, 71_000)), ["Reconnected"]);
    }

    #[test]
    fn v111_08_a_forced_rejoin_of_a_quiet_room_starts_where_it_got_in() {
        let mut r = Room::new("chan", Some(Me::new(ME, "me", "Me")));
        r.set_status(ConnStatus::Joined, 5_000);
        r.rejoin(9_000);
        r.set_status(ConnStatus::Joined, 9_500);
        assert_eq!(r.take_gap(9_500).map(|g| g.0), Some(5_000));
        assert_eq!(r.heard(), Some(9_500));
    }

    #[test]
    fn v111_40_the_gap_starts_at_the_ban_not_at_the_anonymous_read() {
        let mut r = room();
        say(&mut r, "a", "7", 1_000);
        feed(&mut r, &format!("@room-id=9;target-user-id={ME};tmi-sent-ts=2000 :tmi.twitch.tv CLEARCHAT #chan :me"), 2_000);
        r.deaf_since_heard(2_000);
        // The anonymous read's first lines arrive before the rejoin goes out.
        say(&mut r, "b", "8", 3_000);
        r.rejoin(4_000);
        feed(&mut r, "@msg-id=msg_banned :tmi.twitch.tv NOTICE #chan :You are permanently banned from talking in chan.", 4_500);
        assert_eq!(r.take_gap(4_500).map(|g| g.0), Some(1_000));
    }

    #[test]
    fn v111_08_banned_while_in_the_rejoin_answer_is_the_ban() {
        let mut r = room();
        say(&mut r, "a", "7", 1_000);
        // Our own ban: the lock, still "joined" as far as IRC goes.
        let ev = feed(&mut r, &format!("@room-id=9;target-user-id={ME};tmi-sent-ts=2000 :tmi.twitch.tv CLEARCHAT #chan :me"), 2_000);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Restricted { how: Restriction::Banned })));
        assert_eq!(r.status, ConnStatus::Joined);
        r.rejoin(4_000);
        let ev = feed(&mut r, "@msg-id=msg_banned :tmi.twitch.tv NOTICE #chan :You are permanently banned from talking in chan.", 4_500);
        assert_eq!(r.status, ConnStatus::Banned, "{ev:?}");
        // Read anonymously meanwhile (the app's mirror): lines keep coming, the gap fills.
        assert_eq!(r.take_gap(4_500).map(|g| g.0), Some(1_000));
        assert!(say(&mut r, "b", "8", 5_000).iter().any(|e| matches!(e, RoomEvent::Message { .. })));
        assert_eq!(r.heard_others(), Some(5_000));
        // Unbanned: the account's JOIN echo brings it back with "Joined chat".
        r.set_status(ConnStatus::Connecting, 9_000);
        assert_eq!(lines(&r.set_status(ConnStatus::Joined, 9_100)), ["Joined chat"]);
    }

    #[test]
    fn v111_08_timed_out_the_rejoin_lets_us_in_and_the_wait_stays() {
        let mut r = room();
        let ev = feed(&mut r, &format!("@room-id=9;ban-duration=600;target-user-id={ME};tmi-sent-ts=2000 :tmi.twitch.tv CLEARCHAT #chan :me"), 2_000);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Restricted { how: Restriction::TimedOut { secs: 600 } })));
        r.rejoin(3_000);
        // A timed-out account still gets in; the lock is the standing's business, not the link's.
        feed(&mut r, ":me!me@me.tmi.twitch.tv JOIN #chan", 3_200);
        r.set_status(ConnStatus::Joined, 3_200);
        assert_eq!(r.status, ConnStatus::Joined);
        assert!(say(&mut r, "c", "8", 4_000).iter().any(|e| matches!(e, RoomEvent::Message { .. })));
    }

    #[test]
    fn v111_08_our_line_back_only_from_the_writer_is_a_dead_room() {
        let mut r = room();
        r.sending(send("hello", "n1", 10_000));
        // The writer's USERSTATE: Twitch took it, the echo is built here.
        feed(&mut r, &format!("@client-nonce=n1;id=s1;user-id={ME} :tmi.twitch.tv USERSTATE #chan"), 10_100);
        assert!(!r.stalled(20_000));
        assert!(r.stalled(10_100 + ECHO_WAIT_MS), "never came back on the reader");
        // Came back: alive.
        feed(&mut r, &format!("@id=s1;tmi-sent-ts=10200;user-id={ME} :me!me@me PRIVMSG #chan :hello"), 10_300);
        assert!(!r.stalled(60_000));
        // A rejoin ends the wait too.
        r.sending(send("again", "n2", 70_000));
        feed(&mut r, &format!("@client-nonce=n2;id=s2;user-id={ME} :tmi.twitch.tv USERSTATE #chan"), 70_100);
        r.rejoin(71_000);
        assert!(!r.stalled(200_000));
        let mine = r.snapshot().messages.iter().filter(|m| m.has(Flag::Own)).count();
        assert_eq!(mine, 2);
    }
}
