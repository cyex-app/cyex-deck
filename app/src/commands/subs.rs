//! IPC for subscriptions (1.28) and Twitch's own pages in the app (3.8, 1.27).

use cyex_twitch::gql::subs::{ChannelSub, Subs};
use tauri::AppHandle;

use crate::subs;
use crate::windows::twitch;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn subs_mine(app: AppHandle, account: String) -> CmdResult<Subs> {
    subs::mine(&app, &account).await
}

#[tauri::command]
pub async fn subs_channel(app: AppHandle, account: String, login: String) -> CmdResult<Option<ChannelSub>> {
    subs::channel(&app, &account, &login).await
}

#[tauri::command]
pub async fn subs_share_resub(
    app: AppHandle,
    account: String,
    login: String,
    token_id: String,
    message: String,
    streak: bool,
) -> CmdResult<()> {
    subs::share_resub(&app, &account, &login, &token_id, &message, streak).await
}

#[tauri::command]
pub async fn subs_prime(app: AppHandle, account: String, channel_id: String) -> CmdResult<()> {
    subs::prime(&app, &account, &channel_id).await
}

#[tauri::command]
pub async fn subs_stop_renewal(app: AppHandle, account: String, sub_id: String) -> CmdResult<()> {
    subs::stop_renewal(&app, &account, &sub_id).await
}

/// Opens one of Twitch's pages in the account's Twitch window: `wallet`, `subscriptions`, `bits`,
/// `subscribe` or `gift` (these two name a channel).
#[tauri::command]
pub async fn twitch_page(
    app: AppHandle,
    account: String,
    kind: String,
    login: Option<String>,
) -> CmdResult<()> {
    let url = subs::page(&kind, login.as_deref().unwrap_or_default()).ok_or("That page isn’t available.")?;
    // V102-20 Payment methods and subs wait for Twitch's own sign-in.
    if crate::auth::helix_token(&app, &account).is_none() {
        return Err("Sign in with Twitch first.".into());
    }
    twitch::open(&app, &account, &url).await
}

/// Opens any twitch.tv address in the account's Twitch window, e.g. from the bell.
#[tauri::command]
pub async fn twitch_open(app: AppHandle, account: String, url: String) -> CmdResult<()> {
    twitch::open(&app, &account, &url).await
}

/// Signs the account out of Twitch's pages in the app.
#[tauri::command]
pub fn twitch_sign_out(app: AppHandle, account: String) -> CmdResult<()> {
    twitch::sign_out(&app, &account)
}

/// V108-04 "Connect Twitch pages": twitch.tv's login in the account's window; `true` once
/// signed in there.
#[tauri::command]
pub async fn twitch_connect(app: AppHandle, account: String) -> CmdResult<bool> {
    if crate::auth::accounts(&app).get(&account).is_none() {
        return Err("That account isn’t signed in here.".into());
    }
    twitch::connect(&app, &account).await
}

/// V108-04 The accounts whose Twitch pages are signed in.
#[tauri::command]
pub fn twitch_connected(app: AppHandle) -> Vec<String> {
    twitch::connected(&app)
}
