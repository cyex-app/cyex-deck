//! V106-07 Whispers feed: whispers the default account receives this session. Emptied when
//! the default account changes (Q4).

use cyex_chat::model::ChatMessage;
use cyex_settings::schema::FeedKind;
use cyex_twitch::gql::whispers as api;
use serde::Serialize;
use tauri::AppHandle;
use ts_rs::TS;

use super::{FeedEntry, push};

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WhisperEntry {
    /// The message id.
    pub key: String,
    #[ts(type = "number")]
    pub ts: i64,
    pub thread_id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    pub text: String,
}

pub fn received(app: &AppHandle, thread_id: &str, from: &api::Person, line: &ChatMessage) {
    let entry = WhisperEntry {
        key: line.id.clone(),
        ts: line.ts,
        thread_id: thread_id.to_owned(),
        login: from.login.clone(),
        name: if from.display.is_empty() {
            from.login.clone()
        } else {
            from.display.clone()
        },
        avatar: from.avatar.clone(),
        text: crate::whispers::preview_text(line),
    };
    push(app, FeedKind::Whispers, FeedEntry::Whisper(entry), |_| false);
}

pub fn clear(app: &AppHandle) {
    super::clear(app, FeedKind::Whispers);
}
