//! V1051-02/03 The cosmetics menu's commands.

use cyex_emotes::seventv::account::view::CosmeticsView;
use cyex_settings::schema::{ComboDone, ComboItem, ComboOp};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::auth;
use crate::cosmetics::{self, Applied, Kind, Picked, SelectError, Want};
use crate::windows::seventv::SignIn;

fn known(app: &AppHandle, account: &str) -> Result<(), String> {
    auth::accounts(app)
        .get(account)
        .map(|_| ())
        .ok_or_else(|| "Unknown account.".to_owned())
}

#[tauri::command]
pub async fn cosmetics_view(app: AppHandle, account: String, fresh: bool) -> CosmeticsView {
    cosmetics::view(&app, &account, fresh).await
}

#[tauri::command]
pub async fn cosmetics_allow(app: AppHandle, account: String) -> CosmeticsView {
    cosmetics::allow(&app, &account).await
}

#[tauri::command]
pub async fn cosmetics_select(
    app: AppHandle,
    account: String,
    kind: Kind,
    id: Option<String>,
) -> Result<Picked, SelectError> {
    cosmetics::select(&app, &account, kind, id).await
}

#[tauri::command]
pub async fn cosmetics_sign_out(app: AppHandle, account: String) -> Result<(), String> {
    known(&app, &account)?;
    cosmetics::sign_out(&app, &account).await;
    Ok(())
}

#[tauri::command]
pub async fn cosmetics_withdraw(app: AppHandle, account: String) -> Result<(), String> {
    known(&app, &account)?;
    cosmetics::withdraw(&app, &account).await;
    Ok(())
}

/// V1051-02 Opens 7TV's sign-in window and waits for the outcome. `shared`: in the account's
/// Twitch pages profile ("Open in: Browser", V108-7T) when it's connected, else a private window.
#[tauri::command]
pub async fn cosmetics_sign_in(app: AppHandle, account: String, shared: bool) -> Result<SignIn, String> {
    known(&app, &account)?;
    Ok(cosmetics::sign_in(&app, &account, shared).await)
}

/// V108-7K Keeps a 7TV token pasted from the user's browser, once 7TV says it's this
/// account's. Rejects with the message to show.
#[tauri::command]
pub async fn cosmetics_paste(app: AppHandle, account: String, token: String) -> Result<(), String> {
    known(&app, &account)?;
    cosmetics::paste(&app, &account, &token).await.map_err(|r| r.text().to_owned())
}

/// V108-7L The "Send to cyex deck" bookmark's address (this install's nonce in it).
#[tauri::command]
pub fn cosmetics_bookmarklet(app: AppHandle) -> Result<String, String> {
    cosmetics::link::bookmarklet(&app)
}

/// V108-7L Waits for the bookmark clicked on 7tv.app. Rejects with the message to show
/// ("" when stopped).
#[tauri::command]
pub async fn cosmetics_link_wait(app: AppHandle, account: String) -> Result<(), String> {
    known(&app, &account)?;
    cosmetics::link::wait(&app, &account).await
}

#[tauri::command]
pub fn cosmetics_link_stop(app: AppHandle) {
    cosmetics::link::stop(&app);
}

/// V108-7K Opens 7tv.app in the browser (and profile) used last, else the default one.
#[tauri::command]
pub fn cosmetics_open_7tv(app: AppHandle) -> Result<(), String> {
    const URL: &str = "https://7tv.app/";
    if crate::sys::browsers::open_in_last_browser(URL) {
        return Ok(());
    }
    app.opener().open_url(URL, None::<&str>).map_err(|e| e.to_string())
}

/// V1051-04 Puts on a pair (a combo, or Undo). Both parts always named; `None` = none.
#[tauri::command]
pub async fn cosmetics_apply(
    app: AppHandle,
    account: String,
    paint: Option<ComboItem>,
    badge: Option<ComboItem>,
    name: Option<String>,
) -> Result<Applied, SelectError> {
    if known(&app, &account).is_err() {
        return Err(SelectError::SignedOut);
    }
    cosmetics::apply(&app, &account, Want { paint, badge }, name.as_deref()).await
}

/// V1051-04 One change to the saved combos.
#[tauri::command]
pub fn combos_change(app: AppHandle, account: String, op: ComboOp) -> Result<ComboDone, String> {
    known(&app, &account)?;
    cosmetics::change_combos(&app, &account, op).map_err(|e| e.to_string())
}
