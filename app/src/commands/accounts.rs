//! Account commands (1.3, 1.4, 1.5, 1.13, 1.23).

use cyex_auth::TokenKind;
use cyex_auth::paste::ManualToken;
use cyex_chat::ConnectionCount;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::auth::{self, Auth, AuthSnapshot, DeviceStart};
use crate::connections;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn accounts_snapshot(app: AppHandle) -> AuthSnapshot {
    auth::snapshot(&app)
}

/// Device Code sign-in. Sends the code over `on_code`, then resolves with the display
/// name once the user approves on twitch.tv.
#[tauri::command]
pub async fn accounts_sign_in(
    app: AppHandle,
    kind: TokenKind,
    on_code: Channel<DeviceStart>,
) -> CmdResult<String> {
    auth::sign_in_device(&app, kind, |code| {
        let _ = on_code.send(code);
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_cancel_sign_in(auth: State<Auth>) {
    auth.cancel_sign_in();
}

/// Adds a token someone has (1.4, V101-08): name, user ID, client ID and token, where only
/// the token is needed. Resolves with the display name.
#[tauri::command]
pub async fn accounts_add_manual(app: AppHandle, fields: ManualToken) -> CmdResult<String> {
    auth::add_manual(&app, &fields).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_remove(app: AppHandle, user_id: String) -> CmdResult<()> {
    auth::remove_account(&app, &user_id)
}

#[tauri::command]
pub fn accounts_remove_token(app: AppHandle, user_id: String, kind: TokenKind) -> CmdResult<()> {
    auth::remove_token(&app, &user_id, kind)
}

#[tauri::command]
pub fn accounts_set_default(app: AppHandle, user_id: String) -> CmdResult<()> {
    auth::update_accounts(&app, |a| cyex_auth::roster::set_default(a, &user_id))?;
    auth::notify(&app);
    Ok(())
}

/// First-run prompt (D5): saves this app's Twitch client ID to the app data `.env`.
#[tauri::command]
pub fn accounts_set_client_id(app: AppHandle, auth: State<Auth>, client_id: String) -> CmdResult<()> {
    auth.set_client_id(&client_id)?;
    auth::notify(&app);
    Ok(())
}

#[tauri::command]
pub fn connections_counts(app: AppHandle) -> Vec<ConnectionCount> {
    connections::counts(&app)
}
