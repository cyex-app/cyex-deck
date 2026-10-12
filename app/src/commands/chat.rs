//! Chat commands (1.8, 1.9, 1.36, 1.40, X2).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use cyex_chat::model::{ChatMessage, RoomEvent};
use cyex_twitch::gql::modlogs::Chatters;
use cyex_twitch::gql::{self, Points, Profile};
use cyex_twitch::resolver::{self, LinkInfo};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::auth::Auth;
use crate::chat::catalog::{self, ChannelCatalog, GlobalCatalog};
use crate::chat::run::{MenuContext, UiAction};
use crate::chat::{self, ChatOpened};

type CmdResult<T> = Result<T, String>;

/// Opens a chat view; events stream over `on_event` until `chat_close`. `filters` are
/// the F3 filter ids the panel shows messages through. `follows`: the mod view's or the
/// stream manager's own chat, which isn't a regular chat (V103-22).
#[tauri::command]
pub fn chat_open(
    app: AppHandle,
    window: WebviewWindow,
    login: String,
    account: Option<String>,
    filters: Option<Vec<String>>,
    follows: Option<bool>,
    on_event: Channel<Vec<RoomEvent>>,
) -> CmdResult<ChatOpened> {
    chat::open(
        &app,
        window.label(),
        &login,
        account.as_deref(),
        &filters.unwrap_or_default(),
        follows.unwrap_or(false),
        on_event,
    )
}

#[tauri::command]
pub fn chat_close(app: AppHandle, view: u32) {
    chat::close(&app, view);
}

/// V103-22 Where the regular chats show and which windows are still loading; changes come
/// as `chat:shown`.
#[tauri::command]
pub fn chat_shown(app: AppHandle) -> chat::shown::ChatShown {
    chat::shown::current(&app)
}

/// A typed line: custom and built-in commands, then Twitch (1.8). Some commands ask the
/// window to do something.
#[tauri::command]
pub async fn chat_send(app: AppHandle, view: u32, text: String, reply_to: Option<String>) -> CmdResult<Option<UiAction>> {
    chat::run::input(&app, view, &text, reply_to).await
}

/// S1 A menu entry or usercard button.
#[tauri::command]
pub async fn chat_menu(app: AppHandle, view: u32, command: String, context: MenuContext) -> CmdResult<Option<UiAction>> {
    chat::run::menu(&app, view, &command, context).await
}

/// V1041-01 Command names Tab completes after `/` in this view.
#[tauri::command]
pub fn chat_commands(app: AppHandle, view: u32) -> CmdResult<Vec<String>> {
    chat::run::complete::names(&app, view)
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
pub async fn chat_points(app: AppHandle, view: u32, fresh: bool) -> CmdResult<Option<Points>> {
    chat::points::points(&app, view, fresh).await
}

/// 1.48, V101-35 Who's in a channel's chat; every viewer when `account` moderates there.
#[tauri::command]
pub async fn chat_viewers(app: AppHandle, account: Option<String>, channel: String) -> CmdResult<Chatters> {
    crate::moderation::queue::chatters(&app, account.as_deref(), &channel).await
}

#[tauri::command]
pub async fn chat_redeem(
    app: AppHandle,
    view: u32,
    reward_id: String,
    text: Option<String>,
    emote: Option<String>,
) -> CmdResult<String> {
    chat::points::redeem(&app, view, &reward_id, text, emote).await
}

/// What the two "choose an emote" rewards can give.
#[tauri::command]
pub async fn chat_emote_choices(app: AppHandle, view: u32) -> CmdResult<chat::points::EmoteRewardChoices> {
    chat::points::emote_choices(&app, view).await
}

/// 1.40 The account's badge picks and name color in a channel.
#[tauri::command]
pub async fn chat_identity(app: AppHandle, account: String, login: String) -> CmdResult<gql::ChatIdentity> {
    chat::identity::identity(&app, &account, &login).await
}

/// 1.40 Wears a badge. `channel_id` `None` = everywhere; `set` `None` takes it off.
#[tauri::command]
pub async fn chat_pick_badge(
    app: AppHandle,
    account: String,
    channel_id: Option<String>,
    set: Option<String>,
    version: Option<String>,
) -> CmdResult<()> {
    let badge = set.as_deref().zip(version.as_deref());
    chat::identity::pick_badge(&app, &account, channel_id.as_deref(), badge).await
}

/// 1.40 Name color, `#RRGGBB`.
#[tauri::command]
pub async fn chat_set_color(app: AppHandle, account: String, color: String) -> CmdResult<()> {
    chat::identity::set_color(&app, &account, &color).await
}

/// The user's last messages in a channel (usercard, 1.36).
#[tauri::command]
pub fn chat_user_messages(app: AppHandle, channel: String, user_id: String) -> Vec<Arc<ChatMessage>> {
    chat::recent_from(&app, &channel, &user_id)
}

/// X2 Where a link really goes, and 1.52 its preview. Cached for the run.
#[tauri::command]
pub async fn chat_resolve_link(app: AppHandle, url: String) -> CmdResult<Option<LinkInfo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<LinkInfo>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    if let Some(hit) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(&url) {
        return Ok(hit.clone());
    }
    let http = app.state::<Auth>().http.clone();
    let resolved = resolver::resolve(&http, &url).await.map_err(|e| e.to_string())?;
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
