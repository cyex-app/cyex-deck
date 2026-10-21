//! V107-R37 IPC for the stream manager's rewards panel: the manager account's own
//! channel points rewards and redemption queue.

use cyex_twitch::gql::dashboard::rewards::{Queue, Reward, RewardEdit, Rewards, Status, Switch};
use tauri::AppHandle;

use crate::streammanager::rewards;

#[tauri::command]
pub async fn sm_rewards(app: AppHandle) -> Result<Rewards, String> {
    rewards::list(&app).await
}

#[tauri::command]
pub fn sm_reward_check(edit: RewardEdit) -> Vec<String> {
    rewards::check(&edit)
}

#[tauri::command]
pub async fn sm_reward_create(app: AppHandle, edit: RewardEdit) -> Result<Reward, String> {
    rewards::create(&app, edit).await
}

#[tauri::command]
pub async fn sm_reward_update(app: AppHandle, id: String, edit: RewardEdit) -> Result<Reward, String> {
    rewards::update(&app, id, edit).await
}

#[tauri::command]
pub async fn sm_reward_switch(app: AppHandle, id: String, switch: Switch) -> Result<Reward, String> {
    rewards::switch(&app, id, switch).await
}

/// `waiting`: complete or refund its waiting redemptions.
#[tauri::command]
pub async fn sm_reward_delete(app: AppHandle, id: String, waiting: Option<Status>) -> Result<(), String> {
    rewards::delete(&app, id, waiting).await
}

#[tauri::command]
pub async fn sm_redemptions(
    app: AppHandle,
    reward: Option<String>,
    cursor: Option<String>,
) -> Result<Queue, String> {
    rewards::queue(&app, reward, cursor).await
}

/// How many went through.
#[tauri::command]
pub async fn sm_redemptions_set(app: AppHandle, ids: Vec<String>, status: Status) -> Result<usize, String> {
    rewards::set_status(&app, ids, status).await
}
