//! Account commands (1.3, 1.5, 1.13, 1.23, V102-19).

use cyex_auth::TokenKind;
use cyex_chat::ConnectionCount;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

use crate::auth::{self, Auth, AuthSnapshot, DeviceStart, SignedIn};
use crate::connections;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn accounts_snapshot(app: AppHandle) -> AuthSnapshot {
    auth::snapshot(&app)
}

/// Device Code sign-in. Sends the code over `on_code`, then resolves with the account
/// once the user approves on Twitch. With `user_id`, it's added to that account and has
/// to be the same user (V102-19).
#[tauri::command]
pub async fn accounts_sign_in(
    app: AppHandle,
    kind: TokenKind,
    user_id: Option<String>,
    on_code: Channel<DeviceStart>,
) -> CmdResult<SignedIn> {
    auth::sign_in_device(&app, kind, user_id.as_deref(), |code| {
        let _ = on_code.send(code);
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_cancel_sign_in(auth: State<Auth>) {
    auth.cancel_sign_in();
}

/// V102-20 A greyed-out tool's flag was clicked, in any window: the main window comes up
/// on the sign-in that unlocks it.
#[tauri::command]
pub fn accounts_show_sign_in(app: AppHandle, kind: TokenKind) {
    crate::windows::show_main(&app);
    let _ = app.emit_to(crate::windows::MAIN, "accounts:sign-in", kind);
}

/// V102-19 A GQL token pasted for an account signed in with Twitch.
#[tauri::command]
pub async fn accounts_add_gql_token(app: AppHandle, user_id: String, token: String) -> CmdResult<SignedIn> {
    auth::add_gql_token(&app, &user_id, &token).await.map_err(|e| e.to_string())
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

/// V103-01 Twitch's page for the running sign-in's code, in the app's Twitch window again
/// (the sign-in opens it once by itself).
#[tauri::command]
pub async fn accounts_open_activation(app: AppHandle, user_id: Option<String>, uri: String) -> CmdResult<()> {
    crate::windows::twitch::open_activation(&app, user_id.as_deref(), &uri).await
}

#[tauri::command]
pub fn connections_counts(app: AppHandle) -> Vec<ConnectionCount> {
    connections::counts(&app)
}
