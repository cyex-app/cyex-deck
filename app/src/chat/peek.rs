//! S5 What plugins may read of open chats, by channel: room state and recent lines.
//! A channel open as several identities answers for the one in a view, signed in first.

use std::sync::Arc;

use cyex_chat::model::{ChatMessage, RoomState};
use tauri::{AppHandle, Manager};

use super::{Chat, Identity, Inner, RoomKey};

pub(super) fn pick(inner: &Inner, login: &str) -> Option<RoomKey> {
    let shown = |k: &RoomKey| inner.views.values().any(|(v, _)| v == k);
    let mut keys: Vec<&RoomKey> = inner.rooms.keys().filter(|k| k.1 == login).collect();
    keys.sort_by_key(|k| (!shown(k), k.0 == Identity::Anon));
    keys.first().map(|k| (*k).clone())
}

/// The room's key and state.
pub fn state(app: &AppHandle, login: &str) -> Option<(RoomKey, RoomState)> {
    let chat = app.try_state::<Chat>()?;
    let inner = chat.lock();
    let key = pick(&inner, login)?;
    let state = inner.rooms.get(&key)?.room.state.clone();
    Some((key, state))
}

/// The room a channel's lines go to.
pub fn room(app: &AppHandle, login: &str) -> Option<RoomKey> {
    let chat = app.try_state::<Chat>()?;
    let inner = chat.lock();
    pick(&inner, login)
}

/// The last `n` lines, oldest first.
pub fn lines(app: &AppHandle, login: &str, n: usize) -> Vec<Arc<ChatMessage>> {
    let Some(chat) = app.try_state::<Chat>() else {
        return Vec::new();
    };
    let inner = chat.lock();
    let Some(key) = pick(&inner, login) else {
        return Vec::new();
    };
    inner
        .rooms
        .get(&key)
        .map(|e| e.room.snapshot_tail(n).messages)
        .unwrap_or_default()
}

/// A message a view shows, and its channel.
pub fn message(app: &AppHandle, view: u32, id: &str) -> Option<(String, Arc<ChatMessage>)> {
    let chat = app.try_state::<Chat>()?;
    let inner = chat.lock();
    let key = inner.views.get(&view)?.0.clone();
    let m = inner.rooms.get(&key)?.room.message(id)?;
    Some((key.1, m))
}

/// S7 A line by its id in the room a channel's lines go to.
pub fn find(app: &AppHandle, login: &str, id: &str) -> Option<Arc<ChatMessage>> {
    let chat = app.try_state::<Chat>()?;
    let inner = chat.lock();
    let key = pick(&inner, login)?;
    inner.rooms.get(&key)?.room.message(id)
}

/// The channel a view shows.
pub fn channel_of(app: &AppHandle, view: u32) -> Option<String> {
    let chat = app.try_state::<Chat>()?;
    let inner = chat.lock();
    inner.views.get(&view).map(|(k, _)| k.1.clone())
}
