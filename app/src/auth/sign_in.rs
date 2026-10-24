//! Adding and removing accounts (1.3, 1.5, 1.23, V102-19, V108-01): Twitch's sign-in in the
//! browser or a sign-in window, a code, or a GQL token pasted for an account signed in with Twitch.

use cyex_auth::inspect::{Inspected, inspect};
use cyex_auth::{AuthError, Secret, TokenKind, browser, device, paste, roster};
use cyex_twitch::clients;
use cyex_settings::schema::TokenSource;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::open::{self, Place};
use super::{Auth, accounts, notify, unix_now, update_accounts};

/// What the UI shows while the user approves a code on Twitch (1.5).
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

/// Device Code sign-in ("Sign in with a code"). Calls `on_code` once the code is ready and
/// opens Twitch's page for it, code filled in, where `place` says (V108-02), then waits for
/// approval. With `user_id`, the sign-in is added to that account and must be the same user
/// (V102-19); a code approved by someone else is undone.
pub async fn sign_in_device(
    app: &AppHandle,
    kind: TokenKind,
    user_id: Option<&str>,
    place: Place,
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
    auth.set_page(place, &code.verification_uri);
    if let Err(e) = open::open(app, place, &code.verification_uri) {
        eprintln!("sign-in: couldn't open Twitch's page: {e}");
    }
    let result = async {
        let secret = device::wait(&auth.http, client_id, kind, &code, cancel).await?;
        finish(app, kind, user_id, secret, TokenSource::Device).await
    }
    .await;
    if place == Place::App
        && let Ok(s) = &result
    {
        crate::windows::sign_in::keep_login(app, &s.user_id).await;
    }
    ended(app, &mine);
    result
}

/// V108-01 "Sign in with Twitch": Twitch's sign-in page (implicit grant, `clients::SIGN_IN`)
/// where `place` says; the token comes back through the one-shot listener on 127.0.0.1.
/// Calls `on_start` once the page is open. Same-user rule as `sign_in_device`.
pub async fn sign_in_browser(
    app: &AppHandle,
    user_id: Option<&str>,
    place: Place,
    on_start: impl FnOnce(),
) -> Result<SignedIn, AuthError> {
    let auth = app.state::<Auth>();
    let cancel = auth.begin_sign_in();
    let mine = cancel.clone();
    let result = async {
        // Bound first: a taken port fails before any page opens.
        let listener = browser::Listener::bind()?;
        let state = browser::new_state();
        let url = browser::authorize_url(clients::SIGN_IN, &state);
        auth.set_page(place, &url);
        open::open(app, place, &url).map_err(AuthError::Listener)?;
        on_start();
        let access = tauri::async_runtime::spawn_blocking(move || {
            listener.wait(&state, browser::TIMEOUT, || *cancel.borrow())
        })
        .await
        .map_err(|e| AuthError::Listener(e.to_string()))??;
        let secret = Secret {
            access,
            refresh: None,
            client_id: clients::SIGN_IN.to_owned(),
        };
        finish(app, TokenKind::Helix, user_id, secret, TokenSource::Browser).await
    }
    .await;
    if place == Place::App
        && let Ok(s) = &result
    {
        crate::windows::sign_in::keep_login(app, &s.user_id).await;
    }
    ended(app, &mine);
    result
}

/// V108-02 The page to show again ("Open again"), where the running sign-in opened it.
pub fn reopen(app: &AppHandle) -> Result<(), String> {
    let (place, url) = app.state::<Auth>().page().ok_or("No sign-in is running.")?;
    open::open(app, place, &url)
}

/// The app's sign-in window closes unless a newer sign-in owns it.
fn ended(app: &AppHandle, mine: &tokio::sync::watch::Receiver<bool>) {
    let auth = app.state::<Auth>();
    if !auth.sign_in_replaced(mine) {
        auth.clear_page();
        crate::windows::sign_in::close(app);
    }
}

/// Checks the new token (whose, which app), keeps it, or undoes it for the wrong user.
async fn finish(
    app: &AppHandle,
    kind: TokenKind,
    user_id: Option<&str>,
    secret: Secret,
    source: TokenSource,
) -> Result<SignedIn, AuthError> {
    let auth = app.state::<Auth>();
    let who = inspect(&auth.http, &secret.access).await?;
    let wrong = if who.validation.client_id != secret.client_id {
        Some(AuthError::WrongClient)
    } else {
        user_id.and_then(|id| paste::same_user(&who.validation, id).err())
    };
    if let Some(e) = wrong {
        let http = auth.http.clone();
        tauri::async_runtime::spawn(async move {
            let _ = cyex_twitch::id::revoke(&http, &secret.client_id, &secret.access).await;
        });
        return Err(e);
    }
    save(app, &who, kind, secret, source)
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
    // V1051-02 Its 7TV session goes too.
    crate::cosmetics::forget_account(app, user_id);
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
    if matches!(source, Some(TokenSource::Device | TokenSource::Browser)) {
        let http = auth.http.clone();
        tauri::async_runtime::spawn(async move {
            let _ = cyex_twitch::id::revoke(&http, &secret.client_id, &secret.access).await;
        });
    }
    Ok(())
}
