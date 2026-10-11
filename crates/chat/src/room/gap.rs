//! C24 After a reconnect, the messages missed meanwhile come from the history service
//! and go in by time (Chatterino's way: ask for the span, at most 10 a second).

use std::collections::HashSet;
use std::sync::Arc;

use super::Room;
use crate::build::{self, Ctx, Incoming};
use crate::irc::Line;
use crate::model::{ChatMessage, ConnStatus, Flag};
use crate::tokenize::Emotes;

/// The history service keeps no more than this.
const MAX_LINES: i64 = 800;

impl Room {
    /// The link dropped. Only the first drop counts until we're back.
    pub fn lost(&mut self, now_ms: i64) {
        if self.lost_at.is_none() && self.status == ConnStatus::Joined {
            self.lost_at = Some(now_ms);
        }
    }

    /// Back after a drop: the span to ask for, as (after ms, before ms, lines).
    pub fn take_gap(&mut self, now_ms: i64) -> Option<(i64, i64, u32)> {
        let lost = self.lost_at.take()?;
        let secs = (now_ms - lost).max(0) / 1000 + 1;
        Some((lost, now_ms, (secs * 10).min(MAX_LINES) as u32))
    }

    /// Puts in what was missed, by time. Lines already here are skipped.
    pub fn fill_gap(&mut self, lines: &[String], now_ms: i64, emotes: &dyn Emotes) -> Vec<Arc<ChatMessage>> {
        let known: HashSet<String> = self.buffer.iter().map(|m| m.id.clone()).collect();
        let mut added: Vec<ChatMessage> = Vec::new();
        for raw in lines {
            let Some(line) = Line::parse(raw) else { continue };
            let incoming = {
                let mut ctx = Ctx::new(self.me.as_ref(), &mut self.colors, now_ms);
                ctx.emotes = emotes;
                build::build(&line, &mut ctx)
            };
            match incoming {
                Some(
                    Incoming::Message(mut m)
                    | Incoming::Clear { notice: mut m }
                    | Incoming::Gift { message: mut m, .. },
                ) => {
                    m.flags.retain(|f| *f != Flag::History);
                    if !known.contains(&m.id) && self.screen_old(&mut m) {
                        added.push(m);
                    }
                }
                Some(Incoming::Delete { id }) => {
                    if let Some(m) = added.iter_mut().find(|m| m.id == id) {
                        m.set(Flag::Deleted);
                    }
                }
                Some(Incoming::ClearUser { user_id, .. }) => {
                    for m in added
                        .iter_mut()
                        .filter(|m| m.notice.is_none() && m.sender_id() == Some(user_id.as_str()))
                    {
                        m.set(Flag::Deleted);
                    }
                }
                _ => {}
            }
        }
        let added: Vec<Arc<ChatMessage>> = added.into_iter().map(Arc::new).collect();
        for m in &added {
            // Before the first line that's newer, so above "Reconnected" (V101-32).
            let at = self.buffer.iter().position(|b| b.ts > m.ts).unwrap_or(self.buffer.len());
            self.buffer.insert(at, m.clone());
        }
        self.trim();
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenize::NoEmotes;

    fn line(id: &str, ts: i64) -> String {
        format!("@id={id};user-id=7;tmi-sent-ts={ts};historical=1 :u!u@u PRIVMSG #chan :msg {id}")
    }

    #[test]
    fn gap_fills_in_order() {
        let mut r = Room::new("chan", None);
        r.set_status(ConnStatus::Joined, 500);
        r.handle_line(
            &Line::parse(&line("a", 1_000).replace(";historical=1", "")).unwrap(),
            1_000,
            &NoEmotes,
        );
        r.lost(2_000);
        r.set_status(ConnStatus::Reconnecting, 2_000);
        r.lost(3_000);
        r.set_status(ConnStatus::Joined, 8_500);
        r.handle_line(
            &Line::parse(&line("d", 9_000).replace(";historical=1", "")).unwrap(),
            9_000,
            &NoEmotes,
        );
        assert_eq!(r.take_gap(8_500), Some((2_000, 8_500, 70)));
        assert_eq!(r.take_gap(8_600), None);
        let added = r.fill_gap(
            &[line("a", 1_000), line("b", 2_500), line("c", 3_000)],
            9_000,
            &NoEmotes,
        );
        assert_eq!(added.len(), 2);
        assert!(!added[0].has(Flag::History));
        // V101-32 What was missed sits between the drop and the way back.
        let ids: Vec<_> = r
            .snapshot()
            .messages
            .iter()
            .map(|m| m.notice.as_ref().filter(|_| m.sender.is_none()).map_or(m.id.clone(), |n| n.text.clone()))
            .collect();
        assert_eq!(ids, ["Joined chat", "a", "Disconnected", "b", "c", "Reconnected", "d"]);
    }
}
