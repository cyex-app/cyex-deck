//! V107-R19 Changes the account's profile picture, banner and offline image from the app:
//! ask Twitch for an upload address, PUT the file, then watch the public image URLs until
//! they change. Twitch's internal GQL, live-tested 2026/10/21 (V107-R19b). The image a change replaces is kept
//! (`backup`) so "Restore previous" can put it back. Mocks builds can point everything at
//! `tools/mock/profile-images.mjs` (`CYEX_GQL_PROFILE_MOCK`, test data folder only).

mod backup;
mod check;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cyex_twitch::Http;
use cyex_twitch::gql::{self, profile_images as api};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

pub use check::{ImageError, ImageErrorKind};

use crate::auth;

const NO_GQL: &str = "This needs a GQL sign-in for the account (Settings → Accounts).";
const POLL_FOR: Duration = Duration::from_secs(15);
const POLL_EVERY: Duration = Duration::from_millis(1500);
#[cfg(feature = "mocks")]
const MOCK_VAR: &str = "CYEX_GQL_PROFILE_MOCK";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ProfileImageKind {
    Avatar,
    Banner,
    Offline,
}

impl ProfileImageKind {
    const ALL: [Self; 3] = [Self::Avatar, Self::Banner, Self::Offline];

    fn api(self) -> api::ImageKind {
        match self {
            Self::Avatar => api::ImageKind::Avatar,
            Self::Banner => api::ImageKind::Banner,
            Self::Offline => api::ImageKind::Offline,
        }
    }

    fn file(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::Banner => "banner",
            Self::Offline => "offline",
        }
    }
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileImageSlot {
    pub kind: ProfileImageKind,
    /// Empty when not set.
    pub url: String,
    /// The image the app first replaced, which "Restore original" puts back.
    pub backup: bool,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileImagesView {
    pub slots: Vec<ProfileImageSlot>,
    /// The upload went through but Twitch didn't show the new image within 15 s.
    pub waiting: bool,
}

#[derive(Default)]
pub struct State {
    /// Accounts with an upload running.
    busy: Mutex<HashSet<String>>,
}

pub fn init(app: &AppHandle) {
    app.manage(State::default());
}

/// The mock's address in a mocks build on a test data folder. Release builds: never.
#[cfg(feature = "mocks")]
pub fn mock_base(app: &AppHandle) -> Option<String> {
    crate::paths::webview(app)?;
    crate::mock_url::loopback_base(&std::env::var(MOCK_VAR).ok()?)
}

#[cfg(not(feature = "mocks"))]
pub fn mock_base(_: &AppHandle) -> Option<String> {
    None
}

struct Target {
    http: Http,
    token: String,
    client_id: String,
    base: Option<String>,
    user_id: String,
}

impl Target {
    fn auth(&self) -> gql::Auth<'_> {
        auth::gql_auth(&self.token, &self.client_id)
    }

    fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }
}

/// The account's GQL sign-in, or a fake one aimed at the mock (never a real token there).
fn target(app: &AppHandle, account: &str) -> Result<Target, ImageError> {
    if auth::accounts(app).get(account).is_none() {
        return Err(ImageError::with(ImageErrorKind::Failed, "That account isn’t here anymore."));
    }
    let base = mock_base(app);
    let (token, client_id) = match &base {
        Some(_) => ("mock-token".to_owned(), cyex_twitch::clients::WEB.to_owned()),
        None => auth::gql_token(app, account).ok_or_else(|| ImageError::with(ImageErrorKind::Failed, NO_GQL))?,
    };
    Ok(Target {
        http: app.state::<auth::Auth>().http.clone(),
        token,
        client_id,
        base,
        user_id: account.to_owned(),
    })
}

fn backup_dir(app: &AppHandle, account: &str) -> Option<PathBuf> {
    backup::dir(&crate::paths::local(app).ok()?.join("profile-backup"), account)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn view(app: &AppHandle, account: &str, images: &api::Images, waiting: bool) -> ProfileImagesView {
    let dir = backup_dir(app, account);
    let slots = ProfileImageKind::ALL
        .iter()
        .map(|&kind| ProfileImageSlot {
            kind,
            url: images.get(kind.api()).to_owned(),
            backup: dir.as_deref().and_then(|d| backup::newest(d, kind.file())).is_some(),
        })
        .collect();
    ProfileImagesView { slots, waiting }
}

/// The three images as they are now, public data.
pub async fn get(app: &AppHandle, account: &str) -> Result<ProfileImagesView, ImageError> {
    let base = mock_base(app);
    let http = app.state::<auth::Auth>().http.clone();
    let images = api::images(&http, base.as_deref(), account).await.map_err(|e| check::map(&e))?;
    Ok(inline(&http, base.as_deref(), view(app, account, &images, false)).await)
}

/// Test runs: the window only loads https images, so the mock's come inline.
async fn inline(http: &Http, base: Option<&str>, mut v: ProfileImagesView) -> ProfileImagesView {
    let Some(base) = base else { return v };
    for slot in &mut v.slots {
        if slot.url.starts_with(base)
            && let Ok(Some(bytes)) = http.get_small(&slot.url, api::MAX_BYTES).await
        {
            slot.url = api::data_url(&bytes);
        }
    }
    v
}

/// One upload per account at a time.
struct Busy<'a>(&'a State, String);

