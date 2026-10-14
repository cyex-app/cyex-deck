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
}

/// One entry per signed-in account, zeros included.
pub fn counts(app: &AppHandle) -> Vec<ConnectionCount> {
    let rooms = crate::chat::account_rooms(app);
    crate::auth::accounts(app)
        .list
        .iter()
        .map(|a| {
            let (joined, refused) = rooms.get(&a.user_id).cloned().unwrap_or_default();
            ConnectionCount { user_id: a.user_id.clone(), joined, refused }
        })
        .collect()
}

pub fn notify(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, counts(app));
}
