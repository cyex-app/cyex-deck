//! Emote, badge and paint commands (1.37-1.41).

use cyex_chat::model::Provider;
use cyex_emotes::Looks;
use cyex_emotes::bttv::api::BotCommand;
use cyex_emotes::ffz::api::RoomBadges;
use tauri::AppHandle;

use crate::emotes::edit::{self, EditDone, EditFail, EditOp, EditOptions, NameCheck, Reachability};
use crate::emotes::{self, EmoteCard, menu::EmoteMenu};

/// Who wears what among these users (by id), with the paints and badges they use.
#[tauri::command]
pub fn emotes_looks(app: AppHandle, users: Vec<String>) -> Looks {
    emotes::looks_for(&app, &users)
}

/// 1.40 Everything the emote menu shows for a chat and the account reading it.
#[tauri::command]
pub async fn emotes_menu(
    app: AppHandle,
    login: String,
    account: Option<String>,
) -> Result<EmoteMenu, String> {
    emotes::menu::menu(&app, &login, account.as_deref()).await
}

/// 1.41 An emote card's facts.
#[tauri::command]
pub async fn emotes_card(
    app: AppHandle,
    login: String,
    provider: Provider,
    id: String,
    name: String,
) -> Result<EmoteCard, String> {
    emotes::card(&app, &login, provider, &id, &name).await
}

/// 1.39 FFZ's pictures for a channel's mods and VIPs.
#[tauri::command]
pub fn emotes_room_badges(app: AppHandle, login: String) -> RoomBadges {
    emotes::room_badges(&app, &login)
}

/// C13 Chat bot commands in a channel.
#[tauri::command]
pub async fn emotes_bot_commands(app: AppHandle, login: String) -> Result<Vec<BotCommand>, String> {
    emotes::bot_commands(&app, &login).await
}

/// C23 A user's 7TV profile picture, if they have one.
#[tauri::command]
pub async fn emotes_avatar(app: AppHandle, login: String) -> Option<cyex_emotes::seventv::api::Avatar> {
    emotes::avatar(&app, &login).await
}

/// V106-31/32 Reads what the account can edit on 7TV (kept 5 min unless `fresh`).
#[tauri::command]
pub async fn emotes_edit_warm(app: AppHandle, account: String, fresh: bool) -> Reachability {
    edit::warm(&app, &account, fresh).await
}

/// V106-31/32 What the menus offer for one emote in this chat. No network.
#[tauri::command]
pub fn emotes_edit_options(
    app: AppHandle,
    account: String,
    login: String,
    emote_id: String,
    name: String,
) -> EditOptions {
    edit::options(&app, &account, &login, &emote_id, &name)
}

/// V106-31/32 Whether a name is free in a set (R16).
#[tauri::command]
pub fn emotes_edit_name(
    app: AppHandle,
    account: String,
    set_id: String,
    alias: String,
    rename_from: Option<String>,
) -> NameCheck {
    edit::name_check(&app, &account, &set_id, &alias, rename_from.as_deref())
}

/// V106-31/32 One 7TV set edit (add, remove, rename).
#[tauri::command]
pub async fn emotes_edit(app: AppHandle, account: String, op: EditOp) -> Result<EditDone, EditFail> {
    edit::run(&app, &account, op).await
}
