//! Phase 8 chat commands: search (F1, C29), feeds (F4), translation (C17), recordings
//! (X6), polls and predictions (C4), power-ups (1.42) and GIFs (1.43).

use cyex_chat::model::NoticeKind;
use cyex_twitch::giphy::Gif;
use cyex_twitch::gql::gifs::GifSent;
use cyex_twitch::gql::powerups::PowerUps;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::chat::feeds::{self, FeedEvent, FeedLine};
use crate::chat::powerups::{self, GifInfo, PowerUpBuy};
use crate::chat::record::{self, Recording, RecordingMeta};
use crate::chat::votes::{self, VotesUpdate};
use crate::chat::{self, Chat};

type CmdResult<T> = Result<T, String>;

/// F1 Search one chat, or with no view every joined chat (C29).
#[tauri::command]
pub async fn chat_search(app: AppHandle, view: Option<u32>, query: String) -> CmdResult<Vec<FeedLine>> {
    chat::search::search(&app, view, &query).await
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FeedOpened {
    pub view: u32,
    pub lines: Vec<FeedLine>,
}

/// F4 Streams a feed tab's lines.
#[tauri::command]
pub fn feed_open(app: AppHandle, window: tauri::WebviewWindow, name: String, on_event: Channel<Vec<FeedEvent>>) -> FeedOpened {
    let (view, lines) = feeds::open(&app, &name, window.label(), on_event);
    FeedOpened { view, lines }
}

/// V107-MF A rules feed's Clear: only the lines the tab hid.
#[tauri::command]
pub fn feed_clear(app: AppHandle, name: String, keys: Vec<String>) {
    feeds::remove(&app, &name, &keys);
}

#[tauri::command]
pub fn feed_close(app: AppHandle, view: u32) {
    feeds::close(&app, view);
}

/// C17 Right-click → Translate.
#[tauri::command]
pub async fn chat_translate(app: AppHandle, view: u32, id: String) -> CmdResult<()> {
    chat::translate::one(&app, view, &id).await
}

/// C17 The window's language, the default translation target.
#[tauri::command]
pub fn chat_set_language(app: AppHandle, lang: String) {
    chat::translate::set_ui_language(&app, &lang);
}

fn view_key(app: &AppHandle, view: u32) -> CmdResult<chat::RoomKey> {
    let chat = app.state::<Chat>();
    chat.view_key(view).ok_or_else(|| "This chat is closed.".into())
}

/// X6 Starts or stops recording a chat. Returns whether it records now.
#[tauri::command]
pub fn chat_record(app: AppHandle, view: u32, on: bool) -> CmdResult<bool> {
    let key = view_key(&app, view)?;
    if on {
        record::start(&app, &key)?;
        chat::local_line(&app, &key, NoticeKind::Info, "Recording this chat.");
    } else if let Some(meta) = record::stop(&app, &key) {
        let text = format!(
            "Recording saved: {} messages. It’s in Settings → Backup and sharing.",
            meta.messages
        );
        chat::local_line(&app, &key, NoticeKind::Info, &text);
    }
    Ok(record::is_recording(&app, &key))
}

#[tauri::command]
pub fn chat_recording_state(app: AppHandle, view: u32) -> CmdResult<bool> {
    Ok(record::is_recording(&app, &view_key(&app, view)?))
}

#[tauri::command]
pub fn recordings_list(app: AppHandle) -> Vec<RecordingMeta> {
    record::list(&app)
}

#[tauri::command]
pub fn recording_load(app: AppHandle, id: String) -> CmdResult<Recording> {
    record::load(&app, &id)
}

#[tauri::command]
pub fn recording_delete(app: AppHandle, id: String) -> CmdResult<()> {
    record::delete(&app, &id)
}

/// C4 What a chat shows of polls and predictions now.
#[tauri::command]
pub fn chat_votes(app: AppHandle, login: String) -> Option<VotesUpdate> {
    votes::state(&app, &login.to_ascii_lowercase())
}

#[tauri::command]
pub async fn chat_vote(app: AppHandle, view: u32, choice: String, points: Option<u32>) -> CmdResult<()> {
    votes::vote(&app, view, &choice, points).await
}

#[tauri::command]
pub async fn chat_predict(app: AppHandle, view: u32, outcome: String, points: u64) -> CmdResult<()> {
    votes::predict(&app, view, &outcome, points).await
}

/// 1.42 Power-ups and their prices in a chat.
#[tauri::command]
pub async fn chat_power_ups(app: AppHandle, view: u32) -> CmdResult<PowerUps> {
    powerups::list(&app, view).await
}

/// 1.42 Buys a power-up for `bits`, the price the user confirmed.
#[tauri::command]
pub async fn chat_power_up(app: AppHandle, view: u32, what: PowerUpBuy, bits: u32) -> CmdResult<()> {
    powerups::buy(&app, view, what, bits).await
}

#[tauri::command]
pub async fn chat_gif_info(app: AppHandle, view: u32) -> CmdResult<GifInfo> {
    powerups::gif_info(&app, view).await
}

#[tauri::command]
pub async fn chat_gif_search(app: AppHandle, view: u32, query: String, offset: u32) -> CmdResult<Vec<Gif>> {
    powerups::gif_search(&app, view, &query, offset).await
}

#[tauri::command]
pub async fn chat_gif_send(
    app: AppHandle,
    view: u32,
    id: String,
    url: String,
    search: String,
) -> CmdResult<GifSent> {
    powerups::gif_send(&app, view, &id, &url, &search).await
}
