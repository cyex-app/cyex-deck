//! Our sends until Twitch answers them (1.8, V105-06). V111-30 Each written line gets an answer
//! or a visible "Not sent" line: the writer's USERSTATE (or the reader relaying our line)
//! confirms it, a NOTICE refuses it, and silence past [`CONFIRM_MS`] counts as not sent.
//! The line carries the text for Copy and Retry; a late answer takes it back.

use std::sync::Arc;

use super::{PendingSend, Room};
use crate::build;
use crate::irc::Line;
use crate::model::{ChatMessage, NoticeKind, RoomEvent, Unsent};

/// V111-30 How long Twitch gets to answer a written line. It answers within a second; 5 s
/// leaves room for a slow link without leaving the user wondering.
pub const CONFIRM_MS: i64 = 5_000;
/// V111-30 Twitch answers a written line within about a second: a NOTICE later than this after
/// a line is taken as the answer to a newer one, if there is one.
const ANSWER_MS: i64 = 1_500;
/// A send that never got written nor dropped (the app lost track) is forgotten after this.
const PENDING_TTL_MS: i64 = 30_000;
/// A "no reply" line is taken back when Twitch answers this soon after all.
const LATE_MS: i64 = 60_000;
/// "No reply" sends kept for a late answer.
const LATE_MAX: usize = 20;

pub const NO_REPLY: &str = "Not sent: no reply from Twitch.";
pub const LOST: &str = "Not sent: lost the connection.";

/// V111-30 Whether Retry can help after `msg_id`: not after a ban, a timeout, an AutoMod hold
/// or an account check, where the same send meets the same answer.
pub fn retry_helps(msg_id: &str) -> bool {
    !matches!(
        msg_id,
        "msg_banned"
            | "msg_channel_blocked"
            | "msg_channel_suspended"
            | "msg_timedout"
            | "msg_rejected"
            | "msg_suspended"
            | "msg_verified_email"
            | "msg_requires_verified_phone_number"
            | "tos_ban"
    ) && !msg_id.contains("warn")
}

/// What a "Not sent" line offers for `p`.
pub fn unsent(p: &PendingSend, retry: bool) -> Unsent {
    Unsent {
        text: p.typed.clone(),
        reply_to: p.reply.as_ref().map(|r| r.parent_id.clone()),
        retry,
    }
}

impl Room {
    pub fn sending(&mut self, send: PendingSend) {
        self.expire(send.at_ms);
        self.pending.push_back(send);
    }

    /// The send with `nonce` didn't go out: Twitch won't answer it.
    pub fn unsend(&mut self, nonce: &str) -> Option<PendingSend> {
        let i = self.pending.iter().position(|p| p.nonce.as_deref() == Some(nonce))?;
        self.pending.remove(i)
    }

    /// V111-30 The send with `nonce` was written: Twitch's answer is due from now.
    pub fn wrote(&mut self, nonce: &str, now_ms: i64) {
        if let Some(p) = self.pending.iter_mut().find(|p| p.nonce.as_deref() == Some(nonce)) {
            p.written_ms = Some(now_ms);
        }
    }

    /// V111-30 The wording Twitch last took here (no older than `since_ms`): a repeat must differ.
    pub fn last_taken(&self, since_ms: i64) -> Option<&str> {
        self.taken.as_ref().filter(|(_, at)| *at >= since_ms).map(|(w, _)| w.as_str())
    }

    /// V111-30 A send that didn't go out after all (the link dropped before it was written):
    /// a "Not sent" line with Retry.
    pub fn not_written(&mut self, nonce: &str, now_ms: i64) -> Option<RoomEvent> {
        let p = self.unsend(nonce)?;
        Some(self.unsent_line(LOST, &p, true, now_ms).0)
    }

