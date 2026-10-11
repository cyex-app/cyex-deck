//! C37 Joins and leaves as chat lines. Names merge into one line per kind while it's the
//! newest line, up to 10 names or 30 s, like Chatty's compact lines.

use std::sync::Arc;

use super::Room;
use crate::build;
use crate::model::{NoticeKind, RoomEvent};

const MAX_NAMES: usize = 10;
const MERGE_MS: i64 = 30_000;

/// The joins or leaves line still taking names.
pub(super) struct Open {
    id: String,
    joined: bool,
    names: Vec<String>,
    since_ms: i64,
}

/// "ana joined", "Joined: ana, bob".
fn text(joined: bool, names: &[String]) -> String {
    match (names, joined) {
        ([one], true) => format!("{one} joined"),
        ([one], false) => format!("{one} left"),
        (_, true) => format!("Joined: {}", names.join(", ")),
        (_, false) => format!("Left: {}", names.join(", ")),
    }
}

impl Room {
    pub(super) fn presence(&mut self, login: String, joined: bool, now_ms: i64) -> Option<RoomEvent> {
        if let Some(open) = self.presence.as_mut()
            && open.joined == joined
            && open.names.len() < MAX_NAMES
            && now_ms - open.since_ms < MERGE_MS
            && let Some(slot) = self.buffer.back_mut().filter(|m| m.id == open.id)
        {
            if open.names.contains(&login) {
                return None;
            }
            open.names.push(login);
            let mut m = (**slot).clone();
            m.notice.as_mut()?.text = text(joined, &open.names);
            let m = Arc::new(m);
            *slot = m.clone();
            return Some(RoomEvent::Update { message: m });
        }
        let line = text(joined, std::slice::from_ref(&login));
        let m = Arc::new(build::system(NoticeKind::Info, line, now_ms));
        self.presence = Some(Open {
            id: m.id.clone(),
            joined,
            names: vec![login],
            since_ms: now_ms,
        });
        self.push(m.clone());
        Some(RoomEvent::Message { message: m })
    }
}

#[cfg(test)]
mod tests {
    use crate::build::Me;
    use crate::irc::Line;
    use crate::model::RoomEvent;
    use crate::room::Room;
    use crate::tokenize::NoEmotes;

    fn feed(r: &mut Room, raw: &str, now: i64) -> Vec<RoomEvent> {
        r.handle_line(&Line::parse(raw).unwrap(), now, &NoEmotes)
    }

    fn text(e: &RoomEvent) -> String {
        match e {
            RoomEvent::Message { message } | RoomEvent::Update { message } => {
                message.notice.as_ref().unwrap().text.clone()
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn joins_merge_while_they_are_the_newest_line() {
        let mut r = Room::new("chan", Some(Me::new("1", "me", "Me")));
        let first = feed(&mut r, ":ana!ana@ana.tmi.twitch.tv JOIN #chan", 0);
        assert_eq!(text(&first[0]), "ana joined");
        let second = feed(&mut r, ":bob!bob@bob.tmi.twitch.tv JOIN #chan", 1000);
        assert!(matches!(second[0], RoomEvent::Update { .. }));
        assert_eq!(text(&second[0]), "Joined: ana, bob");
        // The same name again changes nothing; a leave starts its own line.
        assert!(feed(&mut r, ":bob!bob@bob.tmi.twitch.tv JOIN #chan", 2000).is_empty());
        assert_eq!(
            text(&feed(&mut r, ":cy!cy@cy.tmi.twitch.tv PART #chan", 3000)[0]),
            "cy left"
        );
        // Joins after it, or after 30 s, start over.
        assert_eq!(
            text(&feed(&mut r, ":dan!dan@dan.tmi.twitch.tv JOIN #chan", 4000)[0]),
            "dan joined"
        );
        assert_eq!(
            text(&feed(&mut r, ":eve!eve@eve.tmi.twitch.tv JOIN #chan", 40_000)[0]),
            "eve joined"
        );
    }

    #[test]
    fn our_own_and_anonymous_joins_are_not_lines() {
        let mut r = Room::new("chan", Some(Me::new("1", "me", "Me")));
        assert!(feed(&mut r, ":me!me@me.tmi.twitch.tv JOIN #chan", 0).is_empty());
        assert!(
            feed(
                &mut r,
                ":justinfan123!justinfan123@justinfan123.tmi.twitch.tv PART #chan",
                0
            )
            .is_empty()
        );
    }
}
