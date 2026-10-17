//! Accounts and sign-in (spec 01) wired to Tauri: shared state, the `accounts:changed`
//! event, the token keeper. Logic lives in `cyex_auth`.

mod keeper;
mod sign_in;

pub use sign_in::{DeviceStart, SignedIn, add_gql_token, remove_account, remove_token, sign_in_device};

use std::sync::{Arc, Mutex, MutexGuard};

use cyex_auth::secrets::KeyringStore;
use cyex_auth::secrets::{MemoryStore, SecretStore};
use cyex_auth::{AccountView, SevenTvTokens, TokenKind, Vault, views};
use cyex_settings::Partition;
use cyex_settings::schema::Accounts;
use cyex_twitch::Http;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;
use ts_rs::TS;

use crate::settings::Settings;

const CHANGED_EVENT: &str = "accounts:changed";

pub struct Auth {
    pub http: Http,
    pub vault: Vault,
    /// V1051-02 7TV sessions (own keys; a test run keeps them in memory).
    pub seventv: SevenTvTokens,
    /// Cancels the Device Code sign-in that's running, if any.
    sign_in: Mutex<Option<watch::Sender<bool>>>,
}

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuthSnapshot {
    pub accounts: Vec<AccountView>,
}

/// V103-01 The sign-in's client ID is built in (`clients::SIGN_IN`); a `.env` left in the
/// app's data folder from before isn't read, and is deleted on start (`paths::remove_old_env`).
pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let vault = Vault::new(Arc::new(KeyringStore));
    for e in vault.load(&accounts(app)) {
        eprintln!("auth: couldn't read a token: {e}");
    }
    let store: Arc<dyn SecretStore> = if crate::paths::webview(app).is_some() {
        Arc::new(MemoryStore::default())
    } else {
        Arc::new(KeyringStore)
    };
    app.manage(Auth {
        http: Http::new()?,
        vault,
        seventv: SevenTvTokens::new(store),
        sign_in: Mutex::default(),
    });
    keeper::spawn(app.clone());
    Ok(())
}

impl Auth {
    /// Starts tracking a new sign-in and cancels the previous one.
    fn begin_sign_in(&self) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        if let Some(old) = lock(&self.sign_in).replace(tx) {
            let _ = old.send(true);
        }
        rx
    }

    pub fn cancel_sign_in(&self) {
        if let Some(tx) = lock(&self.sign_in).take() {
            let _ = tx.send(true);
        }
    }

    /// Whether another sign-in started after the one `mine` belongs to.
    fn sign_in_replaced(&self, mine: &watch::Receiver<bool>) -> bool {
        lock(&self.sign_in)
            .as_ref()
            .is_some_and(|tx| !tx.subscribe().same_channel(mine))
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn accounts(app: &AppHandle) -> Accounts {
    app.state::<Settings>().read(Partition::Accounts)
}

/// Changes the accounts list and saves it if `f` returns true.
pub fn update_accounts(app: &AppHandle, f: impl FnOnce(&mut Accounts) -> bool) -> Result<(), String> {
    app.state::<Settings>().update(app, Partition::Accounts, f)
}

pub fn snapshot(app: &AppHandle) -> AuthSnapshot {
    let auth = app.state::<Auth>();
    AuthSnapshot {
        accounts: views(&accounts(app), &auth.vault, &auth.seventv, unix_now()),
    }
}

pub fn notify(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, snapshot(app));
    crate::connections::notify(app);
    crate::chat::retry_refused(app);
    crate::tabs::on_accounts_changed(app);
    crate::moderation::feeds::sync(app);
    if app.try_state::<crate::streammanager::State>().is_some() {
        crate::streammanager::accounts_changed(app);
    }
    crate::whispers::accounts_changed(app);
    crate::tabs::live::nudge(app); // V106-17 own channel joins the live check
    crate::bell::accounts_changed(app);
    if app.try_state::<crate::chat::rules::RuleState>().is_some() {
        crate::chat::rules::load_blocked(app.clone());
    }
}

/// The account's usable GQL token and the client ID it was issued to.
pub fn gql_token(app: &AppHandle, user_id: &str) -> Option<(String, String)> {
    token(app, user_id, TokenKind::Gql)
}

/// The account's usable Helix token and the client ID it was issued to.
pub fn helix_token(app: &AppHandle, user_id: &str) -> Option<(String, String)> {
    token(app, user_id, TokenKind::Helix)
}

/// The Helix token, only if it was granted `scope`.
pub fn helix_scoped(app: &AppHandle, user_id: &str, scope: &str) -> Option<(String, String)> {
    let scoped = accounts(app)
        .get(user_id)
        .and_then(|a| a.helix.as_ref())
        .is_some_and(|m| m.scopes.iter().any(|s| s == scope));
    scoped.then(|| helix_token(app, user_id)).flatten()
}

fn token(app: &AppHandle, user_id: &str, kind: TokenKind) -> Option<(String, String)> {
    let auth = app.state::<Auth>();
    auth.vault
        .state(user_id, kind)
        .usable()
        .then(|| auth.vault.secret(user_id, kind))
        .flatten()
        .map(|s| (s.access, s.client_id))
}

/// After settings were restored from a backup: re-read tokens for the restored list.
pub fn reload(app: &AppHandle) {
    for e in app.state::<Auth>().vault.load(&accounts(app)) {
        eprintln!("auth: couldn't read a token: {e}");
    }
    notify(app);
}

pub fn unix_now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
