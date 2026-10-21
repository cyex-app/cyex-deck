//! IPC for the stream manager's Roles Manager and the usercard's VIP slots (V107-R36).

use cyex_moderation::roles_manager::{Role, VipSlots};
use tauri::AppHandle;

use crate::moderation::roles::{self, RolesView};

/// Everyone with a role in your own channel (`login`, the account's).
#[tauri::command]
pub async fn roles_view(app: AppHandle, account: String, login: String) -> Result<RolesView, String> {
    roles::view(&app, &account, &login).await
}

/// Gives (`on`) or takes `role`; `holds` is what their row showed. The lists come back.
#[tauri::command]
pub async fn roles_set(app: AppHandle, account: String, login: String, who: String, role: Role, on: bool, holds: Vec<Role>) -> Result<RolesView, String> {
    roles::set(&app, &account, &login, &who, role, on, holds).await
}

/// The VIP slots in `channel` when it's the account's own; `None` elsewhere.
#[tauri::command]
pub async fn roles_vip_slots(app: AppHandle, account: String, channel: String) -> Result<Option<VipSlots>, String> {
    roles::vip_slots(&app, &account, &channel).await
}
