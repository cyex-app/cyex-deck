//! C37, V102-18 Joins and leaves as chat lines, batched: everyone who joined within 10 s
//! of the first goes on one "joined: a, b, c" line, leaves on one "parted: …" line, each
//! updated in place wherever it is in the chat. Past 10 names the line ends in "+N more".

use std::collections::HashSet;
use std::sync::Arc;

use super::Room;
use crate::build;
use crate::model::{NoticeKind, RoomEvent};

/// How long a line keeps taking names, from its first.
const WINDOW_MS: i64 = 10_000;
/// Names written out; the rest are counted.
const SHOWN: usize = 10;

/// A joins or leaves line still taking names.
pub(super) struct Open {
    id: String,
    since_ms: i64,
    /// In order of arrival; only the first `SHOWN` are written.
    names: Vec<String>,
    seen: HashSet<String>,
}

/// The two lines being filled: joins and leaves.
#[derive(Default)]
pub(super) struct Batches {
    joined: Option<Open>,
    parted: Option<Open>,
}

/// "joined: ana, bob", "parted: ana, bob, … +3 more".
fn text(joined: bool, names: &[String]) -> String {
    let shown = names.len().min(SHOWN);
    let mut line = format!("{}: {}", if joined { "joined" } else { "parted" }, names[..shown].join(", "));
    if names.len() > shown {
        line.push_str(&format!(" +{} more", names.len() - shown));
    }
    line
}

impl Room {
    pub(super) fn presence(&mut self, login: String, joined: bool, now_ms: i64) -> Option<RoomEvent> {
        let slot = if joined { &mut self.presence.joined } else { &mut self.presence.parted };
        if let Some(open) = slot.as_mut().filter(|o| now_ms - o.since_ms < WINDOW_MS)
            && let Some(line) = self.buffer.iter_mut().rev().find(|m| m.id == open.id)
        {
            if !open.seen.insert(login.clone()) {
                return None;
            }
            open.names.push(login);
            let mut m = (**line).clone();
            m.notice.as_mut()?.text = text(joined, &open.names);
            let m = Arc::new(m);
            *line = m.clone();
            return Some(RoomEvent::Update { message: m });
        }
        let m = Arc::new(build::system(NoticeKind::Info, text(joined, std::slice::from_ref(&login)), now_ms));
        *slot = Some(Open {
            id: m.id.clone(),
            since_ms: now_ms,
            names: vec![login.clone()],
            seen: HashSet::from([login]),
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

    fn join(r: &mut Room, who: &str, now: i64) -> Vec<RoomEvent> {
        feed(r, &format!(":{who}!{who}@{who}.tmi.twitch.tv JOIN #chan"), now)
    }

    fn part(r: &mut Room, who: &str, now: i64) -> Vec<RoomEvent> {
        feed(r, &format!(":{who}!{who}@{who}.tmi.twitch.tv PART #chan"), now)
    }

    fn text(e: &RoomEvent) -> String {
        match e {
            RoomEvent::Message { message } | RoomEvent::Update { message } => {
                message.notice.as_ref().unwrap().text.clone()
            }
            other => panic!("{other:?}"),
        }
    }

    fn room() -> Room {
        Room::new("chan", Some(Me::new("1", "me", "Me")))
    }

    #[test]
    fn joins_and_parts_batch_within_ten_seconds() {
        let mut r = room();
        let first = join(&mut r, "ana", 0);
        assert!(matches!(first[0], RoomEvent::Message { .. }));
        assert_eq!(text(&first[0]), "joined: ana");
        let second = join(&mut r, "bob", 1000);
        assert!(matches!(second[0], RoomEvent::Update { .. }));
        assert_eq!(text(&second[0]), "joined: ana, bob");
        // Parts get their own line, and joins keep filling theirs alongside it.
        assert_eq!(text(&part(&mut r, "cy", 2000)[0]), "parted: cy");
        assert_eq!(text(&part(&mut r, "dan", 3000)[0]), "parted: cy, dan");
        assert_eq!(text(&join(&mut r, "eve", 4000)[0]), "joined: ana, bob, eve");
        // The same name again changes nothing.
        assert!(join(&mut r, "bob", 5000).is_empty());
        // 10 s after the first name, a new line starts.
        let late = join(&mut r, "fay", 10_000);
        assert!(matches!(late[0], RoomEvent::Message { .. }));
        assert_eq!(text(&late[0]), "joined: fay");
    }

    #[test]
    fn long_lists_end_in_a_count() {
        let mut r = room();
        let mut last = Vec::new();
        for i in 0..14 {
            last = join(&mut r, &format!("user{i}"), i);
        }
        assert_eq!(
            text(&last[0]),
            "joined: user0, user1, user2, user3, user4, user5, user6, user7, user8, user9 +4 more"
        );
    }

    #[test]
    fn lines_update_even_after_chat_moved_on() {
        let mut r = room();
        join(&mut r, "ana", 0);
        feed(&mut r, "@id=x;user-id=5;tmi-sent-ts=1 :zed!zed@zed.tmi.twitch.tv PRIVMSG #chan :hi", 500);
        let ev = join(&mut r, "bob", 1000);
        assert!(matches!(ev[0], RoomEvent::Update { .. }));
        assert_eq!(text(&ev[0]), "joined: ana, bob");
    }

    #[test]
    fn our_own_and_anonymous_joins_are_not_lines() {
        let mut r = room();
        assert!(join(&mut r, "me", 0).is_empty());
        assert!(part(&mut r, "justinfan123", 0).is_empty());
    }
}
