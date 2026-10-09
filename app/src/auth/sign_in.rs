//! Adding and removing accounts (1.3, 1.4, 1.5, 1.23).

use cyex_auth::inspect::{Inspected, inspect};
use cyex_auth::{AuthError, Secret, TokenKind, device, paste, roster};
use cyex_settings::schema::TokenSource;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{Auth, accounts, notify, unix_now, update_accounts};

/// What the UI shows while the user approves the sign-in on twitch.tv.
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceStart {
    pub user_code: String,
    /// twitch.tv/activate with the code filled in.
    pub uri: String,
    pub expires_in: u32,
}

/// Device Code sign-in. Calls `on_code` once the code is ready, then waits for approval.
/// Returns the account's display name.
pub async fn sign_in_device(
    app: &AppHandle,
    kind: TokenKind,
    on_code: impl FnOnce(DeviceStart),
) -> Result<String, AuthError> {
    let auth = app.state::<Auth>();
    let client_id = device::client_for(kind, auth.client_id().as_deref())?;
    let cancel = auth.begin_sign_in();
    let code = device::start(&auth.http, &client_id, kind).await?;
    on_code(DeviceStart {
        user_code: code.user_code.clone(),
        uri: code.verification_uri.clone(),
        expires_in: u32::try_from(code.expires_in).unwrap_or(u32::MAX),
    });
    let secret = device::wait(&auth.http, &client_id, kind, &code, cancel).await?;
    let who = inspect(&auth.http, &secret.access).await?;
    save(app, &who, kind, secret, TokenSource::Device)
}

/// Adds a pasted token (1.4). First-party tokens become the GQL token, others Helix.
pub async fn add_pasted(app: &AppHandle, text: &str) -> Result<String, AuthError> {
    let token = paste::token_from(text).ok_or(AuthError::NotAToken)?;
    let auth = app.state::<Auth>();
    let who = inspect(&auth.http, &token).await?;
    let kind = TokenKind::for_client(&who.validation.client_id);
    let secret = Secret {
        access: token,
        refresh: None,
        client_id: who.validation.client_id.clone(),
    };
    save(app, &who, kind, secret, TokenSource::Pasted)
}

fn save(
    app: &AppHandle,
    who: &Inspected,
    kind: TokenKind,
    secret: Secret,
    source: TokenSource,
) -> Result<String, AuthError> {
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
    Ok(who.display_name.clone())
}

pub fn remove_account(app: &AppHandle, user_id: &str) -> Result<(), String> {
    for kind in TokenKind::ALL {
        forget(app, user_id, kind)?;
    }
    update_accounts(app, |a| {
        roster::remove_account(a, user_id);
        true
    })?;
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
