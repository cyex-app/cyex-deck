//! Emote, badge and paint commands (1.37-1.41).

use cyex_chat::model::Provider;
use cyex_emotes::Looks;
use cyex_emotes::bttv::api::BotCommand;
use cyex_emotes::ffz::api::RoomBadges;
use tauri::AppHandle;

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
