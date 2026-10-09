//! Chat commands (1.8, 1.9, 1.36, 1.40, X2).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use cyex_chat::model::{ChatMessage, RoomEvent};
use cyex_twitch::gql::{self, Points, Profile};
use cyex_twitch::resolver;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::auth::Auth;
use crate::chat::catalog::{self, ChannelCatalog, GlobalCatalog};
use crate::chat::{self, ChatOpened};

type CmdResult<T> = Result<T, String>;

/// Opens a chat view; events stream over `on_event` until `chat_close`.
#[tauri::command]
pub fn chat_open(
    app: AppHandle,
    window: WebviewWindow,
    login: String,
    account: Option<String>,
    on_event: Channel<Vec<RoomEvent>>,
) -> CmdResult<ChatOpened> {
    chat::open(&app, window.label(), &login, account.as_deref(), on_event)
}

#[tauri::command]
pub fn chat_close(app: AppHandle, view: u32) {
    chat::close(&app, view);
}

#[tauri::command]
pub fn chat_send(app: AppHandle, view: u32, text: String, reply_to: Option<String>) -> CmdResult<()> {
    chat::send(&app, view, &text, reply_to)
}

#[tauri::command]
pub async fn chat_global_catalog(app: AppHandle) -> CmdResult<Arc<GlobalCatalog>> {
    catalog::global(&app).await
}

#[tauri::command]
pub async fn chat_channel_catalog(app: AppHandle, login: String) -> CmdResult<Option<ChannelCatalog>> {
    Ok(catalog::channel(&app, &login, false)
        .await?
        .map(|c| catalog::channel_view(&c)))
}

#[tauri::command]
pub async fn chat_points(app: AppHandle, view: u32) -> CmdResult<Option<Points>> {
    chat::points::points(&app, view).await
}

#[tauri::command]
pub async fn chat_redeem(app: AppHandle, view: u32, reward_id: String, text: Option<String>) -> CmdResult<String> {
    chat::points::redeem(&app, view, &reward_id, text).await
}

/// The user's last messages in a channel (usercard, 1.36).
#[tauri::command]
pub fn chat_user_messages(app: AppHandle, channel: String, user_id: String) -> Vec<Arc<ChatMessage>> {
    chat::recent_from(&app, &channel, &user_id)
}

/// X2 Where a link really goes. Cached for the run.
#[tauri::command]
pub async fn chat_resolve_link(app: AppHandle, url: String) -> CmdResult<Option<String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    if let Some(hit) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(&url) {
        return Ok(hit.clone());
    }
    let http = app.state::<Auth>().http.clone();
    let resolved = resolver::final_url(&http, &url).await.map_err(|e| e.to_string())?;
    let mut cache = cache.lock().unwrap_or_else(|p| p.into_inner());
    if cache.len() > 2000 {
        cache.clear();
    }
    cache.insert(url, resolved.clone());
    Ok(resolved)
}

/// Public profile for the usercard (1.36), by login or id.
#[tauri::command]
pub async fn user_profile(app: AppHandle, login: Option<String>, id: Option<String>) -> CmdResult<Option<Profile>> {
    let http = app.state::<Auth>().http.clone();
    gql::profile(&http, login.as_deref(), id.as_deref())
        .await
        .map_err(|e| e.to_string())
}
