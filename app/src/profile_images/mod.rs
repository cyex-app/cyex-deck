//! V107-R19 Changes the account's profile picture, banner and offline image from the app:
//! ask Twitch for an upload address, PUT the file, then watch the public image URLs until
//! they change. Twitch's internal GQL, live-tested 2026/10/21 (V107-R19b). The image a change replaces is kept
//! (`backup`) so "Restore previous" can put it back. Mocks builds can point everything at
//! `tools/mock/profile-images.mjs` (`CYEX_GQL_PROFILE_MOCK`, test data folder only).
//! V107-R19c Every image put up here also goes in a short local history (`history`) to switch back to.

mod backup;
mod check;
mod history;

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
/// V107-R19c History thumbs: 2x the 48 px they show at.
const THUMB_SIDE: u32 = 96;
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

    /// V107-R19c How many changed images the history keeps.
    fn keep(self) -> usize {
        match self {
            Self::Avatar => 5,
            Self::Banner | Self::Offline => 3,
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
    /// V107-R19c Images changed here before, newest first.
    pub history: Vec<ProfileImageSaved>,
    /// V107-Mh Twitch has a way to remove it and there's one to remove.
    pub removable: bool,
    /// V107-Mz The channel page shows the accent colour, so "Default", not "None".
    pub accent: bool,
}

/// V107-R19c One image from the history.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileImageSaved {
    pub id: String,
    /// A small inline copy.
    pub thumb: String,
    /// It's the image Twitch shows now.
    pub current: bool,
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

/// The mock's address in a mocks build on a test data folder. Release builds: never. Set but
/// unusable: an error, nothing is sent at all (never Twitch with a real token instead).
#[cfg(feature = "mocks")]
pub fn mock_base(app: &AppHandle) -> Result<Option<String>, String> {
    crate::mock_url::gated(app, &[MOCK_VAR], "profile image")
}

#[cfg(not(feature = "mocks"))]
pub fn mock_base(_: &AppHandle) -> Result<Option<String>, String> {
    Ok(None)
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
    let base = mock_base(app).map_err(|e| ImageError::with(ImageErrorKind::Failed, &e))?;
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
            url: images.shown(kind.api()).to_owned(),
            backup: dir.as_deref().and_then(|d| backup::newest(d, kind.file())).is_some(),
            history: dir.as_deref().map_or_else(Vec::new, |d| saved(d, kind, images.shown(kind.api()))),
            removable: can_remove(kind, images.shown(kind.api())),
            accent: kind == ProfileImageKind::Banner && images.banner_is_accent(),
        })
        .collect();
    ProfileImagesView { slots, waiting }
}

/// V107-R19c The history of a kind, thumbs inline. Entries whose thumb won't read are left out.
fn saved(dir: &std::path::Path, kind: ProfileImageKind, now: &str) -> Vec<ProfileImageSaved> {
    history::list(&history::dir(dir), kind.file())
        .into_iter()
        .filter_map(|s| {
            let bytes = std::fs::read(s.thumb.as_ref().unwrap_or(&s.path)).ok()?;
            let current = !now.is_empty() && history::url_of(&s).is_some_and(|u| u == now);
            Some(ProfileImageSaved { id: s.id, thumb: api::data_url(&bytes), current })
        })
        .collect()
}

/// The three images as they are now, public data.
pub async fn get(app: &AppHandle, account: &str) -> Result<ProfileImagesView, ImageError> {
    let base = mock_base(app).map_err(|e| ImageError::with(ImageErrorKind::Failed, &e))?;
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
    let copy = bytes.clone();
    let (now, waiting) = upload(&t, kind, format, bytes, &before).await?;
    let url = (!waiting).then(|| now.get(kind.api()).to_owned());
    remember(app, account, kind, format, copy, url).await;
    Ok(shown(app, &t, kind, &now, waiting).await)
}

/// V107-R19c Puts an image from the history up again, as a change: no new crop, the original
/// backup rules as for any change.
pub async fn use_saved(app: &AppHandle, account: &str, kind: ProfileImageKind, id: &str) -> Result<ProfileImagesView, ImageError> {
    let gone = || ImageError::with(ImageErrorKind::Failed, "That image isn’t saved anymore.");
    let dir = backup_dir(app, account).map(|d| history::dir(&d)).ok_or_else(gone)?;
    let path = history::find(&dir, kind.file(), id).ok_or_else(gone)?.path;
    let bytes = std::fs::read(path).map_err(|_| gone())?;
    change(app, account, kind, bytes).await
}

