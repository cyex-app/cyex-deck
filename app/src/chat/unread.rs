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
    /// V108-N11 `#rrggbb` of the latest highlight rule that matched, for the tab's mark.
    pub color: Option<String>,
}

/// V107-Mr A room's news: how lit, and how many mentions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct News {
    pub level: Unread,
    pub mentions: u32,
    /// V108-N11 The latest matched rule's colour as `0xRRGGBB`; the newer one wins.
    pub color: Option<u32>,
}

/// V108-N11 A rule's `#rrggbb` as a number; anything else (empty, short, named) is none.
pub fn rule_color(hex: &str) -> Option<u32> {
    let digits = hex.strip_prefix('#')?;
    (digits.len() == 6).then(|| u32::from_str_radix(digits, 16).ok()).flatten()
}

impl News {
    /// `self` with `more` on top.
    pub fn add(self, more: News) -> News {
        News {
            level: self.level.max(more.level),
            mentions: self.mentions.saturating_add(more.mentions),
            color: more.color.or(self.color),
        }
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
        // V108-N11 The newest rule colour on a line that lights the tab.
        let lit = m.has(Flag::Mention) || m.sender.is_some();
        if lit && let Some(c) = m.meta.highlight.as_ref().and_then(|h| h.color.as_deref()).and_then(rule_color) {
            out.color = Some(c);
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
        color: news.color.map(|c| format!("#{c:06x}")),
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
        assert_eq!(one, News { level: Unread::Mention, mentions: 1, color: None });
        let plain = News { level: Unread::Message, mentions: 0, color: None };
        assert_eq!(plain.add(one).add(one), News { level: Unread::Mention, mentions: 2, color: None });
        assert_eq!(one.add(plain), one);
    }

    /// V108-N11 Only `#rrggbb` counts; the newer colour wins and a plain line keeps the old one.
    #[test]
    fn rule_colors_for_the_tab_mark() {
        assert_eq!(rule_color("#ff8800"), Some(0xff8800));
        assert_eq!(rule_color("#FF8800"), Some(0xff8800));
        for bad in ["", "ff8800", "#ff88", "#ff88000", "#gg0000", "red"] {
            assert_eq!(rule_color(bad), None, "{bad}");
        }
        let a = News { level: Unread::Mention, mentions: 1, color: Some(0x111111) };
        let b = News { level: Unread::Message, mentions: 0, color: Some(0x222222) };
        let none = News { level: Unread::Message, mentions: 0, color: None };
        assert_eq!(a.add(b).color, Some(0x222222));
        assert_eq!(a.add(none).color, Some(0x111111));
        assert_eq!(None, News::default().add(none).color);
    }

    #[test]
    fn a_colored_rule_marks_the_news() {
        let mut room = Room::new("chan", Some(Me::new("1", "me", "Me")));
        let h = Highlights {
            users: vec![cyex_settings::schema::UserRule {
                enabled: true,
                login: "two".into(),
                alert: cyex_settings::schema::Alert { color: "#ff8800".into(), ..Default::default() },
                ..Default::default()
            }],
            ..Highlights::default()
        };
        room.set_rules(Arc::new(Rules::new(&h)));
        let raw = "@id=x;user-id=2 :two!two@two.tmi.twitch.tv PRIVMSG #chan :hi";
        let n = news(&room.handle_line(&Line::parse(raw).unwrap(), 0, &NoEmotes));
        assert_eq!((n.level, n.color), (Unread::Message, Some(0xff8800)));
        let other = "@id=y;user-id=3 :three!three@three.tmi.twitch.tv PRIVMSG #chan :hi";
        assert_eq!(news(&room.handle_line(&Line::parse(other).unwrap(), 0, &NoEmotes)).color, None);
    }
}
