//! V1051-02/03 The cosmetics menu's backend: one account's public 7TV inventory, kept for
//! a few minutes, and picks that need the account's 7TV session. Nothing here calls 7TV
//! before the account's consent (`NoConsent` is answered from settings alone).

mod apply;
mod combos;
pub mod lookup;
mod paste;
mod select;
mod status;
mod vars;
mod words;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_auth::{SevenTvToken, SevenTvTokens, seventv_date};
use cyex_emotes::seventv::account::model::Inventory;
use cyex_emotes::seventv::account::view::{CosmeticsView, Status};
use cyex_emotes::seventv::account::{Client, Error};
use cyex_settings::schema::SevenTvConsent;
use tauri::{AppHandle, Manager};

pub use apply::{Applied, Part, Want, apply};
pub use paste::paste;
pub use combos::{change as change_combos, find as find_combo, list as list_combos};
pub use words::{select_error, why};
pub use select::{Kind, Picked, SelectError, select};
pub(crate) use select::forget_if_same;
pub use vars::VarScope;
use status::status;

use crate::auth::{self, Auth};
use crate::emotes::presence;
use crate::windows::seventv::SignIn;

/// How long the parsed inventory is kept after it was read from 7TV.
const KEEP: Duration = Duration::from_secs(5 * 60);
/// Test runs only: a local 7TV mock (`http://127.0.0.1:<port>`) and a fake session for the default account.
#[cfg(feature = "mocks")]
const URL_VAR: &str = "CYEX_7TV_URL";
#[cfg(feature = "mocks")]
const TOKEN_VAR: &str = "CYEX_7TV_TEST_TOKEN";
/// V106-31/32 `mock`: emote reads go to the 7TV mock too.
#[cfg(feature = "mocks")]
const SETS_VAR: &str = "CYEX_7TV_SETS";

type Gate = Arc<tokio::sync::Mutex<()>>;

pub struct Cosmetics {
    api: Client,
    /// By Twitch id.
    cache: Mutex<HashMap<String, (Instant, Arc<Inventory>)>>,
    /// The latest pick per account and kind (a counter).
    latest: Mutex<HashMap<(String, Kind), u64>>,
    /// One call in flight per account and kind.
    gates: Mutex<HashMap<(String, Kind), Gate>>,
    /// Test runs: print how long a parse takes (V1051-03 budget B2).
    timed: bool,
    /// V106-31/32 The client points at the loopback mock (test runs only).
    mocked: bool,
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
            timed: false,
            mocked: false,
        }
    }

    fn cached(&self, account: &str) -> Option<(Instant, Arc<Inventory>)> {
        lock(&self.cache).get(account).cloned()
    }

    fn clear(&self, account: &str) {
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
        let answer = self.api.inventory(account).await?;
        let t = Instant::now();
        let Some(inv) = answer.read()? else {
            lock(&self.cache).remove(account);
            return Ok(None);
        };
        let inv = Arc::new(inv);
        if self.timed {
            let n = inv.paints.len() + inv.badges.len();
            eprintln!("cosmetics: parsed {n} in {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
        }
        lock(&self.cache).insert(account.to_owned(), (Instant::now(), inv.clone()));
        Ok(Some(inv))
    }
}

/// V1051-04 The inventory in the cache, if any; never fetches.
pub fn cached(app: &AppHandle, account: &str) -> Option<Arc<Inventory>> {
    state(app).cached(account).map(|(_, i)| i)
}

fn state(app: &AppHandle) -> tauri::State<'_, Cosmetics> {
    app.state::<Cosmetics>()
}

/// V106-31/32 The 7TV client (the mock's in test runs that set one).
pub(crate) fn api(app: &AppHandle) -> Client {
    state(app).api.clone()
}

/// V106-31/32 Edits go out only when this is true in a test run.
pub(crate) fn mocked(app: &AppHandle) -> bool {
    state(app).mocked
}

pub(crate) fn has_consent(app: &AppHandle, account: &str) -> bool {
    auth::accounts(app)
        .get(account)
        .is_some_and(|a| a.seventv.is_some())
}

/// After `auth::init` and `emotes::init`.
pub fn init(app: &AppHandle) {
    let http = app.state::<Auth>().http.clone();
    let test = crate::paths::webview(app).is_some();
    #[cfg(feature = "mocks")]
    let mock = mock_7tv(app, test);
    #[cfg(not(feature = "mocks"))]
    let mock: Option<String> = None;
    let mocked = mock.is_some();
    let api = match mock {
        Some(url) => Client::at(http, &url),
        None => Client::new(http),
    };
    app.manage(Cosmetics {
        timed: test,
        mocked,
        ..Cosmetics::new(api)
    });
}

