//! A chat line as the UI gets it (ARCHITECTURE §4).

use serde::Serialize;
use ts_rs::TS;

use super::{Fragment, Notice};

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatMessage {
    /// Twitch's message id, or a local one (`local-…`) for system lines.
    pub id: String,
    /// Unix milliseconds.
    #[ts(type = "number")]
    pub ts: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sender: Option<Sender>,
    pub body: Vec<Fragment>,
    /// Subs, raids, announcements, timeouts and other system lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub notice: Option<Notice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reply: Option<Reply>,
    /// 1.9 Channel point reward redeemed with this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reward: Option<RewardRef>,
    /// Shared chat: the channel this message was sent in.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source: Option<SharedSource>,
    pub flags: Vec<Flag>,
}

impl ChatMessage {
    pub fn has(&self, flag: Flag) -> bool {
        self.flags.contains(&flag)
    }

    pub fn set(&mut self, flag: Flag) {
        if !self.has(flag) {
            self.flags.push(flag);
        }
    }

    pub fn sender_id(&self) -> Option<&str> {
        self.sender.as_ref().map(|s| s.id.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sender {
    pub id: String,
    pub login: String,
    /// May be localized (C20).
    pub display: String,
    /// C8 Name color made readable on each theme, `#rrggbb`.
    pub color: NameColor,
    pub badges: Vec<Badge>,
}

impl Sender {
    /// C20 The display name isn't just the login with different casing.
    pub fn localized(&self) -> bool {
        !self.display.eq_ignore_ascii_case(&self.login)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NameColor {
    pub dark: String,
    pub light: String,
}

/// Twitch badge `set/version`. Images come from the room's badge catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Badge {
    pub set: String,
    pub version: String,
    /// From `badge-info`: sub months, prediction choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub info: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Reply {
    pub parent_id: String,
    pub parent_login: String,
    pub parent_display: String,
    pub parent_body: String,
    /// The first message of the thread.
    pub thread_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RewardRef {
    /// Custom reward id, or the automatic reward's type.
    pub id: String,
    /// Filled in from the channel's reward list when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cost: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SharedSource {
    pub room_id: String,
    pub badges: Vec<Badge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Flag {
    /// `/me`
    Action,
    /// Mentions or replies to the signed-in account.
    Mention,
    /// Sent by the signed-in account.
    Own,
    /// C1 Loaded from history when joining.
    History,
    /// C14 Deleted by a mod, timed out or banned.
    Deleted,
    FirstMessage,
    ReturningChatter,
    /// 1.9 "Highlight my message" reward.
    Highlighted,
    /// "Send a message in sub-only mode" reward.
    SkipSubsMode,
    /// 1.42 Power-ups.
    Gigantified,
    Animated,
    /// Paid pinned message (Hype Chat).
    Paid,
}