/// V107-R19c Drops one image from the history.
pub fn forget(app: &AppHandle, account: &str, kind: ProfileImageKind, id: &str) -> Result<(), ImageError> {
    let dir = backup_dir(app, account).map(|d| history::dir(&d));
    if dir.is_some_and(|d| history::remove(&d, kind.file(), id)) {
        Ok(())
    } else {
        Err(ImageError::with(ImageErrorKind::Failed, "That image isn’t saved anymore."))
    }
}

/// V107-R19c Saves what went up, with a small thumb, and the address Twitch gave it.
/// A failure here never fails the change.
async fn remember(app: &AppHandle, account: &str, kind: ProfileImageKind, format: api::Format, bytes: Vec<u8>, url: Option<String>) {
    let Some(dir) = backup_dir(app, account).map(|d| history::dir(&d)) else { return };
    let saved = tauri::async_runtime::spawn_blocking(move || {
        let thumb = crate::sys::thumb::thumbnail(bytes.clone(), THUMB_SIDE);
        let id = history::add(&dir, kind.file(), format.ext(), now_ms(), &bytes, thumb.as_deref(), kind.keep());
        match (&id, url) {
            (Ok(id), Some(u)) => history::set_url(&dir, kind.file(), id, &u),
            (Err(e), _) => eprintln!("profile images: history: {e}"),
            _ => {}
        }
    })
    .await;
    if let Err(e) = saved {
        eprintln!("profile images: history: {e}");
    }
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
    let (now, waiting) = upload(&t, kind, format, bytes, &before).await?;
    if let Some(dir) = backup_dir(app, account) {
        backup::remove(&dir, kind.file());
    }
    Ok(shown(app, &t, kind, &now, waiting).await)
}

/// V107-Mh Twitch has a way to remove this kind, and `url` isn't already its default.
fn can_remove(kind: ProfileImageKind, url: &str) -> bool {
    api::removable(kind.api()) && !url.is_empty() && !(kind == ProfileImageKind::Avatar && api::is_default_avatar(url))
}

/// V107-Mh Back to Twitch's default (avatar, offline image). The image removed is kept first,
/// as for a change: the original backup when none is kept yet, and Recent, so it can come back.
pub async fn remove(app: &AppHandle, account: &str, kind: ProfileImageKind) -> Result<ProfileImagesView, ImageError> {
    if !api::removable(kind.api()) {
        return Err(ImageError::with(ImageErrorKind::Failed, "Twitch has no way to remove this one."));
    }
    let state = app.state::<State>();
    let _busy = Busy::take(&state, account)?;
    let t = target(app, account)?;
    let before = api::images(&t.http, t.base(), &t.user_id).await.map_err(|e| check::map(&e))?;
    let old = before.get(kind.api()).to_owned();
    if !can_remove(kind, &old) {
        return Ok(shown(app, &t, kind, &before, false).await);
    }
    let bytes = fetch_current(&t, &old).await?;
    if backup_dir(app, account).and_then(|d| backup::newest(&d, kind.file())).is_none() {
        save_backup(app, &t, kind, &bytes)?;
    }
    let in_recent = backup_dir(app, account)
        .is_some_and(|d| history::list(&history::dir(&d), kind.file()).iter().any(|s| history::url_of(s).is_some_and(|u| u == old)));
    if !in_recent && let Some(format) = api::Format::sniff(&bytes) {
        remember(app, account, kind, format, bytes, Some(old.clone())).await;
    }
    api::remove(&t.http, t.auth(), t.base(), kind.api(), &t.user_id)
        .await
        .map_err(|e| log_map("remove", &e))?;
    let (now, waiting) = wait(&t, kind, &before, |url| url != old).await;
    Ok(shown(app, &t, kind, &now, waiting).await)
}

/// The image Twitch shows now. Nothing changes when that fails.
async fn fetch_current(t: &Target, url: &str) -> Result<Vec<u8>, ImageError> {
    t.http
        .get_small(url, api::MAX_BYTES)
        .await
        .ok()
        .flatten()
        .ok_or_else(save_failed)
}

