//! 1.36 The usercard's extra data: sub status, follow age, name history, logs, and its
//! buttons (S1, block, hide).

use cyex_twitch::logs::{NameChange, SubStatus};
use tauri::AppHandle;

use crate::chat::run::{MenuContext, UiAction};
use crate::chat::usercard::{self, FollowAge, LogPage};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn user_sub_status(app: AppHandle, user: String, channel: String) -> CmdResult<SubStatus> {
    usercard::sub_status(&app, &user, &channel).await
}

#[tauri::command]
pub async fn user_follow_age(app: AppHandle, channel: String, user_id: String) -> CmdResult<FollowAge> {
    usercard::follow_age(&app, &channel, &user_id).await
}

/// V102-13 All their Twitch badges in the channel.
#[tauri::command]
pub async fn user_badges(app: AppHandle, user: String, channel: String) -> CmdResult<Vec<cyex_twitch::gql::BadgeInfo>> {
    usercard::badges(&app, &user, &channel).await
}

#[tauri::command]
pub async fn user_name_history(app: AppHandle, user_id: String) -> CmdResult<Vec<NameChange>> {
    usercard::name_history(&app, &user_id).await
}

#[tauri::command]
pub async fn user_logs(
    app: AppHandle,
    channel: String,
    user_id: String,
    user: String,
    offset: u32,
) -> CmdResult<LogPage> {
    usercard::logs(&app, &channel, &user_id, &user, offset).await
}

/// M8 Someone's messages from Twitch's mod logs, for a mod.
#[tauri::command]
pub async fn user_mod_logs(
    app: AppHandle,
    account: String,
    channel: String,
    user_id: String,
    cursor: Option<String>,
) -> CmdResult<LogPage> {
    usercard::mod_logs(&app, &account, &channel, &user_id, cursor.as_deref()).await
}

/// S1 A usercard button, run in the card's channel.
#[tauri::command]
pub async fn usercard_run(
    app: AppHandle,
    channel: String,
    command: String,
    context: MenuContext,
) -> CmdResult<Option<UiAction>> {
    crate::chat::run::menu_in(&app, &channel.to_ascii_lowercase(), &command, context).await
}

/// F2 Some signed-in account blocks them.
#[tauri::command]
pub fn user_blocked(app: AppHandle, user_id: String) -> bool {
    crate::chat::rules::is_blocked(&app, &user_id)
}
