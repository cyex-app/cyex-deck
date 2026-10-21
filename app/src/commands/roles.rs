//! IPC for the stream manager's Roles Manager and the usercard's VIP slots (V107-R36).

use cyex_moderation::roles::{Role, VipSlots, Wanted};
use tauri::AppHandle;

use crate::moderation::roles::manager::{self, RolesView};

/// Everyone with a role in your own channel (`login`, the account's).
#[tauri::command]
pub async fn roles_view(app: AppHandle, account: String, login: String) -> Result<RolesView, String> {
    manager::view(&app, &account, &login).await
}

/// V107-R42 Gives and takes roles for `who` in one change; Rust picks the order. The lists
/// come back with the change's line; `Err` is the line when not all of it went through.
#[tauri::command]
pub async fn roles_set(
    app: AppHandle,
    account: String,
    login: String,
    who: String,
    give: Vec<Role>,
    take: Vec<Role>,
) -> Result<RolesView, String> {
    manager::set(&app, &account, &login, &who, Wanted { give, take }).await
}

/// The VIP slots in `channel` when it's the account's own; `None` elsewhere.
#[tauri::command]
pub async fn roles_vip_slots(app: AppHandle, account: String, channel: String) -> Result<Option<VipSlots>, String> {
    manager::vip_slots(&app, &account, &channel).await
}
