//! V105-20 Update checks: GitHub's releases at startup and every 8 hours (V1061: was 12), conditional requests, silent when offline. Sends nothing about the user, only the normal request.

pub mod install;
pub mod sign;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cyex_core::version;
use cyex_settings::Partition;
use cyex_settings::schema::General;
use cyex_twitch::github::{self, Fetched, Update};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::auth::Auth;
use crate::settings::Settings;

const AVAILABLE: &str = "update://available";
const FIRST_CHECK: Duration = Duration::from_secs(10);
const EVERY: Duration = Duration::from_secs(8 * 60 * 60);
const RATE_LIMIT_WAIT: i64 = 60 * 60;

#[derive(Default)]
struct State {
    /// The newest release above the running version (None: you're current or it's skipped).
    available: Option<Update>,
    etag: Option<String>,
    /// The last 200 answer, reused when GitHub says 304.
    body: Option<Vec<u8>>,
    /// Unix seconds before which timed checks stay quiet (rate limited).
    wait_until: Option<i64>,
}

#[derive(Default)]
pub struct Updater {
    state: Mutex<State>,
    /// An update is downloading.
    busy: AtomicBool,
    /// "Later": no timed checks until the app starts again.
    later: AtomicBool,
}

impl Updater {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CheckResult {
    Newest,
    Available { update: Update },
    Failed,
}

pub fn init(app: &AppHandle) {
    app.manage(Updater::default());
    // Leftovers of an earlier update.
    let _ = std::fs::remove_dir_all(install::temp_dir());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            let on = app.state::<Settings>().read::<General>(Partition::General).updates.check;
            if on && !app.state::<Updater>().later.load(Ordering::Relaxed) {
                check(&app, false).await;
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

/// "Later" in the pop-over: the timed checks stop until the next start, which asks again.
pub fn later(app: &AppHandle) {
    app.state::<Updater>().later.store(true, Ordering::Relaxed);
}

pub fn available(app: &AppHandle) -> Option<Update> {
    app.state::<Updater>().lock().available.clone()
}

/// What the badge needs after a reload or in another window: the release and "Later" (B4).
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateState {
    pub update: Option<Update>,
    pub later: bool,
}

pub fn state(app: &AppHandle) -> UpdateState {
    let updater = app.state::<Updater>();
    UpdateState {
        update: updater.lock().available.clone(),
        later: updater.later.load(Ordering::Relaxed),
    }
}

/// `update://available` goes to the main window, which owns the badge.
pub fn set_available(app: &AppHandle, update: Option<Update>) {
    {
        let updater = app.state::<Updater>();
        let mut st = updater.lock();
        if st.available == update {
            return;
        }
        st.available = update.clone();
    }
    let _ = app.emit_to(crate::windows::MAIN, AVAILABLE, update);
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn releases_url() -> String {
    // V105-20 R10 Tests point the check at a local mock (mocks builds only).
    #[cfg(feature = "mocks")]
    if let Ok(url) = std::env::var("CYEX_UPDATE_URL") {
        return url;
    }
    github::RELEASES.to_owned()
}

/// A timed check (`manual` false) honors the rate-limit wait and the skipped version; "Check
/// now" ignores both. Nothing is logged but the status.
pub async fn check(app: &AppHandle, manual: bool) -> CheckResult {
    let http = app.state::<Auth>().http.clone();
    let (etag, wait) = {
        let updater = app.state::<Updater>();
        let st = updater.lock();
        (st.etag.clone(), st.wait_until)
    };
    if !manual && wait.is_some_and(|w| now() < w) {
        return CheckResult::Failed;
    }
    let body = match http.get_conditional(&releases_url(), etag.as_deref()).await {
        Ok(Fetched::Body { etag, body }) => {
            eprintln!("update check: 200");
            let updater = app.state::<Updater>();
            let mut st = updater.lock();
            st.etag = etag;
            st.body = Some(body.clone());
            st.wait_until = None;
            body
        }
        Ok(Fetched::NotModified) => {
            eprintln!("update check: 304");
            match app.state::<Updater>().lock().body.clone() {
                Some(b) => b,
                None => return CheckResult::Failed,
            }
        }
        Ok(Fetched::RateLimited { reset_unix }) => {
            eprintln!("update check: rate limited");
            let until = reset_unix.unwrap_or_else(|| now() + RATE_LIMIT_WAIT);
            app.state::<Updater>().lock().wait_until = Some(until);
            return CheckResult::Failed;
        }
        Err(_) => {
            eprintln!("update check: failed");
            return CheckResult::Failed;
        }
    };
    let u = app.state::<Settings>().read::<General>(Partition::General).updates;
    let skip = if manual { None } else { version::parse(&u.skip) };
    match github::pick(&body, &version::current(), u.prereleases, skip.as_ref()) {
        Ok(Some(update)) => {
            set_available(app, Some(update.clone()));
            CheckResult::Available { update }
        }
        Ok(None) => {
            set_available(app, None);
            CheckResult::Newest
        }
        Err(_) => CheckResult::Failed,
    }
}
