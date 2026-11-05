//! V109-T01 Themes shared by link on cyex.app: share, update and stop sharing your own;
//! add one from a link, get its updates, go back to a version you had. The settings side
//! (`ThemeLink`) is in `cyex_settings`; the edit keys are in `keys`.

mod api;
mod form;
mod keys;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use cyex_auth::secrets::{KeyringStore, MemoryStore, SecretStore};
use cyex_settings::Partition;
use cyex_settings::schema::{Appearance, CustomTheme, valid_link_id};
use cyex_settings::theme_code;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::auth::Auth;
use crate::settings::Settings;
use api::{Api, Fail, MAX_UPDATE_IDS};
use form::Picture;
use keys::EditKeys;

/// The server takes pictures up to 5 MB.
const MAX_PICTURE_BYTES: u64 = 5 * 1024 * 1024;
const FIRST_CHECK: Duration = Duration::from_secs(20);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_REASON_CHARS: usize = 500;
const GONE: &str = "That theme is gone.";
const NO_KEY: &str = "This PC doesn’t have the key to change that theme.";

pub struct ThemeShare {
    base: String,
    keys: EditKeys,
}

pub fn init(app: &AppHandle) {
    let store: Arc<dyn SecretStore> = if crate::paths::webview(app).is_some() {
        Arc::new(MemoryStore::default())
    } else {
        Arc::new(KeyringStore)
    };
    app.manage(ThemeShare {
        base: base(app),
        keys: EditKeys::new(store),
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            check(&app).await;
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

/// `CYEX_API_URL` points a mocks build at `tools/mock/themes.mjs` (loopback and a test
/// data folder only; set but refused: nowhere, never the real server).
fn base(app: &AppHandle) -> String {
    #[cfg(feature = "mocks")]
    match crate::mock_url::gated(app, &["CYEX_API_URL"], "theme") {
        Ok(Some(b)) => return b,
        Ok(None) => {}
        Err(_) => return "http://127.0.0.1:0".into(),
    }
    let _ = app;
    api::BASE.into()
}

fn api(app: &AppHandle) -> Api {
    Api {
        http: app.state::<Auth>().http.clone(),
        base: app.state::<ThemeShare>().base.clone(),
    }
}

fn keys(app: &AppHandle) -> &EditKeys {
    &app.state::<ThemeShare>().inner().keys
}

fn theme(app: &AppHandle, id: &str) -> Result<CustomTheme, String> {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    a.themes.into_iter().find(|t| t.id == id).ok_or_else(|| GONE.into())
}

/// Changes one theme; `f` returns false to leave everything as it was.
fn change(app: &AppHandle, id: &str, f: impl FnOnce(&mut CustomTheme) -> bool) -> Result<(), String> {
    let mut found = false;
    app.state::<Settings>().update::<Appearance>(app, Partition::Appearance, |a| {
        let Some(t) = a.themes.iter_mut().find(|t| t.id == id) else {
            return false;
        };
        found = true;
        f(t)
    })?;
    if found { Ok(()) } else { Err(GONE.into()) }
}

/// The picture part of an upload: a file goes up the first time and when it changed.
fn picture(app: &AppHandle, t: &CustomTheme, first: bool) -> Result<Picture, String> {
    let w = &t.wallpaper;
    if w.source.is_empty() {
        return Ok(if first { Picture::Keep } else { Picture::Remove });
    }
    if !w.is_file() || (!first && w.source == t.link.shared_wallpaper) {
        return Ok(Picture::Keep);
    }
    let path = crate::theme::wallpaper_path(app, &w.source).ok_or("Couldn’t find the wallpaper file.")?;
    let size = fs::metadata(&path).map_err(|_| "Couldn’t read the wallpaper file.")?.len();
    if size > MAX_PICTURE_BYTES {
        return Err("The wallpaper is over 5 MB. Pick a smaller one to share.".into());
    }
    let bytes = fs::read(&path).map_err(|_| "Couldn’t read the wallpaper file.")?;
    let ext = w.source.rsplit('.').next().unwrap_or("jpg").to_owned();
    Ok(Picture::File(ext, bytes))
}

/// Uploads a theme for the first time; returns its link. Already shared: its link.
pub async fn share(app: &AppHandle, id: &str) -> Result<String, String> {
    let t = theme(app, id)?;
    if !t.link.shared_id.is_empty() {
        return Ok(theme_code::link_of(&t.link.shared_id));
    }
    let api = api(app);
    let created = api.create(&t, &picture(app, &t, true)?).await?;
    if let Err(e) = keys(app).put(&created.id, &created.edit_key) {
        eprintln!("theme share: couldn't keep the edit key: {e}");
        let _ = api.delete(&created.id, &created.edit_key).await;
        return Err("Couldn’t keep the edit key, so nothing was shared.".into());
    }
    let sid = created.id.clone();
    change(app, id, |x| {
        x.record_share(&sid, 1, &t);
        true
    })?;
    Ok(theme_code::link_of(&created.id))
}

/// "Update shared theme": the same link gets the theme as it is now. Returns the version.
pub async fn update_share(app: &AppHandle, id: &str) -> Result<u32, String> {
    let t = theme(app, id)?;
    let sid = t.link.shared_id.clone();
    let key = keys(app).get(&sid).ok_or(NO_KEY)?;
    let version = api(app).update(&sid, &key, &t, &picture(app, &t, false)?).await?;
    change(app, id, |x| {
        x.record_share(&sid, version, &t);
        true
    })?;
    Ok(version)
}

/// "Stop sharing": the link stops working; the theme stays here.
pub async fn stop_share(app: &AppHandle, id: &str) -> Result<(), String> {
    let t = theme(app, id)?;
    let sid = t.link.shared_id.clone();
    let key = keys(app).get(&sid).ok_or(NO_KEY)?;
    match api(app).delete(&sid, &key).await {
        // Gone already: nothing left to stop.
        Ok(()) | Err(Fail { status: 404 | 410, .. }) => {}
        Err(e) => return Err(e.message),
    }
    let _ = keys(app).forget(&sid);
    change(app, id, |x| {
        x.forget_share();
        true
    })
}

/// True when this PC can change the shared theme (it has the edit key).
pub fn can_edit(app: &AppHandle, id: &str) -> bool {
    theme(app, id).is_ok_and(|t| keys(app).get(&t.link.shared_id).is_some())
}

/// A theme someone pasted, shown before it's added.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThemePreview {
    pub theme: CustomTheme,
    /// The link's id; empty for a theme code.
    pub source_id: String,
    pub version: u32,
    /// `yyyy/mm/dd`; empty for a theme code.
    pub date: String,
}

pub async fn preview(app: &AppHandle, text: &str) -> Result<ThemePreview, String> {
    if let Some(id) = theme_code::parse_link(text) {
        let f = api(app).get(&id).await?;
        return Ok(ThemePreview {
            theme: f.theme,
            source_id: id,
            version: f.version,
            date: f.date,
        });
    }
    let theme = theme_code::from_code(text).map_err(|_| "That isn’t a theme link or code.".to_owned())?;
    Ok(ThemePreview {
        theme,
        source_id: String::new(),
        version: 0,
        date: String::new(),
    })
}

/// The theme to add from a preview: a fresh copy, tied to its link when it came from one.
pub fn from_preview(p: ThemePreview) -> CustomTheme {
    let mut t = crate::theme::fresh(p.theme.shareable());
    if valid_link_id(&p.source_id) {
        t.link_source(&p.source_id, p.version, &p.date);
    }
    t
}

pub async fn report(app: &AppHandle, source_id: &str, reason: &str) -> Result<(), String> {
    if !valid_link_id(source_id) {
        return Err(GONE.into());
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err("Say what’s wrong with it.".into());
    }
    let reason: String = reason.chars().take(MAX_REASON_CHARS).collect();
    api(app).report(source_id, &reason).await.map_err(String::from)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LinkUpdate {
    Done,
    /// You changed the theme: ask before replacing.
    AskReplace,
}

/// "Update": the newest version replaces yours; yours is kept for Go back.
pub async fn update_from_link(app: &AppHandle, id: &str, replace: bool) -> Result<LinkUpdate, String> {
    let t = theme(app, id)?;
    let sid = t.link.source_id.clone();
    if sid.is_empty() {
        return Err("That theme didn’t come from a link.".into());
    }
    if t.link.edited && !replace {
        return Ok(LinkUpdate::AskReplace);
    }
    match api(app).get(&sid).await {
        Ok(f) => {
            change(app, id, |x| {
                x.apply_fetched(&f);
                true
            })?;
            Ok(LinkUpdate::Done)
        }
        Err(e) => {
            if e.status == 410 {
                let _ = change(app, id, |x| x.note_upstream(cyex_settings::schema::Upstream::Removed));
            }
            Err(e.message)
        }
    }
}

pub fn restore(app: &AppHandle, id: &str, index: usize) -> Result<(), String> {
    let mut ok = false;
    change(app, id, |x| {
        ok = x.restore_copy(index);
        ok
    })?;
    if ok { Ok(()) } else { Err("That version is gone.".into()) }
}

pub fn ignore(app: &AppHandle, id: &str) -> Result<(), String> {
    change(app, id, CustomTheme::ignore_latest)
}

/// The link ids to ask about, once each.
fn source_ids(a: &Appearance) -> Vec<String> {
    let mut ids: Vec<String> = a.themes.iter().map(|t| t.link.source_id.clone()).filter(|s| !s.is_empty()).collect();
    ids.sort();
    ids.dedup();
    ids
}

/// One question per 50 themes added from links; nothing when there are none. Quiet when offline.
async fn check(app: &AppHandle) {
    let a: Appearance = app.state::<Settings>().read(Partition::Appearance);
    let ids = source_ids(&a);
    if ids.is_empty() {
        return;
    }
    let api = api(app);
    for chunk in ids.chunks(MAX_UPDATE_IDS) {
        let Ok(news) = api.updates(chunk).await else {
            continue;
        };
        let _ = app.state::<Settings>().update::<Appearance>(app, Partition::Appearance, |a| {
            let mut changed = false;
            for t in &mut a.themes {
                if let Some(up) = news.get(&t.link.source_id) {
                    changed |= t.note_upstream(*up);
                }
            }
            changed
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::{Store, SystemClock};
    use cyex_settings::schema::Fetched;

    const KEY: &str = "Zx9_secret-edit-key-0123456789abcdefghijklm";

    #[test]
    fn a_preview_becomes_a_linked_copy() {
        let mut theme = CustomTheme {
            name: "Neon".into(),
            ..CustomTheme::default()
        };
        theme.link.shared_id = "m4n5p6q7".into();
        let t = from_preview(ThemePreview {
            theme,
            source_id: "k3f9x2ab".into(),
            version: 3,
            date: "2026/11/04".into(),
        });
        assert_eq!(t.id.len(), 10);
        assert!(t.link.shared_id.is_empty(), "someone else's share isn't yours");
        assert_eq!((t.link.source_id.as_str(), t.link.source_version), ("k3f9x2ab", 3));
        let code = from_preview(ThemePreview {
            theme: CustomTheme::default(),
            source_id: String::new(),
            version: 0,
            date: String::new(),
        });
        assert!(code.link.is_empty(), "a code has no link");
    }

    #[test]
    fn ids_to_check_once_each() {
        let mut a = Appearance::default();
        for (i, sid) in ["k3f9x2ab", "", "k3f9x2ab", "m4n5p6q7"].iter().enumerate() {
            let mut t = CustomTheme {
                id: format!("t{i}"),
                ..CustomTheme::default()
            };
            t.link.source_id = (*sid).into();
            a.themes.push(t);
        }
        assert_eq!(source_ids(&a), ["k3f9x2ab", "m4n5p6q7"]);
        assert!(source_ids(&Appearance::default()).is_empty(), "nothing to ask: no call");
    }

    /// The edit key lives in the secret store only: not in the settings file, an export,
    /// a theme code or an upload.
    #[test]
    fn the_edit_key_never_leaves_the_vault() {
        let keys = EditKeys::new(Arc::new(MemoryStore::default()));
        let mut t = CustomTheme {
            id: "local1".into(),
            name: "Mine".into(),
            ..CustomTheme::default()
        };
        let sent = t.clone();
        t.record_share("k3f9x2ab", 1, &sent);
        keys.put("k3f9x2ab", KEY).unwrap();
        t.apply_fetched(&Fetched {
            version: 2,
            date: "2026/11/05".into(),
            theme: sent.content(),
        });

        let dir = std::env::temp_dir().join(format!("cyex-theme-share-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (mut store, _) = Store::open(&dir, Arc::new(SystemClock)).unwrap();
        let a = Appearance {
            theme_id: t.id.clone(),
            themes: vec![t.clone()],
            ..Appearance::default()
        };
        store.set(Partition::Appearance, serde_json::to_value(&a).unwrap()).unwrap();
        store.flush().unwrap();
        let export = store.export(&Partition::ALL);
        let file = fs::read_to_string(dir.join(Partition::Appearance.file_name())).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert!(export.to_json().contains("k3f9x2ab"), "the link id travels");
        for text in [export.to_json(), export.to_share_code(), file, theme_code::to_code(&t)] {
            assert!(!text.contains(KEY), "edit key leaked");
        }
        assert_eq!(keys.get("k3f9x2ab").as_deref(), Some(KEY));
    }
}
