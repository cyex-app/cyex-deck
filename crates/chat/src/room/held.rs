//! V111-17 AutoMod's held messages in chat, for the channel's mods only: one line per held
//! message (keyed by its id, so a second source or a second mod account adds nothing), whose
//! state follows Twitch's decision (approved, denied, expired). Viewers' rooms get nothing.

use std::sync::Arc;

use super::Room;
use crate::build;
use crate::model::{HeldNote, HeldStatus, NoticeAction, NoticeKind, RoomEvent};

/// The line id of a held message's line.
pub fn line_id(message_id: &str) -> String {
    format!("held:{message_id}")
}

/// The line's text, for search, logs and copies (the UI draws the parts itself).
fn text(n: &HeldNote) -> String {
    format!("AutoMod held a message from {}: {}", n.user.name, n.text)
}

impl Room {
    /// A held message. `None` in a room that doesn't moderate, or when it is already shown
    /// and nothing changed. A repeat refreshes the text; a decision already shown stays.
    pub fn held_line(&mut self, mut note: HeldNote, now_ms: i64) -> Option<RoomEvent> {
        if !self.privileged() {
            return None;
        }
        let id = line_id(&note.message_id);
        if let Some(slot) = self.buffer.iter_mut().rev().find(|b| b.id == id) {
            let mut u = (**slot).clone();
            let n = u.notice.as_mut()?;
            let old = n.held.as_ref()?;
            note.status = old.status;
            note.by = old.by.clone();
            if *old == note {
                return None;
            }
            n.text = text(&note);
            n.links = vec![note.user.clone()];
            n.held = Some(note);
            *slot = Arc::new(u);
            return Some(RoomEvent::Update { message: slot.clone() });
        }
        let mut links = vec![note.user.clone()];
        self.colour(&mut links);
        note.user = links[0].clone();
        let mut m = build::system(NoticeKind::Moderation, text(&note), now_ms);
        m.id = id;
        if let Some(n) = m.notice.as_mut() {
            n.links = links;
            n.held = Some(note);
        }
        let m = Arc::new(m);
        self.push(m.clone());
        Some(RoomEvent::Message { message: m })
    }

    /// A mod decided on a held message, or it expired. `None` when the room doesn't show it,
    /// or already shows this decision. A later answer only fills in who decided.
    pub fn held_done(&mut self, message_id: &str, status: HeldStatus, by: Option<&str>) -> Option<RoomEvent> {
        let id = line_id(message_id);
        let slot = self.buffer.iter_mut().rev().find(|b| b.id == id)?;
        let mut u = (**slot).clone();
        let held = u.notice.as_mut()?.held.as_mut()?;
        let by = by.filter(|b| !b.is_empty()).map(str::to_owned);
        match held.status {
            HeldStatus::Pending => {
                held.status = status;
                held.by = by;
            }
            s if s == status && held.by.is_none() && by.is_some() => held.by = by,
            _ => return None,
        }
        *slot = Arc::new(u);
        Some(RoomEvent::Update { message: slot.clone() })
    }

    /// V111-17 A one-time line with a button, e.g. "AutoMod messages need a fresh sign-in."
    pub fn action_line(&mut self, kind: NoticeKind, text: &str, action: NoticeAction, now_ms: i64) -> RoomEvent {
        let mut m = build::system(kind, text, now_ms);
        if let Some(n) = m.notice.as_mut() {
            n.action = Some(action);
        }
        let m = Arc::new(m);
        self.push(m.clone());
        RoomEvent::Message { message: m }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irc::Line;
    use crate::model::NoticeUser;
    use crate::build::Me;
    use crate::tokenize::NoEmotes;

    fn room(modded: bool) -> Room {
        let mut r = Room::new("chan", Some(Me::new("1", "me", "Me")));
        let badges = if modded { "moderator/1" } else { "" };
        let raw = format!("@badges={badges};mod={} :tmi.twitch.tv USERSTATE #chan", u8::from(modded));
        r.handle_line(&Line::parse(&raw).unwrap(), 1, &NoEmotes);
        r
    }

    fn note(id: &str) -> HeldNote {
        HeldNote {
            message_id: id.into(),
            user: NoticeUser::new("Spammer", "spammer"),
            text: "buy followers".into(),
            reason: "automod:swearing:2".into(),
            status: HeldStatus::Pending,
            by: None,
        }
    }

    fn shown(r: &Room, id: &str) -> Option<HeldNote> {
        r.snapshot().messages.iter().find(|m| m.id == line_id(id)).and_then(|m| m.notice.as_ref()?.held.clone())
    }

    #[test]
    fn v111_17_viewers_get_nothing() {
        let mut r = room(false);
        assert!(r.held_line(note("m1"), 5).is_none());
        assert!(r.held_done("m1", HeldStatus::Allowed, Some("kuni")).is_none());
        assert!(r.snapshot().messages.iter().all(|m| m.notice.as_ref().is_none_or(|n| n.held.is_none())));
    }

    #[test]
    fn v111_17_held_then_approved_is_one_line() {
        let mut r = room(true);
        assert!(matches!(r.held_line(note("m1"), 5), Some(RoomEvent::Message { .. })));
        let text = r.snapshot().messages.last().unwrap().notice.as_ref().unwrap().text.clone();
        assert_eq!(text, "AutoMod held a message from Spammer: buy followers");
        // The same hold from Hermes, or another mod account's feed: nothing new.
        assert!(r.held_line(note("m1"), 6).is_none());
        assert!(matches!(r.held_done("m1", HeldStatus::Allowed, Some("Kuni")), Some(RoomEvent::Update { .. })));
        let h = shown(&r, "m1").unwrap();
        assert_eq!((h.status, h.by.as_deref()), (HeldStatus::Allowed, Some("Kuni")));
        // The second source's decision changes nothing; a late hold keeps the decision.
        assert!(r.held_done("m1", HeldStatus::Allowed, Some("Kuni")).is_none());
        assert!(r.held_done("m1", HeldStatus::Denied, Some("Other")).is_none());
        assert!(r.held_line(note("m1"), 7).is_none());
        assert_eq!(shown(&r, "m1").unwrap().status, HeldStatus::Allowed);
        let lines = r.snapshot().messages.iter().filter(|m| m.id == line_id("m1")).count();
        assert_eq!(lines, 1);
    }

    #[test]
    fn v111_17_denied_expired_and_who_comes_later() {
        let mut r = room(true);
        r.held_line(note("d"), 5);
        r.held_line(note("e"), 5);
        r.held_done("d", HeldStatus::Denied, None);
        assert_eq!(shown(&r, "d").unwrap().by, None);
        // Hermes names the mod after EventSub didn't.
        assert!(r.held_done("d", HeldStatus::Denied, Some("Sarah")).is_some());
        assert_eq!(shown(&r, "d").unwrap().by.as_deref(), Some("Sarah"));
        r.held_done("e", HeldStatus::Expired, None);
        assert_eq!(shown(&r, "e").unwrap().status, HeldStatus::Expired);
        // A decision on a message this room never showed: nothing.
        assert!(r.held_done("x", HeldStatus::Allowed, None).is_none());
    }

    #[test]
    fn v111_17_statuses_from_both_feeds() {
        for (s, want) in [
            ("Approved", HeldStatus::Allowed),
            ("ALLOWED", HeldStatus::Allowed),
            ("Denied", HeldStatus::Denied),
            ("DENIED", HeldStatus::Denied),
            ("Expired", HeldStatus::Expired),
            ("Invalid", HeldStatus::Expired),
        ] {
            assert_eq!(HeldStatus::of(s), want, "{s}");
        }
    }
}
