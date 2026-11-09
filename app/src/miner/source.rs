//! V110-U68 Where the miner's drop data comes from: Twitch, or in mocks builds the fixture
//! folder `CYEX_DROPS_MOCK` names (`drops_mock.rs`). Set but refused (no test data folder),
//! every read fails: nothing goes to Twitch instead.

use cyex_twitch::gql::Auth;
use cyex_twitch::gql::drops::{self, Campaign, Inventory};
use cyex_twitch::{Http, Result};
use tauri::AppHandle;

#[cfg(feature = "mocks")]
macro_rules! mocked {
    ($app:expr, |$dir:ident| $body:expr) => {
        if let Some(found) = super::drops_mock::dir($app) {
            return match found {
                Ok($dir) => $body,
                Err(e) => Err(super::drops_mock::refused(e)),
            };
        }
    };
}

#[cfg(not(feature = "mocks"))]
macro_rules! mocked {
    ($app:expr, |$dir:ident| $body:expr) => {
        let _ = $app;
    };
}

/// Ids of the campaigns running now.
pub async fn dashboard(app: &AppHandle, http: &Http, auth: Auth<'_>) -> Result<Vec<String>> {
    mocked!(app, |dir| super::drops_mock::dashboard(&dir));
    drops::dashboard(http, auth).await
}

pub async fn details(
    app: &AppHandle,
    http: &Http,
    auth: Auth<'_>,
    user_id: &str,
    ids: &[String],
) -> Result<Vec<Campaign>> {
    mocked!(app, |dir| super::drops_mock::details(&dir, ids));
    drops::details(http, auth, user_id, ids).await
}

pub async fn inventory(app: &AppHandle, http: &Http, auth: Auth<'_>) -> Result<Inventory> {
    mocked!(app, |dir| super::drops_mock::inventory(&dir));
    drops::inventory_full(http, auth).await
}

/// Campaign ids per channel id.
pub async fn channels(
    app: &AppHandle,
    http: &Http,
    auth: Auth<'_>,
    ids: &[String],
) -> Result<Vec<(String, Vec<String>)>> {
    mocked!(app, |dir| super::drops_mock::channels(&dir, ids));
    drops::channels_campaigns(http, auth, ids).await
}

pub async fn claim(app: &AppHandle, http: &Http, auth: Auth<'_>, instance: &str) -> Result<bool> {
    mocked!(app, |dir| super::drops_mock::claim(&dir, instance));
    drops::claim(http, auth, instance).await
}

/// Candidate helper channels (id, login) the mock names instead of a search; `None` when
/// no mock is set (search Twitch). With the mock set and no list, there are none: a test
/// never watches strangers it didn't name.
pub fn mock_helpers(app: &AppHandle) -> Option<Vec<(String, String)>> {
    #[cfg(feature = "mocks")]
    if let Some(found) = super::drops_mock::dir(app) {
        return Some(found.map(|d| super::drops_mock::helpers(&d)).unwrap_or_default());
    }
    let _ = app;
    None
}