impl<'a> Busy<'a> {
    fn take(state: &'a State, account: &str) -> Result<Self, ImageError> {
        let mut busy = state.busy.lock().unwrap_or_else(|e| e.into_inner());
        if !busy.insert(account.to_owned()) {
            return Err(ImageError::with(ImageErrorKind::Failed, "Still uploading the last one."));
        }
        Ok(Self(state, account.to_owned()))
    }
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.busy.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.1);
    }
}

/// Puts a new image up. The current one is saved first, unless an original is kept already:
/// that stays until a restore succeeds, across runs.
pub async fn change(app: &AppHandle, account: &str, kind: ProfileImageKind, bytes: Vec<u8>) -> Result<ProfileImagesView, ImageError> {
    let format = check::validate(&bytes)?;
    let state = app.state::<State>();
    let _busy = Busy::take(&state, account)?;
    let t = target(app, account)?;
    let before = api::images(&t.http, t.base(), &t.user_id).await.map_err(|e| check::map(&e))?;
    let kept = backup_dir(app, account).and_then(|d| backup::newest(&d, kind.file())).is_some();
    if !kept {
        keep_current(app, &t, kind, before.get(kind.api())).await?;
    }
    upload(app, &t, kind, format, bytes, &before).await
}

/// Re-uploads the original. Once that worked the backup goes, so the next change saves a fresh one.
pub async fn restore(app: &AppHandle, account: &str, kind: ProfileImageKind) -> Result<ProfileImagesView, ImageError> {
    let none = || ImageError::with(ImageErrorKind::Failed, "No earlier image saved.");
    let path = backup_dir(app, account)
        .and_then(|d| backup::newest(&d, kind.file()))
        .ok_or_else(none)?;
    let bytes = std::fs::read(&path).map_err(|_| none())?;
    let format = check::validate(&bytes)?;
    let state = app.state::<State>();
    let _busy = Busy::take(&state, account)?;
    let t = target(app, account)?;
    let before = api::images(&t.http, t.base(), &t.user_id).await.map_err(|e| check::map(&e))?;
    let mut v = upload(app, &t, kind, format, bytes, &before).await?;
    if let Some(dir) = backup_dir(app, account) {
        backup::remove(&dir, kind.file());
    }
    for slot in v.slots.iter_mut().filter(|s| s.kind == kind) {
        slot.backup = false;
    }
    Ok(v)
}

/// Saves the image about to be replaced. Nothing changes when that fails.
async fn keep_current(app: &AppHandle, t: &Target, kind: ProfileImageKind, url: &str) -> Result<(), ImageError> {
    if url.is_empty() {
        return Ok(());
    }
    let failed = || ImageError::with(ImageErrorKind::Failed, "Couldn’t save the current image, so nothing changed.");
    let bytes = t
        .http
        .get_small(url, api::MAX_BYTES)
        .await
        .ok()
        .flatten()
        .ok_or_else(failed)?;
    let ext = api::Format::sniff(&bytes).map_or("img", api::Format::ext);
    let dir = backup_dir(app, &t.user_id).ok_or_else(failed)?;
    backup::save(&dir, kind.file(), ext, now_ms(), &bytes).map_err(|_| failed())?;
    Ok(())
}

async fn upload(
    app: &AppHandle,
    t: &Target,
    kind: ProfileImageKind,
    format: api::Format,
    bytes: Vec<u8>,
    before: &api::Images,
) -> Result<ProfileImagesView, ImageError> {
    let up = api::create(&t.http, t.auth(), t.base(), kind.api(), &t.user_id, format)
        .await
        .map_err(|e| log_map("create", &e))?;
    // V107-R19b Host, path and query keys only: never the signature.
    eprintln!("profile images: put to {}", api::redact(&up.url));
    api::put(&t.http, &up.url, bytes).await.map_err(|e| log_map("put", &e))?;
    let old = before.get(kind.api()).to_owned();
    let start = Instant::now();
    let mut now = before.clone();
    while start.elapsed() < POLL_FOR {
        tokio::time::sleep(POLL_EVERY).await;
        if let Ok(fresh) = api::images(&t.http, t.base(), &t.user_id).await {
            now = fresh;
            let url = now.get(kind.api());
            if !url.is_empty() && url != old {
                let v = inline(&t.http, t.base(), view(app, &t.user_id, &now, false)).await;
                if kind == ProfileImageKind::Avatar
                    && let Some(slot) = v.slots.iter().find(|s| s.kind == kind)
                {
                    new_avatar(app, &t.user_id, &slot.url);
                }
                return Ok(v);
            }
        }
    }
    Ok(inline(&t.http, t.base(), view(app, &t.user_id, &now, true)).await)
}

fn log_map(step: &str, e: &cyex_twitch::Error) -> ImageError {
    eprintln!("profile images: {step}: {e}");
    check::map(e)
}

/// The account's avatar everywhere in the app.
fn new_avatar(app: &AppHandle, account: &str, url: &str) {
    let changed = auth::update_accounts(app, |a| match a.get_mut(account) {
        Some(acc) if acc.avatar != url => {
            url.clone_into(&mut acc.avatar);
            true
        }
        _ => false,
    });
    if changed.is_ok() {
        auth::notify(app);
    }
}
