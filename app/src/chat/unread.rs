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
    /// V107-Mr Unseen mentions, for the count on the tab.
    pub mentions: u32,
}

/// V107-Mr A room's news: how lit, and how many mentions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct News {
    pub level: Unread,
    pub mentions: u32,
}

impl News {
    /// `self` with `more` on top.
    pub fn add(self, more: News) -> News {
        News { level: self.level.max(more.level), mentions: self.mentions.saturating_add(more.mentions) }
    }
}

/// What `events` add: chatters' messages count, history and our own don't.
pub fn news(events: &[RoomEvent]) -> News {
    let mut out = News::default();
    for e in events {
        let RoomEvent::Message { message: m } = e else { continue };
        if m.has(Flag::History) || m.has(Flag::Own) {
            continue;
        }
        if m.has(Flag::Mention) {
            out.level = Unread::Mention;
            out.mentions += 1;
        } else if m.sender.is_some() {
            out.level = out.level.max(Unread::Message);
        }
    }
    out
}

fn entry(key: &RoomKey, news: News) -> RoomUnread {
    RoomUnread {
        account: match &key.0 {
            Identity::Account(id) => Some(id.clone()),
            Identity::Anon => None,
        },
        login: key.1.clone(),
        level: news.level,
        mentions: news.mentions,
    }
}

/// Tells the main window, where the tab bar is.
pub fn emit(app: &AppHandle, key: &RoomKey, news: News) {
    let _ = app.emit_to("main", EVENT, [entry(key, news)]);
}

/// Every room with news, for a window that just opened.
pub fn snapshot(app: &AppHandle) -> Vec<RoomUnread> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter(|(_, e)| e.unread.level != Unread::Seen)
        .map(|(k, e)| entry(k, e.unread))
        .collect()
}

#[cfg(test)]
mod tests {
    use cyex_chat::Room;
    use cyex_chat::build::Me;
    use cyex_chat::irc::Line;
    use cyex_chat::rules::Rules;
    use cyex_chat::tokenize::NoEmotes;
    use cyex_settings::schema::Highlights;
    use std::sync::Arc;

    use super::*;

    fn news_of(raw: &str) -> News {
        let mut room = Room::new("chan", Some(Me::new("1", "me", "Me")));
        // V106-33 Your name lights up through its Highlights row (on by default).
        room.set_rules(Arc::new(Rules::new(&Highlights::default())));
        news(&room.handle_line(&Line::parse(raw).unwrap(), 0, &NoEmotes))
    }

    fn level_of(raw: &str) -> Unread {
        news_of(raw).level
    }

    #[test]
    fn chatters_light_up_mentions_more() {
        let msg = |user: &str, text: &str| format!("@id=x;user-id={user} :u!u@u.tmi.twitch.tv PRIVMSG #chan :{text}");
        assert_eq!(level_of(&msg("2", "hi")), Unread::Message);
        assert_eq!(level_of(&msg("2", "hi @me")), Unread::Mention);
        assert_eq!(level_of(&msg("1", "my own")), Unread::Seen);
        assert_eq!(level_of("@slow=0 :tmi.twitch.tv ROOMSTATE #chan"), Unread::Seen);
    }

    #[test]
    fn mentions_add_up() {
        let msg = |text: &str| format!("@id=x;user-id=2 :u!u@u.tmi.twitch.tv PRIVMSG #chan :{text}");
        assert_eq!(news_of(&msg("hi")).mentions, 0);
        let one = news_of(&msg("hi @me"));
        assert_eq!(one, News { level: Unread::Mention, mentions: 1 });
        let plain = News { level: Unread::Message, mentions: 0 };
        assert_eq!(plain.add(one).add(one), News { level: Unread::Mention, mentions: 2 });
        assert_eq!(one.add(plain), one);
    }
}
