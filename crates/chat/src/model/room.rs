//! Room state and the events streamed to chat views.

use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;

use super::{Badge, ChatMessage, Gone, ModRole};

/// 1.9 A pinned message.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Pinned {
    pub id: String,
    /// Who pinned it.
    pub by: Option<String>,
    /// V107-P5 Their login, for the usercard.
    pub by_login: Option<String>,
    /// V106-19 RFC 3339, when the pin began; the timer bar counts from it.
    pub starts_at: Option<String>,
    /// RFC 3339; `None` = until unpinned.
    pub ends_at: Option<String>,
    /// Paid pin (Hype Chat).
    pub paid: bool,
    pub message: ChatMessage,
}

/// ROOMSTATE plus what USERSTATE says about us.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomState {
    pub channel_id: Option<String>,
    /// Slow mode in seconds, 0 = off.
    pub slow: u32,
    /// Followers-only minimum follow age in minutes. `None` = off.
    pub followers_only: Option<u32>,
    pub subs_only: bool,
    pub emote_only: bool,
    /// r9k: unique messages only.
    pub unique: bool,
    /// `None` when reading anonymously.
    pub own: Option<SelfState>,
    /// C6 Unix ms until we can talk again (slow mode, timeout).
    #[ts(type = "number | null")]
    pub wait_until: Option<i64>,
}

/// Our account in a room: badges from USERSTATE (or our own lines), and the role the
/// one check reads (V102-21).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SelfState {
    pub login: String,
    pub badges: Vec<Badge>,
    /// 1.57 `None`: no mod rights here.
    pub role: Option<ModRole>,
    pub vip: bool,
}

impl SelfState {
    /// From badges and USERSTATE's `mod` tag. `owner`: the channel is the account's own.
    pub fn new(login: &str, badges: Vec<Badge>, mod_tag: bool, owner: bool) -> Self {
        let role = ModRole::of(badges.iter().map(|b| b.set.as_str()), mod_tag, owner);
        let vip = badges.iter().any(|b| b.set == "vip");
        Self {
            login: login.to_owned(),
            badges,
            role,
            vip,
        }
    }

    pub fn broadcaster(&self) -> bool {
        self.role == Some(ModRole::Broadcaster)
    }

    /// Broadcaster, lead moderator or moderator: free from slow mode.
    pub fn privileged(&self) -> bool {
        self.role.is_some()
    }

    /// 100 messages per 30 s instead of 20.
    pub fn fast_lane(&self) -> bool {
        self.privileged() || self.vip
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ConnStatus {
    #[default]
    Connecting,
    Joined,
    /// Lost the connection; trying again.
    Reconnecting,
    /// Twitch refused the token. Waits for a new one.
    AuthFailed,
    /// The channel is suspended or doesn't exist.
    Unavailable,
    /// V105-05 Twitch didn't let us in: no answer to our JOIN within 20 s, or a notice instead.
    /// Usually too many chats joined.
    Refused,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum RoomEvent {
    Message {
        #[ts(as = "ChatMessage")]
        message: Arc<ChatMessage>,
    },
    /// Replaces the message with the same id, e.g. merged gift subs (C16).
    Update {
        #[ts(as = "ChatMessage")]
        message: Arc<ChatMessage>,
    },
    /// C14 One message deleted. V107-R34 `gone`: why, for its note; none for a plugin's hide.
    Delete {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        gone: Option<Gone>,
    },
    /// C14 All of a user's messages (timeout or ban). V107-R34 Sent again when a mod feed
    /// names the mod.
    ClearUser {
        #[serde(rename = "userId")]
        user_id: String,
        gone: Gone,
    },
    /// S8 A mod cleared the chat; its notice line comes too.
    Cleared,
    /// S14 A plugin took back its own line.
    Remove { id: String },
    /// S14 A plugin emptied the chat on the user's screen, not on Twitch.
    Wipe,
    /// 1.9 The pinned message changed or went away.
    Pinned {
        #[ts(as = "Option<Pinned>")]
        pin: Option<Box<Pinned>>,
    },
    /// C1 Older messages, shown above the rest.
    History {
        #[ts(as = "Vec<ChatMessage>")]
        messages: Vec<Arc<ChatMessage>>,
    },
    /// C24 Messages missed while disconnected, to put in by time.
    Gap {
        #[ts(as = "Vec<ChatMessage>")]
        messages: Vec<Arc<ChatMessage>>,
    },
    State { state: RoomState },
    Status { status: ConnStatus },
}
