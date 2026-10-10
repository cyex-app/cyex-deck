//! What the whispers view gets (1.26).

use cyex_chat::model::{ChatMessage, NameColor};
use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperUser {
    pub id: String,
    pub login: String,
    pub name: String,
    pub color: NameColor,
    pub avatar: Option<String>,
}

/// One conversation in the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperThread {
    pub id: String,
    pub other: WhisperUser,
    pub unread: u32,
    pub archived: bool,
    pub muted: bool,
    /// Twitch flagged it as spam and nobody said otherwise.
    pub spam: bool,
    /// They can whisper you for 48 hours because you whispered them.
    pub temporary: bool,
    /// The newest line, as plain text.
    pub last_text: String,
    pub last_own: bool,
    /// Unix ms; 0 when there's nothing yet.
    #[ts(type = "number")]
    pub last_ts: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperState {
    /// The account whispers are for: the default one.
    pub account: Option<WhisperUser>,
    /// Past whispers load from Twitch (GQL sign-in). Without, only this run's show.
    pub history: bool,
    pub threads: Vec<WhisperThread>,
    pub more: bool,
    pub loading: bool,
    pub block_strangers: bool,
    /// Unread in threads that aren't muted or archived.
    pub unread: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperConversation {
    pub thread: WhisperThread,
    /// Oldest first.
    pub messages: Vec<ChatMessage>,
    /// For older lines; `None` at the start.
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperPage {
    /// Oldest first.
    pub messages: Vec<ChatMessage>,
    pub cursor: Option<String>,
}

/// A new line in a thread, sent or received.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperLine {
    pub thread_id: String,
    pub message: ChatMessage,
}

/// A change to a thread's options.
#[derive(Clone, Debug, Default, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperEdit {
    #[ts(optional)]
    pub archived: Option<bool>,
    #[ts(optional)]
    pub muted: Option<bool>,
    /// `false`: not spam; `true`: report as spam.
    #[ts(optional)]
    pub spam: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub revoke: Option<bool>,
}
