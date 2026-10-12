//! Adding and removing accounts (1.3, 1.5, 1.23, V102-19): Twitch's sign-in, a code, or a
//! GQL token pasted for an account signed in with Twitch.

use cyex_auth::inspect::{Inspected, inspect};
use cyex_auth::{AuthError, Secret, TokenKind, device, paste, roster};
use cyex_twitch::clients;
use cyex_settings::schema::TokenSource;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Auth, accounts, notify, unix_now, update_accounts};
use crate::windows::twitch;

/// What the UI shows while the user approves the sign-in on Twitch, in the app's Twitch
/// window (V103-01).
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceStart {
    pub user_code: String,
    /// twitch.tv/activate with the code filled in.
    pub uri: String,
    pub expires_in: u32,
}

/// What a finished sign-in tells the UI.
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignedIn {
    pub user_id: String,
    pub display_name: String,
}

/// Device Code sign-in. Calls `on_code` once the code is ready and opens Twitch's page
/// for it in the app (V103-01), then waits for approval. With `user_id`, the sign-in is
/// added to that account and must be the same user (V102-19); a code approved by someone
/// else is undone.
pub async fn sign_in_device(
    app: &AppHandle,
    kind: TokenKind,
    user_id: Option<&str>,
    on_code: impl FnOnce(DeviceStart),
) -> Result<SignedIn, AuthError> {
    let auth = app.state::<Auth>();
    let client_id = device::client_for(kind);
    let cancel = auth.begin_sign_in();
    let mine = cancel.clone();
    let code = device::start(&auth.http, client_id, kind).await?;
    on_code(DeviceStart {
        user_code: code.user_code.clone(),
        uri: code.verification_uri.clone(),
        expires_in: u32::try_from(code.expires_in).unwrap_or(u32::MAX),
    });
    if let Err(e) = twitch::open_activation(app, user_id, &code.verification_uri).await {
        eprintln!("sign-in: couldn't open Twitch's page: {e}");
    }
    let result = approved(app, kind, user_id, client_id, &code, cancel).await;
    // A sign-in started after this one owns the window now.
    if !auth.sign_in_replaced(&mine) {
        twitch::sign_in_done(app, user_id, result.as_ref().ok().map(|s| s.user_id.as_str()));
    }
    result
}

async fn approved(
    app: &AppHandle,
    kind: TokenKind,
    user_id: Option<&str>,
    client_id: &str,
    code: &device::DeviceCode,
    cancel: tokio::sync::watch::Receiver<bool>,
) -> Result<SignedIn, AuthError> {
    let auth = app.state::<Auth>();
    let secret = device::wait(&auth.http, client_id, kind, code, cancel).await?;
    let who = inspect(&auth.http, &secret.access).await?;
    if let Some(id) = user_id
        && let Err(e) = paste::same_user(&who.validation, id)
    {
        let http = auth.http.clone();
        tauri::async_runtime::spawn(async move {
            let _ = cyex_twitch::id::revoke(&http, &secret.client_id, &secret.access).await;
        });
        return Err(e);
    }
    save(app, &who, kind, secret, TokenSource::Device)
}

/// V102-19 A GQL token pasted for an account signed in with Twitch, like Chatterino's
/// advanced sign-in without the other fields: the name and user ID are the account's.
/// The token has to be made for the GQL client ID and be that account's.
pub async fn add_gql_token(app: &AppHandle, user_id: &str, pasted: &str) -> Result<SignedIn, AuthError> {
    if accounts(app).get(user_id).is_none() {
        return Err(AuthError::NoAccount);
    }
    let token = paste::token_from(pasted).ok_or(AuthError::NotAToken)?;
    let auth = app.state::<Auth>();
    let who = inspect(&auth.http, &token).await?;
    paste::check_gql(&who.validation, user_id)?;
    let secret = Secret {
        access: token,
        refresh: None,
        client_id: clients::GQL.to_owned(),
    };
    save(app, &who, TokenKind::Gql, secret, TokenSource::Pasted)
}

fn save(
    app: &AppHandle,
    who: &Inspected,
    kind: TokenKind,
    secret: Secret,
    source: TokenSource,
) -> Result<SignedIn, AuthError> {
    let user_id = &who.validation.user_id;
    app.state::<Auth>()
        .vault
        .put(user_id, kind, secret)
        .map_err(AuthError::Store)?;
    let now = unix_now();
    update_accounts(app, |a| {
        roster::record_sign_in(a, who, kind, source, now);
        true
    })
    .map_err(AuthError::Store)?;
    notify(app);
    Ok(SignedIn {
        user_id: user_id.clone(),
        display_name: who.display_name.clone(),
    })
}

pub fn remove_account(app: &AppHandle, user_id: &str) -> Result<(), String> {
    for kind in TokenKind::ALL {
        forget(app, user_id, kind)?;
    }
    update_accounts(app, |a| {
        roster::remove_account(a, user_id);
        true
    })?;
    // 3.8 Its Twitch-page sign-in goes too.
    crate::windows::twitch::forget(app, user_id);
    notify(app);
    Ok(())
}

pub fn remove_token(app: &AppHandle, user_id: &str, kind: TokenKind) -> Result<(), String> {
    forget(app, user_id, kind)?;
    update_accounts(app, |a| {
        roster::remove_token(a, user_id, kind);
        true
    })?;
    notify(app);
    Ok(())
}

/// Deletes the token. Tokens this app signed in for are also revoked on Twitch; pasted
/// ones are left alone, since they may be used elsewhere.
fn forget(app: &AppHandle, user_id: &str, kind: TokenKind) -> Result<(), String> {
    let auth = app.state::<Auth>();
    let source = accounts(app)
        .get(user_id)
        .and_then(|a| kind.meta(a))
        .map(|m| m.source);
    let Some(secret) = auth.vault.forget(user_id, kind)? else {
        return Ok(());
    };
    if source == Some(TokenSource::Device) {
        let http = auth.http.clone();
        tauri::async_runtime::spawn(async move {
            let _ = cyex_twitch::id::revoke(&http, &secret.client_id, &secret.access).await;
        });
    }
    Ok(())
}
