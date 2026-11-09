//! Connection counter (1.13, 1.23, D7): the chats each account has open, and the ones
//! Twitch refused (V105-05). Read from the chat rooms; no limit of our own.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

const CHANGED_EVENT: &str = "connections:changed";

/// One account's numbers for the counter in the top bar and the account manager.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectionCount {
    pub user_id: String,
    /// Chats it has open, mod channels too, refused ones not.
    pub joined: u32,
    /// V105-05 Channels (logins) Twitch didn't let it join, while their chat is open.
    pub refused: Vec<String>,
    /// The logins of those chats, sorted (the joined-chats list).
    pub channels: Vec<String>,
}

/// One entry per signed-in account, zeros included.
pub fn counts(app: &AppHandle) -> Vec<ConnectionCount> {
    let rooms = crate::chat::account_rooms(app);
    crate::auth::accounts(app)
        .list
        .iter()
        .map(|a| {
            let (channels, refused) = rooms.get(&a.user_id).cloned().unwrap_or_default();
            ConnectionCount { user_id: a.user_id.clone(), joined: channels.len() as u32, refused, channels }
        })
        .collect()
}

pub fn notify(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, counts(app));
}

/// V110-U65 One socket's reconnects in the last hour, for the About page's table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkHealth {
    pub label: String,
    pub count: u32,
    /// ms since the epoch; as a number in TS.
    #[ts(type = "number")]
    pub last_at: u64,
    pub last_error: String,
}

/// Every socket that has dropped since start, latest first.
pub fn health() -> Vec<LinkHealth> {
    cyex_core::health::snapshot()
        .into_iter()
        .map(|l| LinkHealth { label: l.label, count: l.count, last_at: l.last_at, last_error: l.last_error })
        .collect()
}

