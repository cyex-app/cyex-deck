//! V1051-02/03 The cosmetics menu's backend: one account's public 7TV inventory, kept for
//! a few minutes, and picks that need the account's 7TV session. Nothing here calls 7TV
//! before the account's consent (`NoConsent` is answered from settings alone).

mod apply;
mod combos;
mod lookup;
mod select;
mod status;
mod vars;
mod words;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_auth::{SevenTvToken, SevenTvTokens, seventv_date};
use cyex_emotes::seventv::account::model::{self, Inventory};
use cyex_emotes::seventv::account::view::{CosmeticsView, Status};
use cyex_emotes::seventv::account::{Client, Error};
use cyex_settings::schema::SevenTvConsent;
use tauri::{AppHandle, Manager};

pub use apply::{Applied, Want, apply};
pub use combos::change as change_combos;
pub use select::{Kind, Picked, SelectError, select};
pub use vars::VarScope;
use status::status;

use crate::auth::{self, Auth};
use crate::emotes::presence;
use crate::windows::seventv::SignIn;

/// How long the parsed inventory is kept after its last use.
const KEEP: Duration = Duration::from_secs(5 * 60);
/// Test runs only: a local 7TV mock (`http://127.0.0.1:<port>`) and a fake session for the default account.
const URL_VAR: &str = "CYEX_7TV_URL";
const TOKEN_VAR: &str = "CYEX_7TV_TEST_TOKEN";

type Gate = Arc<tokio::sync::Mutex<()>>;

pub struct Cosmetics {
    api: Client,
    /// By Twitch id.
    cache: Mutex<HashMap<String, (Instant, Arc<Inventory>)>>,
    /// The latest pick per account and kind (a counter).
    latest: Mutex<HashMap<(String, Kind), u64>>,
    /// One call in flight per account and kind.
    gates: Mutex<HashMap<(String, Kind), Gate>>,
    /// Accounts whose 7TV session was rejected: the UI says "Sign in again".
    expired: Mutex<HashSet<String>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl Cosmetics {
    pub fn new(api: Client) -> Self {
        Self {
            api,
            cache: Mutex::default(),
            latest: Mutex::default(),
            gates: Mutex::default(),
            expired: Mutex::default(),
        }
    }

    fn cached(&self, account: &str) -> Option<(Instant, Arc<Inventory>)> {
        lock(&self.cache).get(account).cloned()
    }

    fn mark_expired(&self, account: &str) {
        lock(&self.expired).insert(account.to_owned());
    }

    fn clear(&self, account: &str) {
        lock(&self.expired).remove(account);
        lock(&self.cache).remove(account);
    }

    /// The inventory from the cache (unless `fresh`) or from 7TV.
    async fn inventory(&self, account: &str, fresh: bool) -> Result<Option<Arc<Inventory>>, Error> {
        if !fresh
            && let Some((at, inv)) = self.cached(account)
            && at.elapsed() < KEEP
        {
            return Ok(Some(inv));
        }
        let Some(raw) = self.api.inventory(account).await? else {
            lock(&self.cache).remove(account);
            return Ok(None);
        };
        let inv = Arc::new(model::parse_user(&raw)?.ok_or(Error::Bad)?);
        lock(&self.cache).insert(account.to_owned(), (Instant::now(), inv.clone()));
        Ok(Some(inv))
    }
}

fn state(app: &AppHandle) -> tauri::State<'_, Cosmetics> {
    app.state::<Cosmetics>()
}

fn has_consent(app: &AppHandle, account: &str) -> bool {
    auth::accounts(app)
        .get(account)
        .is_some_and(|a| a.seventv.is_some())
}

/// After `auth::init` and `emotes::init`.
pub fn init(app: &AppHandle) {
    let http = app.state::<Auth>().http.clone();
    let test = crate::paths::webview(app).is_some();
    let mock = std::env::var(URL_VAR)
        .ok()
        .filter(|u| test && u.starts_with("http://127.0.0.1:"));
    if let Some(url) = &mock
        && let Ok(raw) = std::env::var(TOKEN_VAR)
        && let Some(token) = SevenTvToken::parse(&raw)
        && let Some(id) = auth::accounts(app).default_id
    {
        let _ = app.state::<Auth>().seventv.put(&id, &token);
        eprintln!("cosmetics: mock 7TV at {url}");
    }
    let api = match mock {
        Some(url) => Client::at(http, &url),
        None => Client::new(http),
    };
    app.manage(Cosmetics::new(api));
}

/// Why `read` has no inventory.
#[derive(Debug, PartialEq, Eq)]
pub enum Why {
    NoConsent,
    NoAccount,
    Down,
}

/// The one cache-or-fetch path. `None` = no consent (checked first, so 7TV is never called).
async fn fetch(
    app: &AppHandle,
    account: &str,
    fresh: bool,
) -> Option<Result<Option<Arc<Inventory>>, Error>> {
    if !has_consent(app, account) {
        return None;
    }
    Some(state(app).inventory(account, fresh).await)
}

/// V1051-04 The account's inventory, from the 5-minute cache or 7TV. Never calls 7TV
/// without consent.
pub async fn read(app: &AppHandle, account: &str) -> Result<Arc<Inventory>, Why> {
    match fetch(app, account, false).await {
        None => Err(Why::NoConsent),
        Some(Ok(Some(inv))) => Ok(inv),
        Some(Ok(None)) => Err(Why::NoAccount),
        Some(Err(e)) => {
            eprintln!("cosmetics: {account}: {e}");
            Err(Why::Down)
        }
    }
}