    /// V111-30 Written sends Twitch hasn't answered in [`CONFIRM_MS`]: each gets a "Not sent"
    /// line, kept a while in case the answer comes late.
    pub fn unanswered(&mut self, now_ms: i64) -> Vec<RoomEvent> {
        let due = |p: &PendingSend| p.written_ms.is_some_and(|w| now_ms - w >= CONFIRM_MS);
        let mut events = Vec::new();
        while let Some(i) = self.pending.iter().position(due) {
            let Some(p) = self.pending.remove(i) else { break };
            let (ev, id) = self.unsent_line(NO_REPLY, &p, true, now_ms);
            events.push(ev);
            self.unconfirmed.push_back((p, id));
            if self.unconfirmed.len() > LATE_MAX {
                self.unconfirmed.pop_front();
            }
        }
        events
    }

    fn unsent_line(&mut self, text: &str, p: &PendingSend, retry: bool, now_ms: i64) -> (RoomEvent, String) {
        let mut m: ChatMessage = build::system(NoticeKind::Error, text, now_ms);
        if let Some(n) = m.notice.as_mut() {
            n.unsent = Some(unsent(p, retry));
        }
        let id = m.id.clone();
        let m = Arc::new(m);
        self.push(m.clone());
        (RoomEvent::Message { message: m }, id)
    }

    /// The send a USERSTATE answers: the one with its nonce, or without one the oldest written
    /// (else the oldest). A late answer to one called "not sent" takes that line back.
    pub(super) fn answered(&mut self, nonce: Option<&str>, now_ms: i64, events: &mut Vec<RoomEvent>) -> Option<PendingSend> {
        let p = match nonce {
            Some(n) => match self.pending.iter().position(|p| p.nonce.as_deref() == Some(n)) {
                Some(i) => self.pending.remove(i),
                None => self.late(n, events),
            },
            None => {
                let i = self.pending.iter().position(|p| p.written_ms.is_some()).unwrap_or(0);
                self.pending.remove(i)
            }
        }?;
        self.taken = Some((p.sent_as.clone(), now_ms));
        Some(p)
    }

    /// V111-30 Twitch answered a send already called "not sent": its line goes.
    fn late(&mut self, nonce: &str, events: &mut Vec<RoomEvent>) -> Option<PendingSend> {
        let i = self.unconfirmed.iter().position(|(p, _)| p.nonce.as_deref() == Some(nonce))?;
        let (p, line) = self.unconfirmed.remove(i)?;
        events.extend(self.remove(&line));
        Some(p)
    }

    /// V111-30 Our line as the reader relays it (its `client-nonce` is ours) confirms the send
    /// as the USERSTATE does, should that be lost.
    pub(super) fn relayed(&mut self, line: &Line<'_>, now_ms: i64) -> Vec<RoomEvent> {
        let mut events = Vec::new();
        if line.command != "PRIVMSG" {
            return events;
        }
        let Some(nonce) = line.tag_nonempty("client-nonce") else { return events };
        let mine = self.me.as_ref().is_some_and(|m| line.tag_nonempty("user-id") == Some(m.user_id.as_str()));
        if mine {
            self.answered(Some(nonce), now_ms, &mut events);
        }
        events
    }

    /// V111-30 The send a refusal NOTICE answers (it names none): the oldest written one Twitch
    /// could still be answering. One still waiting for its turn hasn't reached Twitch; one written
    /// over [`ANSWER_MS`] ago is passed over for a newer one, as Twitch answers in order within
    /// a second, so an older one it ignored is not what this NOTICE is about.
    pub(super) fn refused(&mut self, now_ms: i64) -> Option<PendingSend> {
        self.expire(now_ms);
        let written: Vec<(usize, i64)> = self
            .pending
            .iter()
            .enumerate()
            .filter_map(|(i, p)| Some((i, p.written_ms?)))
            .collect();
        let newest = written.last()?.0;
        let i = written.iter().find(|(_, w)| now_ms - w <= ANSWER_MS).map_or(newest, |(i, _)| *i);
        self.pending.remove(i)
    }

