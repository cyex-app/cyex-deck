//! V112-14..16 cyex.tv per account: the sign-in (`cyex_auth::cyextv`), the Bearer every GraphQL
//! call gets (refreshed one at a time, a 401 retried once with a fresh one) and the cosmetic
//! picker's cyex.tv side. A different service from the deck's own cyex.app (`crate::cyex`).

mod cosmetics;
mod sign_in;

use std::future::Future;
use std::sync::{Mutex, MutexGuard};

use cyex_auth::CyextvError;
use cyex_auth::cyextv as flow;
use cyex_emotes::cyextv::gql::{Gql, GqlError};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

pub use cosmetics::{TvCosmetics, TvKind, select, view};
pub use sign_in::{cancel, forget_account, sign_in, sign_out};

/// V112-23 The one line anything cyex.tv answers while `cyex_core::CYEXTV_READY` is false.
pub(crate) const SOON: &str = "cyex.tv emotes are coming soon.";
/// V112-23 The same for the sign-in and the picker.
pub(crate) const SOON_TV: &str = "cyex.tv is coming soon.";

use crate::auth::{self, Auth};

/// Mocks builds only: a local cyex.tv (`http://127.0.0.1:<port>`, test data folder only) for the
/// sign-in and GraphQL.
#[cfg(feature = "mocks")]
const URL_VAR: &str = "CYEX_CYEXTV_URL";

pub struct Cyextv {
    /// `https://api.cyex.tv/v1`, or the mock's `/v1`.
    base: String,
    gql: Gql,
    /// Cancels the sign-in that's running, if any.
    signing_in: Mutex<Option<watch::Sender<bool>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// After `auth::init`.
pub fn init(app: &AppHandle) {
    let base = base(app);
    let http = app.state::<Auth>().http.clone();
    app.manage(Cyextv {
        gql: Gql::new(http, &base),
        base,
        signing_in: Mutex::default(),
    });
}

/// A mock that is set but refused goes nowhere (never cyex.tv with a real session).
fn base(app: &AppHandle) -> String {
    #[cfg(feature = "mocks")]
    match crate::mock_url::gated(app, &[URL_VAR], "cyex.tv") {
        Ok(Some(b)) => {
            eprintln!("cyex.tv: mock at {b}");
            return format!("{b}/v1");
        }
        Ok(None) => {}
        Err(_) => return "http://127.0.0.1:0/v1".into(),
    }
    let _ = app;
    flow::API.to_owned()
}

fn state(app: &AppHandle) -> tauri::State<'_, Cyextv> {
    app.state::<Cyextv>()
}

/// V112-15 The GraphQL client (Phase 6's commands use it with [`with_bearer`]).
pub(crate) fn gql(app: &AppHandle) -> Gql {
    state(app).gql.clone()
}

/// A live access token for the account: the kept one or a refreshed one (one refresh at a time;
/// the rotated refresh token is saved before this returns). An ended session updates Accounts.
async fn bearer(app: &AppHandle, account: &str) -> Result<String, CyextvError> {
    let http = app.state::<Auth>().http.clone();
    let base = state(app).base.clone();
    let got = app
        .state::<Auth>()
        .cyextv
        .bearer(account, auth::unix_now(), |refresh| async move {
            flow::refresh(&http, &base, &refresh).await
        })
        .await;
    if got == Err(CyextvError::SignedOut) && app.state::<Auth>().cyextv.ended(account) {
        auth::notify(app);
    }
    got
}

/// Why a call with the account's Bearer didn't go through.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CallFail {
    /// No live access token (signed out, session ended, cyex.tv down while refreshing).
    Auth(CyextvError),
    Gql(GqlError),
}

impl std::fmt::Display for CallFail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth(e) => e.fmt(f),
            Self::Gql(e) => e.fmt(f),
        }
    }
}

/// V112-15 Runs `call` with the account's Bearer. A 401 drops that access token and tries once
/// more with a refreshed one. Errs with the line to show.
pub(crate) async fn with_bearer<T, F, Fut>(app: &AppHandle, account: &str, call: F) -> Result<T, String>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, GqlError>>,
{
    with_bearer_typed(app, account, call).await.map_err(|e| e.to_string())
}

/// [`with_bearer`] with the reason kept apart (V112-17 the commands word each themselves).
pub(crate) async fn with_bearer_typed<T, F, Fut>(app: &AppHandle, account: &str, call: F) -> Result<T, CallFail>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<T, GqlError>>,
{
    let mut tries = 0;
    loop {
        tries += 1;
        let token = bearer(app, account).await.map_err(CallFail::Auth)?;
        match call(token.clone()).await {
            Err(GqlError::SignedOut) => {
                app.state::<Auth>().cyextv.stale(account, &token);
                if tries >= 2 {
                    return Err(CallFail::Gql(GqlError::SignedOut));
                }
            }
            done => return done.map_err(CallFail::Gql),
        }
    }
}

/// V112-17 Whether cyex.tv calls go to a loopback mock (a test run may only write there).
pub(crate) fn mocked(app: &AppHandle) -> bool {
    state(app).base != flow::API
}
