//! V111-60 Settings › Import: finds Chatterino's folder (or a portable one picked by hand),
//! reads the files each kind needs (read only, nothing is written there) and hands each kind
//! to its own import, so the old entry points and this page share one path. Ignores, never
//! highlight, muted chats, filters and nicknames are imported here. Accounts never are.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::chatterino::extras::{self, Extras, ExtrasPreview};
use cyex_settings::chatterino::{CommandPreview, HighlightPreview, ImportReport};
use cyex_settings::schema::{Chat, Highlights};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use crate::settings::Settings;
use crate::tabs::import as tabs;

use super::CmdResult;

const LAYOUT: &str = "window-layout.json";
const SETTINGS: &str = "settings.json";
const COMMANDS: &str = "commands.json";
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const NOT_SETTINGS: &str = "That file isn’t Chatterino’s settings.json.";
const NO_LOOKUP: &str = "couldn’t look the names up. Sign in, then try again.";

/// A Chatterino settings folder and which of its files are there.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatterinoFolder {
    /// The folder holding the files (`…\Chatterino2\Settings`).
    pub path: String,
    pub layout: bool,
    pub settings: bool,
    pub commands: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImportKind {
    Tabs,
    Highlights,
    Commands,
    Extras,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KindError {
    pub kind: ImportKind,
    pub text: String,
}

/// Each asked-for kind's preview, or why it can't come.
#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatterinoScan {
    #[ts(optional = nullable)]
    pub tabs: Option<ImportReport>,
    #[ts(optional = nullable)]
    pub highlights: Option<HighlightPreview>,
    #[ts(optional = nullable)]
    pub commands: Option<CommandPreview>,
    #[ts(optional = nullable)]
    pub extras: Option<ExtrasPreview>,
    pub errors: Vec<KindError>,
}

/// What the last scan found for the extras, until their import or the next scan.
static PICKED: Mutex<Option<Extras>> = Mutex::new(None);

fn picked() -> std::sync::MutexGuard<'static, Option<Extras>> {
    PICKED.lock().unwrap_or_else(|p| p.into_inner())
}

fn describe(dir: PathBuf) -> ChatterinoFolder {
    ChatterinoFolder {
        layout: dir.join(LAYOUT).is_file(),
        settings: dir.join(SETTINGS).is_file(),
        commands: dir.join(COMMANDS).is_file(),
        path: dir.to_string_lossy().into_owned(),
    }
}

/// The folder with the files: `dir\Settings` (an install or a portable folder) or `dir`
/// itself when it holds them.
fn settings_dir(dir: &Path) -> Option<PathBuf> {
    let has = |d: &Path| [LAYOUT, SETTINGS, COMMANDS].iter().any(|f| d.join(f).is_file());
    let inner = dir.join("Settings");
    if has(&inner) {
        Some(inner)
    } else if has(dir) {
        Some(dir.to_path_buf())
    } else {
        None
    }
}

/// Where Chatterino keeps its settings on this PC. Test builds: `CYEX_CHATTERINO_DIR`.
fn default_dir(app: &AppHandle) -> Option<PathBuf> {
    #[cfg(feature = "mocks")]
    if let Ok(dir) = std::env::var("CYEX_CHATTERINO_DIR") {
        return Some(PathBuf::from(dir));
    }
    Some(app.path().data_dir().ok()?.join("Chatterino2"))
}

/// Chatterino's folder if it's on this PC, with its files.
#[tauri::command]
pub fn chatterino_folder(app: AppHandle) -> Option<ChatterinoFolder> {
    default_dir(&app).and_then(|d| settings_dir(&d)).map(describe)
}