fn save_failed() -> ImageError {
    ImageError::with(ImageErrorKind::Failed, "Couldn’t save the current image, so nothing changed.")
}

fn save_backup(app: &AppHandle, t: &Target, kind: ProfileImageKind, bytes: &[u8]) -> Result<(), ImageError> {
    let ext = api::Format::sniff(bytes).map_or("img", api::Format::ext);
    let dir = backup_dir(app, &t.user_id).ok_or_else(save_failed)?;
    backup::save(&dir, kind.file(), ext, now_ms(), bytes).map(|_| ()).map_err(|_| save_failed())
}

/// Saves the image about to be replaced. Nothing changes when that fails.
async fn keep_current(app: &AppHandle, t: &Target, kind: ProfileImageKind, url: &str) -> Result<(), ImageError> {
    if url.is_empty() {
        return Ok(());
    }
    let bytes = fetch_current(t, url).await?;
    save_backup(app, t, kind, &bytes)
}

/// The images after the upload, and whether Twitch was still showing the old one at the end.
async fn upload(
    t: &Target,
    kind: ProfileImageKind,
    format: api::Format,
    bytes: Vec<u8>,
    before: &api::Images,
) -> Result<(api::Images, bool), ImageError> {
    let up = api::create(&t.http, t.auth(), t.base(), kind.api(), &t.user_id, format)
        .await
        .map_err(|e| log_map("create", &e))?;
    // V107-R19b Host, path and query keys only: never the signature.
    eprintln!("profile images: put to {}", api::redact(&up.url));
    api::put(&t.http, &up.url, bytes).await.map_err(|e| log_map("put", &e))?;
    let old = before.get(kind.api()).to_owned();
    Ok(wait(t, kind, before, |url| !url.is_empty() && url != old).await)
}

/// Polls the public images until `done` takes the kind's URL; `true` when it never did.
async fn wait(t: &Target, kind: ProfileImageKind, before: &api::Images, done: impl Fn(&str) -> bool) -> (api::Images, bool) {
    let start = Instant::now();
    let mut now = before.clone();
    while start.elapsed() < POLL_FOR {
        tokio::time::sleep(POLL_EVERY).await;
        if let Ok(fresh) = api::images(&t.http, t.base(), &t.user_id).await {
            now = fresh;
            if done(now.get(kind.api())) {
                return (now, false);
            }
        }
    }
    (now, true)
}

/// The view after an upload; a new avatar shows everywhere in the app.
async fn shown(app: &AppHandle, t: &Target, kind: ProfileImageKind, now: &api::Images, waiting: bool) -> ProfileImagesView {
    let v = inline(&t.http, t.base(), view(app, &t.user_id, now, waiting)).await;
    if !waiting
        && kind == ProfileImageKind::Avatar
        && let Some(slot) = v.slots.iter().find(|s| s.kind == kind)
    {
        new_avatar(app, &t.user_id, &slot.url);
    }
    v
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "mocks")]
    #[test]
    fn v107_h2_a_bad_mock_address_sends_nothing() {
        let pick = |raw| crate::mock_url::pick(Some(raw), true, "profile image");
        assert!(pick("http://127.0.0.1:5230").is_ok_and(|b| b.is_some()));
        assert!(pick("https://api.twitch.tv").is_err(), "never falls back to real Twitch");
        assert!(pick("http://127.0.0.1:5230/x").is_err());
    }

    #[test]
    fn remove_shows_only_where_twitch_has_a_way() {
        let own = "https://static-cdn.jtvnw.net/jtv_user_pictures/x-profile_image-600x600.png";
        assert!(can_remove(ProfileImageKind::Avatar, own));
        assert!(!can_remove(ProfileImageKind::Avatar, "https://static-cdn.jtvnw.net/user-default-pictures/y-profile_image-300x300.jpg"));
        assert!(!can_remove(ProfileImageKind::Avatar, ""));
        assert!(can_remove(ProfileImageKind::Offline, "https://static-cdn.jtvnw.net/jtv_user_pictures/x-channel_offline_image.png"));
        assert!(!can_remove(ProfileImageKind::Offline, ""));
        assert!(!can_remove(ProfileImageKind::Banner, "https://static-cdn.jtvnw.net/jtv_user_pictures/x-profile_banner.png"));
    }
}
