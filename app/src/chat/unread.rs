//! U9 Rooms with news nobody has seen: their tabs light up, more for mentions. A room
//! counts as seen while a view shows it.

use cyex_chat::model::{Flag, RoomEvent};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::{Chat, Identity, RoomKey};

const EVENT: &str = "chat:unread";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Unread {
    #[default]
    Seen,
    Message,
    Mention,
}

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomUnread {
    /// `None` when read anonymously.
    pub account: Option<String>,
    pub login: String,
    pub level: Unread,
}

/// What `events` add: chatters' messages count, history and our own don't.
pub fn level(events: &[RoomEvent]) -> Unread {
    let mut level = Unread::Seen;
    for e in events {
        let RoomEvent::Message { message: m } = e else { continue };
        if m.has(Flag::History) || m.has(Flag::Own) {
            continue;
        }
        if m.has(Flag::Mention) {
            return Unread::Mention;
        }
        if m.sender.is_some() {
            level = Unread::Message;
        }
    }
    level
}

fn entry(key: &RoomKey, level: Unread) -> RoomUnread {
    RoomUnread {
        account: match &key.0 {
            Identity::Account(id) => Some(id.clone()),
            Identity::Anon => None,
        },
        login: key.1.clone(),
        level,
    }
}

/// Tells the main window, where the tab bar is.
pub fn emit(app: &AppHandle, key: &RoomKey, level: Unread) {
    let _ = app.emit_to("main", EVENT, [entry(key, level)]);
}

/// Every room with news, for a window that just opened.
pub fn snapshot(app: &AppHandle) -> Vec<RoomUnread> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter(|(_, e)| e.unread != Unread::Seen)
        .map(|(k, e)| entry(k, e.unread))
        .collect()
}

#[cfg(test)]
mod tests {
    use cyex_chat::Room;
    use cyex_chat::build::Me;
    use cyex_chat::irc::Line;
    use cyex_chat::tokenize::NoEmotes;

    use super::*;

    fn level_of(raw: &str) -> Unread {
        let mut room = Room::new("chan", Some(Me::new("1", "me", "Me")));
        level(&room.handle_line(&Line::parse(raw).unwrap(), 0, &NoEmotes))
    }

    #[test]
    fn chatters_light_up_mentions_more() {
        let msg = |user: &str, text: &str| format!("@id=x;user-id={user} :u!u@u.tmi.twitch.tv PRIVMSG #chan :{text}");
        assert_eq!(level_of(&msg("2", "hi")), Unread::Message);
        assert_eq!(level_of(&msg("2", "hi @me")), Unread::Mention);
        assert_eq!(level_of(&msg("1", "my own")), Unread::Seen);
        assert_eq!(level_of("@slow=0 :tmi.twitch.tv ROOMSTATE #chan"), Unread::Seen);
    }
}
