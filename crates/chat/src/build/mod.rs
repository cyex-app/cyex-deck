//! IRC lines → what a room needs: messages, deletions, state. Connection-level commands
//! (PING, RECONNECT, 001) are handled by the connection and never get here.

mod moderation;
mod privmsg;
pub mod replay;
pub mod sender;
mod state;
mod usernotice;
pub mod whisper;

use std::sync::atomic::{AtomicU64, Ordering};

pub use moderation::duration_text;
pub use sender::Colors;
pub use state::{RoomStateUpdate, UserState};

use crate::irc::Line;
use crate::model::{ChatMessage, Meta, Notice, NoticeKind};
use crate::tokenize::{Emotes, NoEmotes};

/// The signed-in account reading this room.
#[derive(Clone, Debug)]
pub struct Me {
    pub user_id: String,
    pub login: String,
    /// Lowercase login and display name, for mention detection.
    pub names: Vec<String>,
}

impl Me {
    pub fn new(user_id: &str, login: &str, display: &str) -> Self {
        let mut names = vec![login.to_lowercase()];
        let display = display.to_lowercase();
        if !display.is_empty() && display != names[0] {
            names.push(display);
        }
        Self {
            user_id: user_id.to_owned(),
            login: login.to_owned(),
            names,
        }
    }
}

pub struct Ctx<'a> {
    pub me: Option<&'a Me>,
    pub colors: &'a mut Colors,
    /// Emotes matched by name (phase 7 providers).
    pub emotes: &'a dyn Emotes,
    /// Unix ms, used when a line has no timestamp.
    pub now_ms: i64,
}

impl<'a> Ctx<'a> {
    pub fn new(me: Option<&'a Me>, colors: &'a mut Colors, now_ms: i64) -> Self {
        Self {
            me,
            colors,
            emotes: &NoEmotes,
            now_ms,
        }
    }

    pub fn me_names(&self) -> &'a [String] {
        self.me.map_or(&[], |m| m.names.as_slice())
    }

    pub fn local_id(&self) -> String {
        local_id()
    }
}

/// Id for lines Twitch didn't give one.
pub fn local_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    format!("local-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// C16 How a gift notice relates to others.
#[derive(Debug, PartialEq, Eq)]
pub enum GiftLink {
    /// "X is gifting 5 subs": the gifts that follow name its origin id.
    Mass { origin: String, count: usize },
    /// One gifted sub; `community` is the origin id of its mass gift, if any.
    Single { community: Option<String>, recipient: String },
}

#[derive(Debug)]
pub enum Incoming {
    Message(ChatMessage),
    /// C16 A gift notice that may merge into an earlier mass gift.
    Gift { message: ChatMessage, gift: GiftLink },
    /// CLEARMSG: one message deleted.
    Delete { id: String },
    /// CLEARCHAT with a target: timeout or ban.
    ClearUser {
        user_id: String,
        /// Timeout length; `None` for a ban.
        secs: Option<u64>,
        notice: ChatMessage,
    },
    /// CLEARCHAT without a target.
    Clear { notice: ChatMessage },
    RoomState(RoomStateUpdate),
    UserState(UserState),
    /// NOTICE from Twitch, e.g. why a message wasn't sent.
    Notice { msg_id: String, notice: ChatMessage },
    /// C37 Someone else opened or left the chat (JOIN, PART with `twitch.tv/membership`).
    Presence { login: String, joined: bool },
}

/// `None` for lines a room doesn't care about.
pub fn build(line: &Line<'_>, ctx: &mut Ctx<'_>) -> Option<Incoming> {
    match line.command {
        "PRIVMSG" => privmsg::privmsg(line, ctx).map(Incoming::Message),
        "USERNOTICE" => usernotice::usernotice(line, ctx),
        "CLEARCHAT" => Some(moderation::clearchat(line, ctx)),
        "CLEARMSG" => moderation::clearmsg(line),
        "NOTICE" => Some(moderation::notice(line, ctx)),
        "ROOMSTATE" => Some(Incoming::RoomState(RoomStateUpdate::from_line(line))),
        "USERSTATE" => Some(Incoming::UserState(UserState::from_line(line))),
        "JOIN" | "PART" => presence(line, ctx),
        _ => None,
    }
}

/// C37 Signed-in people only: anonymous readers (`justinfan…`) and we ourselves aren't news.
fn presence(line: &Line<'_>, ctx: &Ctx<'_>) -> Option<Incoming> {
    let login = line.nick();
    let me = ctx.me.is_some_and(|m| m.login.eq_ignore_ascii_case(login));
    if login.is_empty() || me || login.starts_with("justinfan") {
        return None;
    }
    Some(Incoming::Presence {
        login: login.to_ascii_lowercase(),
        joined: line.command == "JOIN",
    })
}

/// `tmi-sent-ts`, else the history service's receive time, else now.
fn timestamp(line: &Line<'_>, now_ms: i64) -> i64 {
    line.tag_num("tmi-sent-ts")
        .or_else(|| line.tag_num("rm-received-ts"))
        .unwrap_or(now_ms)
}

/// A system line with no sender.
pub fn system(kind: NoticeKind, text: impl Into<String>, ts: i64) -> ChatMessage {
    ChatMessage {
        id: local_id(),
        ts,
        sender: None,
        body: Vec::new(),
        notice: Some(Notice::new(kind, text)),
        reply: None,
        reward: None,
        source: None,
        flags: Vec::new(),
        meta: Meta::default(),
    }
}