/// A portable Chatterino: pick its folder. `None` = cancelled.
#[tauri::command]
pub async fn chatterino_pick_folder(app: AppHandle) -> CmdResult<Option<ChatterinoFolder>> {
    // V110-F85 The picker blocks: off the runtime's workers.
    let asked = app.clone();
    let chosen = tokio::task::spawn_blocking(move || {
        asked
            .dialog()
            .file()
            .set_title("Pick Chatterino’s folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(dir) = chosen else { return Ok(None) };
    let dir = dir.into_path().map_err(|e| e.to_string())?;
    settings_dir(&dir)
        .map(|d| Some(describe(d)))
        .ok_or_else(|| "No Chatterino settings in that folder.".into())
}

/// Reads what each kind in `kinds` needs from `path` (a folder from the two calls above)
/// and previews it. Each kind's import call then adds it.
#[tauri::command]
pub async fn chatterino_scan(
    app: AppHandle,
    settings: State<'_, Settings>,
    path: String,
    kinds: Vec<ImportKind>,
) -> CmdResult<ChatterinoScan> {
    let dir = settings_dir(Path::new(&path)).ok_or("Chatterino’s folder is gone. Pick it again.")?;
    let mut scan = ChatterinoScan::default();
    let mut errors = Vec::new();
    for kind in kinds {
        let file = dir.join(match kind {
            ImportKind::Tabs => LAYOUT,
            ImportKind::Highlights | ImportKind::Extras => SETTINGS,
            ImportKind::Commands => COMMANDS,
        });
        if !file.is_file() {
            let name = file.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            errors.push(scan_error(kind, format!("No {name} in that folder.")));
            continue;
        }
        let result = match kind {
            ImportKind::Tabs => tab_preview(&app, &file).await.map(|r| scan.tabs = Some(r)),
            ImportKind::Highlights => highlight_preview(&app, &settings, &file).map(|r| scan.highlights = Some(r)),
            ImportKind::Commands => {
                super::chatterino_commands::preview_at(&settings, &file).map(|r| scan.commands = Some(r))
            }
            ImportKind::Extras => extras_preview(&app, &settings, &file).await.map(|r| scan.extras = Some(r)),
        };
        if let Err(e) = result {
            errors.push(scan_error(kind, e));
        }
    }
    scan.errors = errors;
    Ok(scan)
}

fn scan_error(kind: ImportKind, text: String) -> KindError {
    KindError { kind, text }
}

async fn tab_preview(app: &AppHandle, file: &Path) -> Result<ImportReport, String> {
    // C1 Test builds without an account look the channels up on public GQL.
    #[cfg(feature = "mocks")]
    if tabs::account(app).is_err() {
        return tabs::preview_public(app, file).await;
    }
    tabs::preview(app, file).await
}

/// A settings.json without highlights has none to bring, not a broken file.
fn highlight_preview(app: &AppHandle, settings: &Settings, file: &Path) -> Result<HighlightPreview, String> {
    match super::highlights::preview_at(app, settings, file) {
        Ok(p) => Ok(p),
        Err(_) if read_text(file).is_ok_and(|t| t.trim_start().starts_with('{')) => Ok(HighlightPreview::default()),
        Err(e) => Err(e),
    }
}

fn read_text(path: &Path) -> Result<String, String> {
    let size = std::fs::metadata(path).map_err(|_| "Couldn’t open that file.")?.len();
    if size > MAX_BYTES {
        return Err(NOT_SETTINGS.into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Couldn’t open that file.")?;
    String::from_utf8(bytes).map_err(|_| NOT_SETTINGS.into())
}

/// Reads the extras (tokens in the file are never read), looks the nicknames up, previews.
async fn extras_preview(app: &AppHandle, settings: &Settings, file: &Path) -> Result<ExtrasPreview, String> {
    let text = read_text(file)?;
    let found = extras::parse(&text, cyex_chat::rules::import_regex, |e| {
        cyex_chat::filter::Filter::parse(e).map(|_| ())
    });
    drop(text);
    let mut found = found.map_err(|_| NOT_SETTINGS)?;
    let logins = found.nickname_logins();
    if !logins.is_empty() {
        match tabs::look_up(app, &logins).await {
            Ok(users) => {
                let ids = users.into_iter().map(|(login, c)| (login, c.id)).collect();
                found.resolve_nicknames(&ids);
            }
            Err(_) => found.drop_nicknames(NO_LOOKUP),
        }
    }
    let h: Highlights = settings.read(Partition::Highlights);
    let chat: Chat = settings.read(Partition::Chat);
    let preview = extras::plan(&found, &h, &chat.users);
    *picked() = Some(found);
    Ok(preview)
}

/// Adds the scanned extras: ignores, never highlight, muted chats, filters, nicknames.
/// Rows you have stay.
#[tauri::command]
pub fn chatterino_extras_import(app: AppHandle, settings: State<Settings>) -> CmdResult<ExtrasPreview> {
    let found = picked().take().ok_or("Read the folder again.")?;
    let mut out = extras::start(&found);
    settings.update::<Highlights>(&app, Partition::Highlights, |h| {
        extras::apply_lists(&found, h, &mut out);
        out.ignores + out.never + out.muted + out.filters > 0
    })?;
    settings.update::<Chat>(&app, Partition::Chat, |c| {
        extras::apply_nicknames(&found, &mut c.users, &mut out);
        out.nicknames > 0
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cyex-v111-60-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn finds_the_settings_folder_in_an_install_or_portable_folder() {
        let root = tmp("find");
        assert_eq!(settings_dir(&root), None);
        std::fs::create_dir_all(root.join("Settings")).unwrap();
        assert_eq!(settings_dir(&root), None, "an empty Settings folder isn't one");
        std::fs::write(root.join("Settings").join(COMMANDS), "{}").unwrap();
        assert_eq!(settings_dir(&root), Some(root.join("Settings")));
        // The Settings folder itself, picked by hand.
        assert_eq!(settings_dir(&root.join("Settings")), Some(root.join("Settings")));
        let f = describe(root.join("Settings"));
        assert!(f.commands && !f.layout && !f.settings);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_real_filter_and_regex_readers_take_the_fixture() {
        let text = include_str!("../../../crates/settings/tests/fixtures/chatterino/settings-extras.json");
        let found = extras::parse(text, cyex_chat::rules::import_regex, |e| {
            cyex_chat::filter::Filter::parse(e).map(|_| ())
        })
        .unwrap();
        assert_eq!(found.ignores.len(), 3);
        assert_eq!(found.filters.len(), 2, "{:?}", found.skipped);
        assert!(found.skipped.iter().any(|s| s.pattern == "Broken"));
        assert!(found.skipped.iter().any(|s| s.pattern == "(broken"));
    }
}
