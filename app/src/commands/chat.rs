//! Chat commands (1.8, 1.9, 1.36, 1.40, X2).

use std::sync::Arc;

use cyex_chat::model::{ChatMessage, RoomEvent};
use cyex_twitch::gql::modlogs::Chatters;
use cyex_twitch::gql::{self, Points, Profile};
use cyex_twitch::preview::LinkPreview;
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

/// V109-F81 The Reconnect on a chat that isn't in (no answer, refused, or banned and maybe
/// not anymore): sends its JOIN again.
#[tauri::command]
pub fn chat_rejoin(app: AppHandle, view: u32) -> bool {
    chat::rejoin_view(&app, view)
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

/// V108-BD A line's Twitch badge isn't in its catalog: the channel's (`login`), a Shared Chat
/// source room's (`room_id`) or, with neither, the global one. Read again in the background.
#[tauri::command]
pub fn chat_badge_missing(app: AppHandle, login: Option<String>, room_id: Option<String>) {
    use chat::badge_refresh::{self, Scope};
    let scope = match (room_id, login) {
        (Some(id), _) => Scope::Room(id),
        (None, Some(login)) => Scope::Channel(login),
        (None, None) => Scope::Global,
    };
    badge_refresh::missing(&app, scope);
}

/// V108-C10 V108-N12 A Shared Chat source room: name, picture and its own badge pictures.
#[tauri::command]
pub async fn chat_source_badges(app: AppHandle, room_id: String) -> CmdResult<gql::SourceRoom> {
    Ok(catalog::source(&app, &room_id).await?.as_ref().clone())
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

/// V107-Mk Smiley style, by Twitch's set name; answers with what Twitch has now.
#[tauri::command]
pub async fn chat_set_smilies(app: AppHandle, account: String, name: String) -> CmdResult<gql::smilies::Smilies> {
    chat::identity::set_smilies(&app, &account, &name).await
}

/// The user's last messages in a channel (usercard, 1.36).
#[tauri::command]
pub fn chat_user_messages(app: AppHandle, channel: String, user_id: String) -> Vec<Arc<ChatMessage>> {
    chat::recent_from(&app, &channel, &user_id)
}

/// V109-C01 Follow a reply thread (by its first message's id): new lines in it ping like replies.
#[tauri::command]
pub fn chat_thread_follow(app: AppHandle, channel: String, thread: String, on: bool) {
    chat::follow_thread(&app, &channel, &thread, on);
}

/// V109-C01 Whether the thread is followed.
#[tauri::command]
pub fn chat_thread_followed(app: AppHandle, channel: String, thread: String) -> bool {
    chat::follows_thread(&app, &channel, &thread)
}

/// X2 Where a link really goes, and 1.52 its card (V108-C25). `None`: nothing to show.
#[tauri::command]
pub async fn chat_resolve_link(app: AppHandle, url: String) -> Option<LinkPreview> {
    let http = app.state::<Auth>().http.clone();
    // V110-F75 Pictures only through the resolver's proxy.
    chat::link_preview::get(&http, url).await.map(LinkPreview::through_proxy)
}

/// V110-C66 The same card for a card under a message or an ask ahead of a hover: only for
/// links on the trusted list (https, listed domain), anything else is `None` unasked.
#[tauri::command]
pub async fn chat_embed_link(app: AppHandle, url: String) -> Option<LinkPreview> {
    if !cyex_chat::tokenize::trusted_link(&url) {
        return None;
    }
    let http = app.state::<Auth>().http.clone();
    // V110-F15 It loads unasked, so its picture must come from a known host.
    let mut card = chat::link_preview::get(&http, url).await?;
    card.thumbnail = card
        .thumbnail
        .filter(|t| cyex_twitch::resolver::is_proxied(t) || cyex_chat::tokenize::trusted_media(t));
    // V110-F75 ...and then it is loaded through the proxy, never from that host.
    Some(card.through_proxy())
}

/// Public profile for the usercard (1.36), by login or id.
#[tauri::command]
pub async fn user_profile(app: AppHandle, login: Option<String>, id: Option<String>) -> CmdResult<Option<Profile>> {
    // P4 #16 The usercard's client: a user-lookup mock in test builds.
    let http = chat::usercard::http(&app);
    gql::profile(&http, login.as_deref(), id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// V1051-04 Tab's candidates for an argument of `/combo`, `/paint` and `/badge`.
#[tauri::command]
pub async fn chat_command_args(app: AppHandle, view: u32, command: String, sub: Option<String>) -> CmdResult<Vec<String>> {
    chat::run::complete::args(&app, view, &command, sub.as_deref()).await
}
