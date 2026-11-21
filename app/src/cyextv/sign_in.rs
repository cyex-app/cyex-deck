//! V112-14 "Sign in to cyex.tv" for one account: cyex.tv's consent page in the browser used last
//! (like the Twitch sign-in), the code back on a one-shot 127.0.0.1 listener, then tokens. Sign
//! out revokes the session (RFC 7009) and deletes what's kept.

use cyex_auth::CyextvError;
use cyex_auth::browser::Listener;
use cyex_auth::cyextv::{self as flow, Pkce, callback, sessions::Who};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::watch;

use super::{Cyextv, lock, state};
use crate::auth::{self, Auth};

impl Cyextv {
    /// Starts tracking a new sign-in and cancels the one before.
    fn begin(&self) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        if let Some(old) = lock(&self.signing_in).replace(tx) {
            let _ = old.send(true);
        }
        rx
    }
}

/// Only cyex.tv's authorize page opens, in the browser used last (else the default one).
fn open(app: &AppHandle, base: &str, url: &str) -> Result<(), String> {
    if !flow::is_authorize_url(base, url) {
        return Err("That isn’t cyex.tv’s sign-in page.".into());
    }
    if crate::sys::browsers::open_in_last_browser(url) {
        return Ok(());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Signs `account` in to cyex.tv. Resolves with the cyex.tv name; errs with the line to show
/// (empty when cancelled).
pub async fn sign_in(app: &AppHandle, account: &str) -> Result<String, String> {
    if auth::accounts(app).get(account).is_none() {
        return Err("Unknown account.".into());
    }
    let shown = |e: CyextvError| if e == CyextvError::Cancelled { String::new() } else { e.to_string() };
    let st = state(app);
    let cancel = st.begin();
    let listener = Listener::bind_on(0).map_err(|e| e.to_string())?;
    let redirect = flow::redirect_uri(listener.port());
    let pkce = Pkce::new().map_err(shown)?;
    let want = flow::new_state().map_err(shown)?;
    open(app, &st.base, &flow::authorize_url(&st.base, &redirect, &pkce.challenge, &want))?;
    let code = tauri::async_runtime::spawn_blocking(move || {
        callback::wait(listener, &want, callback::TIMEOUT, || *cancel.borrow())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(shown)?;
    let http = app.state::<Auth>().http.clone();
    let tokens = flow::exchange(&http, &st.base, &code, &pkce.verifier, &redirect)
        .await
        .map_err(shown)?;
    // Who it is, for "Signed in as …"; a failure here still keeps the session.
    let who = match st.gql.my_cosmetics(&tokens.access).await {
        Ok(o) => Who {
            id: o.account.id,
            name: o.account.display_name,
        },
        Err(e) => {
            eprintln!("cyex.tv: {account}: who: {e}");
            Who::default()
        }
    };
    let auth = app.state::<Auth>();
    if let Err(e) = auth.cyextv.start(account, &tokens, &who, auth::unix_now()).await {
        // Not kept: the session ends on cyex.tv too.
        revoke_later(app, tokens.refresh.clone());
        return Err(shown(e));
    }
    auth::notify(app);
    Ok(who.name)
}

/// Stops the sign-in that's waiting, if any.
pub fn cancel(app: &AppHandle) {
    if let Some(tx) = lock(&state(app).signing_in).take() {
        let _ = tx.send(true);
    }
}

/// Ends the session on cyex.tv in the background (best effort).
fn revoke_later(app: &AppHandle, refresh: String) {
    // V112-23 Coming soon: no call to cyex.tv (the session is still forgotten here).
    if !cyex_core::CYEXTV_READY {
        return;
    }
    let http = app.state::<Auth>().http.clone();
    let base = state(app).base.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = flow::revoke(&http, &base, &refresh).await {
            eprintln!("cyex.tv: revoke: {e}");
        }
    });
}

async fn end(app: &AppHandle, account: &str) {
    match app.state::<Auth>().cyextv.forget(account).await {
        Ok(Some(refresh)) => revoke_later(app, refresh),
        Ok(None) => {}
        Err(e) => eprintln!("cyex.tv: {account}: couldn't delete the session: {e}"),
    }
}

pub async fn sign_out(app: &AppHandle, account: &str) {
    end(app, account).await;
    auth::notify(app);
}

/// The account is being removed: its cyex.tv session goes too.
pub fn forget_account(app: &AppHandle, account: &str) {
    if app.try_state::<Cyextv>().is_none() {
        return;
    }
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move { end(&app, &account).await });
}