/// R10 The local 7TV mock of a test run (mocks builds only), with the default account's
/// fake session and, for `CYEX_7TV_SETS=mock`, emote reads pointed at it too.
#[cfg(feature = "mocks")]
fn mock_7tv(app: &AppHandle, test: bool) -> Option<String> {
    let mock = std::env::var(URL_VAR)
        .ok()
        .filter(|_| test)
        .and_then(|u| crate::mock_url::loopback_base(&u));
    if let Some(url) = &mock
        && let Ok(raw) = std::env::var(TOKEN_VAR)
        && let Some(token) = SevenTvToken::parse(&raw)
        && let Some(id) = auth::accounts(app).default_id
    {
        let _ = app.state::<Auth>().seventv.put(&id, &token);
        eprintln!("cosmetics: mock 7TV at {url}");
    }
    // V106-31/32 Emote reads go to the mock too.
    if let Some(url) = &mock
        && std::env::var(SETS_VAR).as_deref() == Ok("mock")
    {
        cyex_emotes::seventv::api::use_mock(url);
    }
    mock
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
        Some(Ok(Some(inv))) => {
            // Chat lookups (/paint, {paint}, Tab) let it go like the menu does.
            release_later(app, account);
            Ok(inv)
        }
        Some(Ok(None)) => Err(Why::NoAccount),
        Some(Err(e)) => {
            eprintln!("cosmetics: {account}: {e}");
            Err(Why::Down)
        }
    }
}

/// What the menu shows. `fresh` skips the cache.
pub async fn view(app: &AppHandle, account: &str, fresh: bool) -> CosmeticsView {
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
    if now == Status::SignedOut && app.state::<Auth>().seventv.rejected(account) {
        now = Status::Expired;
    }
    let until = token.as_ref().and_then(SevenTvToken::expires).map(seventv_date);
    release_later(app, account);
    match inv {
        Ok(Some(i)) => CosmeticsView::from_inventory(&i, now, until),
        _ => CosmeticsView::empty(now),
    }
}

/// Drops the inventory 5 minutes after it was read (a newer read keeps its own copy).
pub(super) fn release_later(app: &AppHandle, account: &str) {
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
        discard(st.api.clone(), t);
    }
}

/// V1051-02 7TV's own sign-in in a small private window. Needs the account's consent. The
/// session is kept only if its Twitch connection is this account's. V108-7T `shared`
/// ("Open in: Browser") borrows the account's Twitch pages profile only while it's connected.
pub async fn sign_in(app: &AppHandle, account: &str, shared: bool) -> SignIn {
    if !has_consent(app, account) {
        return SignIn::Failed;
    }
    let pages = crate::windows::twitch::connected(app).iter().any(|id| id == account);
    let shared = crate::windows::seventv::borrows_pages(shared, pages);
    let api = state(app).api.clone();
    let token = match crate::windows::seventv::capture(app, account, api.login_url(), shared).await {
        Ok(t) => t,
        Err(done) => return done,
    };
    let me = match api.me(token.secret()).await {
        Ok(me) => me,
        Err(e) => {
            eprintln!("cosmetics: {account}: sign-in check: {e}");
            discard(api, token);
            return SignIn::Failed;
        }
    };
    if me.twitch_id.as_deref() != Some(account) {
        discard(api, token);
        return SignIn::OtherAccount;
    }
    // Withdrawn while the window was open: the session isn't kept.
    if !has_consent(app, account) {
        discard(api, token);
        return SignIn::Cancelled;
    }
    if let Err(e) = app.state::<Auth>().seventv.put(account, &token) {
        eprintln!("cosmetics: {account}: couldn't keep the 7TV session: {e}");
        discard(api, token);
        return SignIn::Failed;
    }
    presence::remember_seventv(app, account, Some(me.id));
    auth::notify(app);
    SignIn::Done
}

/// Ends a session we no longer keep on 7TV's side too, in the background (best effort).
fn discard(api: Client, token: SevenTvToken) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = api.logout(token.secret()).await {
            eprintln!("cosmetics: 7TV logout: {e}");
        }
    });
}
