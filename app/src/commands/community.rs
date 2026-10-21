//! V107-R40 IPC for the stream manager's Community panels: your followers, and the roles
//! you hold in other channels. GQL only, the way Twitch's dashboard reads them.

use cyex_twitch::gql::dashboard::community::{self, AssignedList, Followers, Role};
use tauri::AppHandle;

use crate::streammanager::gql_call::gql_call;
use crate::streammanager::words;

/// A page of your followers, newest or oldest first, after `cursor`.
#[tauri::command]
pub async fn cm_followers(app: AppHandle, cursor: Option<String>, newest: bool) -> Result<Followers, String> {
    let c = gql_call(&app)?;
    community::followers(&c.channel(), cursor.as_deref(), newest)
        .await
        .map_err(|e| words::gql(&e))
}

/// Every channel where you moderate, lead mods, edit or manage business.
#[tauri::command]
pub async fn cm_assigned(app: AppHandle) -> Result<AssignedList, String> {
    let c = gql_call(&app)?;
    community::assigned(&c.channel()).await.map_err(|e| words::gql(&e))
}

/// Leaves your own role in a channel.
#[tauri::command]
pub async fn cm_step_down(app: AppHandle, role: Role, channel_id: String) -> Result<(), String> {
    let c = gql_call(&app)?;
    community::step_down(&c.channel(), role, &channel_id)
        .await
        .map_err(|e| words::gql(&e))
}
