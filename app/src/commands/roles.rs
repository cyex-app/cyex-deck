//! IPC for the stream manager's Roles Manager and the usercard's VIP slots (V107-R36).

use cyex_moderation::roles::{Role, VipSlots, Wanted};
use tauri::AppHandle;

use crate::moderation::roles::manager::{self, RolesView};

/// Everyone with a role in your own channel (`login`, the account's).
#[tauri::command]
pub async fn roles_view(app: AppHandle, account: String, login: String) -> Result<RolesView, String> {
    manager::view(&app, &account, &login).await
}

/// V108-C26 `/mods`, `/vips` and `/roles`: who holds which role, with the dates.
/// V108-C27 Any channel `login`; `account` may be none (an anonymous chat).
#[tauri::command]
pub async fn roles_listing(app: AppHandle, account: Option<String>, login: String) -> Result<manager::RoleListing, String> {
    manager::listing(&app, account.as_deref(), &login).await
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

/// V108-F49 The roles card grows to fit its longest name: never narrower than it opened,
/// at most 720 px or the screen. `width` is the CSS width the card asked for.
#[tauri::command]
pub fn roles_fit(window: tauri::WebviewWindow, width: f64) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let screen = window
        .current_monitor()
        .ok()
        .flatten()
        .map_or(f64::MAX, |m| f64::from(m.work_area().size.width) / scale);
    let target = width.min(720.0).min(screen).max(size.width);
    if target - size.width < 1.0 {
        return Ok(());
    }
    window
        .set_size(tauri::LogicalSize::new(target, size.height))
        .map_err(|e| e.to_string())
}
