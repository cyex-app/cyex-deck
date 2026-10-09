//! Room state and the events streamed to chat views.

use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;

use super::{Badge, ChatMessage};

/// 1.9 A pinned message.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Pinned {
    pub id: String,
    /// Who pinned it.
    pub by: Option<String>,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SelfState {
    pub login: String,
    pub badges: Vec<Badge>,
    pub broadcaster: bool,
    pub moderator: bool,
    pub vip: bool,
}

impl SelfState {
    /// Broadcaster or moderator: free from slow mode and the join budget.
    pub fn privileged(&self) -> bool {
        self.broadcaster || self.moderator
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
    /// The account's 100-room budget is full (D7).
    BudgetFull,
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
    /// C14 One message deleted.
    Delete { id: String },
    /// C14 All of a user's messages (timeout or ban).
    ClearUser {
        #[serde(rename = "userId")]
        user_id: String,
    },
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
    State { state: RoomState },
    Status { status: ConnStatus },
}