    pub(super) fn expire(&mut self, now_ms: i64) {
        self.pending.retain(|p| now_ms - p.at_ms <= PENDING_TTL_MS);
        self.unconfirmed.retain(|(p, _)| now_ms - p.written_ms.unwrap_or(p.at_ms) <= LATE_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::Me;
    use crate::model::{Flag, plain};
    use crate::tokenize::NoEmotes;

    fn room() -> Room {
        Room::new("chan", Some(Me::new("42", "me", "Me")))
    }

    fn feed(r: &mut Room, raw: &str, now: i64) -> Vec<RoomEvent> {
        r.handle_line(&Line::parse(raw).unwrap(), now, &NoEmotes)
    }

    fn send(r: &mut Room, text: &str, nonce: &str, at: i64) {
        r.sending(PendingSend {
            text: text.into(),
            typed: text.into(),
            sent_as: text.into(),
            at_ms: at,
            nonce: Some(nonce.into()),
            ..PendingSend::default()
        });
    }

    fn unsent_of(ev: &[RoomEvent]) -> Vec<(String, Unsent)> {
        ev.iter()
            .filter_map(|e| match e {
                RoomEvent::Message { message } => {
                    let n = message.notice.as_ref()?;
                    Some((n.text.clone(), n.unsent.clone()?))
                }
                _ => None,
            })
            .collect()
    }

    fn userstate(nonce: &str, id: &str) -> String {
        format!("@badges=;client-nonce={nonce};id={id};mod=0 :tmi.twitch.tv USERSTATE #chan")
    }

    #[test]
    fn silence_past_the_timeout_says_not_sent() {
        let mut r = room();
        send(&mut r, "hello", "n1", 0);
        // Waiting for its turn: no answer is due yet.
        assert!(r.unanswered(CONFIRM_MS * 2).is_empty());
        r.wrote("n1", 1_000);
        assert!(r.unanswered(1_000 + CONFIRM_MS - 1).is_empty());
        let ev = r.unanswered(1_000 + CONFIRM_MS);
        let got = unsent_of(&ev);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, NO_REPLY);
        assert_eq!((got[0].1.text.as_str(), got[0].1.retry), ("hello", true));
        // Said once.
        assert!(r.unanswered(1_000 + CONFIRM_MS * 3).is_empty());
    }

    #[test]
    fn an_answer_in_time_says_nothing() {
        let mut r = room();
        send(&mut r, "hello", "n1", 0);
        r.wrote("n1", 0);
        let ev = feed(&mut r, &userstate("n1", "s1"), 800);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message } if message.has(Flag::Own))));
        assert!(r.unanswered(CONFIRM_MS * 2).is_empty());
        assert_eq!(r.last_taken(0), Some("hello"));
    }

    #[test]
    fn a_late_answer_takes_the_line_back_and_shows_the_message() {
        let mut r = room();
        send(&mut r, "hello", "n1", 0);
        r.wrote("n1", 0);
        let ev = r.unanswered(CONFIRM_MS);
        let RoomEvent::Message { message: line } = &ev[0] else { panic!() };
        let ev = feed(&mut r, &userstate("n1", "s1"), CONFIRM_MS + 2_000);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Remove { id } if *id == line.id)), "{ev:?}");
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message } if plain(&message.body) == "hello")));
        assert!(r.message(&line.id).is_none());
    }

    #[test]
    fn the_reader_relaying_our_line_confirms_it() {
        let mut r = room();
        send(&mut r, "hello", "n1", 0);
        r.wrote("n1", 0);
        feed(&mut r, "@client-nonce=n1;id=s1;user-id=42 :me!me@me.tmi.twitch.tv PRIVMSG #chan :hello", 500);
        assert!(r.unanswered(CONFIRM_MS * 2).is_empty());
        // Someone else's line with the same nonce confirms nothing.
        send(&mut r, "again", "n2", 0);
        r.wrote("n2", 0);
        feed(&mut r, "@client-nonce=n2;id=x1;user-id=7 :u7!u@u PRIVMSG #chan :again", 500);
        assert_eq!(unsent_of(&r.unanswered(CONFIRM_MS * 2)).len(), 1);
    }

    #[test]
    fn a_refusal_names_the_text_and_skips_lines_still_waiting() {
        let mut r = room();
        send(&mut r, "first", "n1", 0);
        send(&mut r, "second", "n2", 0);
        r.wrote("n1", 0);
        let ev = feed(&mut r, "@msg-id=msg_duplicate :tmi.twitch.tv NOTICE #chan :Your message is identical", 300);
        let got = unsent_of(&ev);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "Not sent: same message as before.");
        assert_eq!(got[0].1.text, "first");
        // A NOTICE while nothing written is out refuses no send.
        let ev = feed(&mut r, "@msg-id=msg_ratelimit :tmi.twitch.tv NOTICE #chan :too fast", 400);
        assert!(unsent_of(&ev).is_empty());
        // The waiting one is still there and shows when Twitch takes it.
        r.wrote("n2", 1_200);
        let ev = feed(&mut r, &userstate("n2", "s2"), 1_500);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message } if plain(&message.body) == "second")));
        // A refused line was never taken.
        assert_eq!(r.last_taken(0), Some("second"));
    }

    /// One Twitch ignored doesn't take the refusal meant for the line after it.
    #[test]
    fn a_refusal_skips_an_older_line_twitch_ignored() {
        let mut r = room();
        send(&mut r, "ignored", "n1", 0);
        r.wrote("n1", 0);
        send(&mut r, "hi", "n2", 0);
        r.wrote("n2", 2_500);
        let ev = feed(&mut r, "@msg-id=msg_duplicate :tmi.twitch.tv NOTICE #chan :identical", 2_800);
        assert_eq!(unsent_of(&ev)[0].1.text, "hi");
        let got = unsent_of(&r.unanswered(CONFIRM_MS + 1));
        assert_eq!(got[0].1.text, "ignored");
    }

    #[test]
    fn answers_go_in_order() {
        let mut r = room();
        for (i, t) in ["a", "b", "c"].iter().enumerate() {
            send(&mut r, t, &format!("n{i}"), 0);
            r.wrote(&format!("n{i}"), i as i64 * 1_100);
        }
        // Refusal for the oldest, nonce-less USERSTATE for the next, the last by its nonce.
        let ev = feed(&mut r, "@msg-id=msg_ratelimit :tmi.twitch.tv NOTICE #chan :too fast", 1_500);
        assert_eq!(unsent_of(&ev)[0].1.text, "a");
        let ev = feed(&mut r, "@badges=;id=s1;mod=0 :tmi.twitch.tv USERSTATE #chan", 3_100);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message } if plain(&message.body) == "b")));
        let ev = feed(&mut r, &userstate("n2", "s2"), 3_200);
        assert!(ev.iter().any(|e| matches!(e, RoomEvent::Message { message } if plain(&message.body) == "c")));
    }

    #[test]
    fn a_dropped_write_says_lost_the_connection() {
        let mut r = room();
        send(&mut r, "/me waves", "n1", 0);
        let ev = r.not_written("n1", 10).unwrap();
        let got = unsent_of(&[ev]);
        assert_eq!(got[0].0, LOST);
        assert_eq!(got[0].1.text, "/me waves");
        assert!(r.not_written("n1", 10).is_none());
    }

    #[test]
    fn retry_only_where_it_can_help() {
        assert!(retry_helps("msg_duplicate") && retry_helps("msg_ratelimit") && retry_helps("msg_slowmode"));
        assert!(retry_helps("msg_followersonly") && retry_helps("msg_emoteonly"));
        assert!(!retry_helps("msg_banned") && !retry_helps("msg_timedout") && !retry_helps("msg_rejected"));
        assert!(!retry_helps("msg_warned"));
    }
}
