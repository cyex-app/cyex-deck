//! V1051-02/03 The cosmetics menu's commands.

use cyex_emotes::seventv::account::view::CosmeticsView;
use cyex_settings::schema::{ComboDone, ComboItem, ComboOp};
use tauri::AppHandle;

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

/// V1051-02 Opens 7TV's sign-in window and waits for the outcome.
#[tauri::command]
pub async fn cosmetics_sign_in(app: AppHandle, account: String) -> Result<SignIn, String> {
    known(&app, &account)?;
    Ok(cosmetics::sign_in(&app, &account).await)
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
