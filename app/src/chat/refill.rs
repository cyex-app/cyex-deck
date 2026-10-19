//! V107-M1 Rooms no view shows keep a small buffer (`cyex_chat::BACKGROUND`). Shown again,
//! a room shows what it kept at once, then gets its older lines back from the history
//! service and hands them to the view a chunk at a time, newest first, so the list
//! grows upward without a stall. Hidden again, it shrinks after `SHRINK_AFTER`.

use std::time::Duration;

use cyex_chat::model::RoomEvent;
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use cyex_twitch::pace::Lane;
use tauri::{AppHandle, Manager};

use super::{Chat, RoomKey, account_of, now_ms, queue};

/// A chat hidden this long drops to the small buffer: coming back sooner costs nothing.
const SHRINK_AFTER: Duration = Duration::from_secs(45);
/// Lines per chunk, and the wait between chunks (just over one flush, so each is its own).
const CHUNK: usize = 50;
const STEP: Duration = Duration::from_millis(60);
/// The history service keeps no more.
const SERVICE_MAX: usize = 800;

/// The chat is shown again after it was in the background.
pub fn start(app: &AppHandle, key: RoomKey) {
    if !app.state::<crate::settings::Settings>().read::<ChatSettings>(Partition::Chat).history {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::emotes::ready(&app, &key.1, Duration::from_secs(4)).await;
        let want = {
            let chat = app.state::<Chat>();
            let inner = chat.lock();
            match inner.rooms.get(&key) {
                Some(e) if !e.views.is_empty() && e.room.missing() > 0 => e.room.limit().min(SERVICE_MAX),
                _ => return,
            }
        };
        let http = app.state::<crate::auth::Auth>().http.clone();
        let lines = match cyex_twitch::recent::recent_messages(&http, &key.1, want as u32, Lane::Front).await {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("chat: no refill for {}: {}", key.1, e.describe("recent-messages"));
                return;
            }
        };
        let emotes = crate::emotes::room(&app, account_of(&key.0), &key.1);
        let run = {
            let chat = app.state::<Chat>();
            let mut inner = chat.lock();
            let Some(entry) = inner.rooms.get_mut(&key) else { return };
            if entry.views.is_empty() {
                return;
            }
            let added = entry.room.backfill(&lines, now_ms(), &emotes);
            let running = !entry.fill.is_empty();
            let mut ids: Vec<String> = added.iter().map(|m| m.id.clone()).collect();
            ids.append(&mut entry.fill);
            entry.fill = ids;
            !running && !entry.fill.is_empty()
        };
        if run {
            drip(&app, &key).await;
        }
    });
}

/// Sends the refilled lines, newest chunk first, until none are left or nobody looks.
async fn drip(app: &AppHandle, key: &RoomKey) {
    loop {
        {
            let chat = app.state::<Chat>();
            let mut inner = chat.lock();
            let Some(entry) = inner.rooms.get_mut(key) else { return };
            if entry.views.is_empty() {
                entry.fill.clear();
            }
            if entry.fill.is_empty() {
                return;
            }
            let at = entry.fill.len().saturating_sub(CHUNK);
            let chunk = entry.fill.split_off(at);
            let messages = entry.room.current(&chunk);
            if !messages.is_empty() {
                queue(entry, [RoomEvent::History { messages }], &chat.wake);
            }
        }
        tokio::time::sleep(STEP).await;
    }
}

/// The last view closed but a tab still holds the room: shrink it unless it's shown again
/// before `SHRINK_AFTER`. `shown`: the room's open count when its last view closed.
pub fn later(app: &AppHandle, key: RoomKey, shown: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SHRINK_AFTER).await;
        let chat = app.state::<Chat>();
        let mut inner = chat.lock();
        if let Some(entry) = inner.rooms.get_mut(&key)
            && entry.views.is_empty()
            && entry.shown == shown
        {
            entry.fill.clear();
            entry.room.set_background(true);
        }
    });
}
