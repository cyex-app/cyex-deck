//! V1051-02/03 The cosmetics menu's commands.

use cyex_emotes::seventv::account::view::CosmeticsView;
use tauri::AppHandle;

use crate::auth;
use crate::cosmetics::{self, Kind, Picked, SelectError};

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