/// What the menu shows. `fresh` skips the cache.
pub async fn view(app: &AppHandle, account: &str, fresh: bool) -> CosmeticsView {
    let st = state(app);
    let Some(inv) = fetch(app, account, fresh).await else {
        return CosmeticsView::empty(Status::NoConsent);
    };
    match &inv {
        Ok(found) => {
            let id = found.as_ref().map(|i| i.seventv_id.clone());
            presence::remember_seventv(app, account, id);
        }
        Err(e) => eprintln!("cosmetics: {account}: {e}"),
    }
    let token = app.state::<Auth>().seventv.get(account);
    let mut now = status(
        true,
        inv.as_ref().map(|o| o.as_deref()),
        token.as_ref(),
        auth::unix_now(),
    );
    if now == Status::SignedOut && lock(&st.expired).contains(account) {
        now = Status::Expired;
    }
    let until = token.as_ref().and_then(SevenTvToken::expires).map(seventv_date);
    release_later(app, account);
    match inv {
        Ok(Some(i)) => CosmeticsView::from_inventory(&i, now, until),
        _ => CosmeticsView::empty(now),
    }
}

/// Drops the inventory 5 minutes after its last use.
fn release_later(app: &AppHandle, account: &str) {
    let Some((at, _)) = state(app).cached(account) else {
        return;
    };
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(KEEP).await;
        let st = state(&app);
        let mut cache = lock(&st.cache);
        if cache.get(&account).is_some_and(|(t, _)| *t == at) {
            cache.remove(&account);
        }
    });
}

/// The user said yes to 7TV for this account.
pub async fn allow(app: &AppHandle, account: &str) -> CosmeticsView {
    let since = seventv_date(auth::unix_now()).replace('/', "-");
    let saved = auth::update_accounts(app, |a| match a.list.iter_mut().find(|x| x.user_id == account) {
        Some(x) if x.seventv.is_none() => {
            x.seventv = Some(SevenTvConsent { since });
            true
        }
        _ => false,
    });
    if let Err(e) = saved {
        eprintln!("cosmetics: {account}: couldn't save consent: {e}");
    }
    auth::notify(app);
    view(app, account, true).await
}

/// Takes the account's 7TV token out of the vault and returns it.
fn drop_session(tokens: &SevenTvTokens, account: &str) -> Option<SevenTvToken> {
    let token = tokens.get(account);
    if let Err(e) = tokens.forget(account) {
        eprintln!("cosmetics: {account}: couldn't delete the 7TV session: {e}");
    }
    token
}

/// Ends the session here and on 7TV (best effort). Doesn't notify.
async fn end_session(app: &AppHandle, account: &str) {
    let token = drop_session(&app.state::<Auth>().seventv, account);
    let st = state(app);
    st.clear(account);
    if let Some(t) = token
        && let Err(e) = st.api.logout(t.secret()).await
    {
        eprintln!("cosmetics: {account}: logout: {e}");
    }
}

pub async fn sign_out(app: &AppHandle, account: &str) {
    end_session(app, account).await;
    auth::notify(app);
}

/// Sign out, then take the consent back.
pub async fn withdraw(app: &AppHandle, account: &str) {
    end_session(app, account).await;
    let saved = auth::update_accounts(app, |a| match a.list.iter_mut().find(|x| x.user_id == account) {
        Some(x) if x.seventv.is_some() => {
            x.seventv = None;
            true
        }
        _ => false,
    });
    if let Err(e) = saved {
        eprintln!("cosmetics: {account}: couldn't save: {e}");
    }
    auth::notify(app);
}

/// The account is being removed: its 7TV session goes at once (logout follows). The
/// consent leaves with the roster entry.
pub fn forget_account(app: &AppHandle, account: &str) {
    let token = drop_session(&app.state::<Auth>().seventv, account);
    let Some(st) = app.try_state::<Cosmetics>() else {
        return;
    };
    st.clear(account);
    if let Some(t) = token {
        let api = st.api.clone();
        let account = account.to_owned();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = api.logout(t.secret()).await {
                eprintln!("cosmetics: {account}: logout: {e}");
            }
        });
    }
}

/// V1051-02 7TV's own sign-in in a small private window. Needs the account's consent. The
/// session is kept only if its Twitch connection is this account's.
pub async fn sign_in(app: &AppHandle, account: &str) -> SignIn {
    if !has_consent(app, account) {
        return SignIn::Failed;
    }
    let api = state(app).api.clone();
    let token = match crate::windows::seventv::capture(app, account, api.login_url()).await {
        Ok(t) => t,
        Err(done) => return done,
    };
    let me = match api.me(token.secret()).await {
        Ok(me) => me,
        Err(e) => {
            eprintln!("cosmetics: {account}: sign-in check: {e}");
            return SignIn::Failed;
        }
    };
    if me.twitch_id.as_deref() != Some(account) {
        return SignIn::OtherAccount;
    }
    if let Err(e) = app.state::<Auth>().seventv.put(account, &token) {
        eprintln!("cosmetics: {account}: couldn't keep the 7TV session: {e}");
        return SignIn::Failed;
    }
    lock(&state(app).expired).remove(account);
    presence::remember_seventv(app, account, Some(me.id));
    auth::notify(app);
    SignIn::Done
}
