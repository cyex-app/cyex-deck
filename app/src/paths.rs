//! V102-02 Where the app keeps its files: `%APPDATA%\cyex-deck` (settings, plugins,
//! recordings and the rest) and `%LOCALAPPDATA%\cyex-deck` (the Twitch window's profiles,
//! toast pictures). Both are copied once from the old name's folders.
//!
//! T01 (v1.04 batch 2b) Test runs: `CYEX_DATA_DIR=<full path outside AppData>` puts all of
//! it, and the app windows' WebView2 folder, under that folder instead (`roaming`, `local`,
//! `webview`). Nothing is copied from the old folders then.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::OnceLock;

use cyex_core::app::{OLD_SLUG, SLUG};
use cyex_settings::migrate::{self, Copied};
use tauri::{AppHandle, Manager};

/// T01 The test switch.
pub const DATA_DIR_VAR: &str = "CYEX_DATA_DIR";

/// The folders this run uses, picked once on start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// `%APPDATA%\cyex-deck`, or `<test>\roaming`.
    pub config: PathBuf,
    /// `%LOCALAPPDATA%\cyex-deck`, or `<test>\local`.
    pub local: PathBuf,
    /// A test run's WebView2 folder for the app's own windows (`<test>\webview`); `None`
    /// leaves it to Tauri (`%LOCALAPPDATA%\app.cyex.deck`).
    pub webview: Option<PathBuf>,
}

impl Dirs {
    pub fn is_test(&self) -> bool {
        self.webview.is_some()
    }
}

static DIRS: OnceLock<Dirs> = OnceLock::new();

/// T01 Picks the folders. `roaming` and `local` are `%APPDATA%` and `%LOCALAPPDATA%`. A
/// test folder must be a full path and must not sit inside either of them or their
/// `AppData` parent: Claude's packaged shells see a merged `AppData` that writes through
/// to the real files (DECISIONS, v1.04 batch 3).
pub fn choose(var: Option<&OsStr>, roaming: &Path, local: &Path) -> Result<Dirs, String> {
    let Some(var) = var.filter(|v| !v.is_empty()) else {
        return Ok(Dirs {
            config: roaming.join(SLUG),
            local: local.join(SLUG),
            webview: None,
        });
    };
    let dir = PathBuf::from(var);
    let plain = dir
        .components()
        .all(|c| !matches!(c, Component::ParentDir | Component::CurDir));
    if !dir.is_absolute() || !plain {
        return Err(format!(
            "{DATA_DIR_VAR} needs a full path without `.` or `..`: {}",
            dir.display()
        ));
    }
    let bases = [Some(roaming), Some(local), roaming.parent(), local.parent()];
    if bases.into_iter().flatten().any(|base| inside(&dir, base)) {
        return Err(format!(
            "{DATA_DIR_VAR} must be outside AppData: {}",
            dir.display()
        ));
    }
    Ok(Dirs {
        config: dir.join("roaming"),
        local: dir.join("local"),
        webview: Some(dir.join("webview")),
    })
}

/// `path` is `base` or below it, ignoring case and a `\\?\` prefix (Windows paths).
fn inside(path: &Path, base: &Path) -> bool {
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| match c {
                Component::Prefix(p) => match p.kind() {
                    Prefix::Disk(d) | Prefix::VerbatimDisk(d) => {
                        format!("{}:", d.to_ascii_lowercase() as char)
                    }
                    _ => p.as_os_str().to_string_lossy().to_lowercase(),
                },
                c => c.as_os_str().to_string_lossy().to_lowercase(),
            })
            .collect()
    };
    let (path, base) = (parts(path), parts(base));
    !base.is_empty() && path.len() >= base.len() && path[..base.len()] == base[..]
}

/// T01 Runs first on start: reads the switch once. An unusable test folder stops the app
/// rather than falling back to the real folders; so does a folder it can't make.
pub fn init(app: &AppHandle) -> &'static Dirs {
    let dirs = DIRS.get_or_init(|| pick(app).unwrap_or_else(|e| fail(None, &e)));
    if let Some(webview) = &dirs.webview {
        eprintln!(
            "paths: test data folder: settings {}, local {}, webview {}",
            dirs.config.display(),
            dirs.local.display(),
            webview.display()
        );
        for d in [&dirs.config, &dirs.local, webview] {
            if let Err(e) = std::fs::create_dir_all(d) {
                fail(Some(d), &why(d, &e));
            }
        }
    }
    dirs
}

/// The text of the start-up error box.
pub fn folder_message(path: Option<&Path>, reason: &str) -> String {
    let mut chars = reason.trim().chars();
    let reason: String = chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    match path {
        Some(p) => format!(
            "cyex deck can't open its settings folder:\n{}\n\n{reason}",
            p.display()
        ),
        None => format!("cyex deck can't open its settings folder.\n\n{reason}"),
    }
}

/// Why `path` can't be made or opened, in plain words.
pub fn why(path: &Path, err: &std::io::Error) -> String {
    if let Some(file) = path.ancestors().find(|p| p.is_file()) {
        return format!("A file is in the way: {}", file.display());
    }
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => "No permission to write there.".into(),
        _ => os_text(&err.to_string()),
    }
}

/// The OS text without its ` (os error 3)` tail.
fn os_text(text: &str) -> String {
    let text = match text.rfind(" (os error ") {
        Some(i) if text.ends_with(')') => &text[..i],
        _ => text,
    };
    text.trim().to_string()
}

/// V107-N9b Opens `dir` in Explorer, making it first. One text for every "Open folder".
pub fn open_folder(app: &AppHandle, dir: &Path) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    std::fs::create_dir_all(dir).map_err(|e| why(dir, &e))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| {
            eprintln!("paths: couldn't open {}: {e}", dir.display());
            "Couldn't open the folder.".to_owned()
        })
}

/// Shows the start-up error box, then quits with exit code 2.
pub fn fail(path: Option<&Path>, reason: &str) -> ! {
    let text = folder_message(path, reason);
    eprintln!("paths: {}", text.replace('\n', " "));
    crate::sys::fatal::show("cyex deck", &text);
    std::process::exit(2);
}

fn pick(app: &AppHandle) -> Result<Dirs, String> {
    let path = app.path();
    let roaming = path.config_dir().map_err(|e| e.to_string())?;
    let local = path.local_data_dir().map_err(|e| e.to_string())?;
    choose(std::env::var_os(DATA_DIR_VAR).as_deref(), &roaming, &local)
}

fn dirs(app: &AppHandle) -> tauri::Result<&'static Dirs> {
    if let Some(d) = DIRS.get() {
        return Ok(d);
    }
    let picked = pick(app).map_err(|_| tauri::Error::UnknownPath)?;
    Ok(DIRS.get_or_init(|| picked))
}

/// The app windows' WebView2 folder in a test run.
pub fn webview(app: &AppHandle) -> Option<PathBuf> {
    dirs(app).ok()?.webview.clone()
}

/// Browser caches in the Twitch window's profiles: they refill on their own, so they
/// aren't copied. Sign-ins and cookies are.
const CACHES: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnCache",
    "DawnGraphiteCache",
    "DawnWebGPUCache",
    "GraphiteDawnCache",
    "GrShaderCache",
    "ShaderCache",
    "Crashpad",
];

/// `%APPDATA%\cyex-deck` (T01: `<test>\roaming`).
pub fn config(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(dirs(app)?.config.clone())
}

/// `%LOCALAPPDATA%\cyex-deck` (T01: `<test>\local`).
pub fn local(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(dirs(app)?.local.clone())
}

/// Runs first on every start; does something only on the first start under the new name.
/// The main window's own browser storage (Tauri's folder named after the bundle id) isn't
/// copied: it's mostly cache and holds only the theme it starts with and the emote menu's
/// last page, which come back on their own. T01 Never in a test run.
pub fn migrate(app: &AppHandle) {
    if dirs(app).is_ok_and(Dirs::is_test) {
        return;
    }
    let path = app.path();
    for (base, skip) in [(path.config_dir(), &[][..]), (path.local_data_dir(), CACHES)] {
        let Ok(base) = base else { continue };
        match migrate::copy_once(&base.join(OLD_SLUG), &base.join(SLUG), skip) {
            Ok(Copied::Yes) => eprintln!("paths: copied {} from the old folder", base.join(SLUG).display()),
            Ok(_) => {}
            Err(e) => eprintln!(
                "paths: couldn't copy the old folder into {}: {e}",
                base.join(SLUG).display()
            ),
        }
    }
}

/// Runs on every start, after `migrate` (which may have just copied it): deletes
/// `%APPDATA%\cyex-deck\.env`, unused since the client ID is built in (V103-01). Only that
/// file; `%APPDATA%\cyex-viewer` is never touched.
pub fn remove_old_env(app: &AppHandle) {
    let Ok(dir) = config(app) else { return };
    match migrate::remove_old_env(&dir) {
        Ok(true) => eprintln!("paths: deleted the old .env"),
        Ok(false) => {}
        Err(e) => eprintln!("paths: couldn't delete the old .env: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bases() -> (PathBuf, PathBuf) {
        (
            PathBuf::from(r"C:\Users\me\AppData\Roaming"),
            PathBuf::from(r"C:\Users\me\AppData\Local"),
        )
    }

    #[test]
    fn unset_or_empty_keeps_the_normal_folders() {
        let (r, l) = bases();
        for var in [None, Some(OsStr::new(""))] {
            let d = choose(var, &r, &l).unwrap();
            assert_eq!(d.config, r.join("cyex-deck"));
            assert_eq!(d.local, l.join("cyex-deck"));
            assert!(!d.is_test());
        }
    }

    #[test]
    fn a_test_folder_takes_everything() {
        let (r, l) = bases();
        let d = choose(Some(OsStr::new(r"A:\repo\target\test-data")), &r, &l).unwrap();
        assert_eq!(d.config, PathBuf::from(r"A:\repo\target\test-data\roaming"));
        assert_eq!(d.local, PathBuf::from(r"A:\repo\target\test-data\local"));
        assert_eq!(
            d.webview,
            Some(PathBuf::from(r"A:\repo\target\test-data\webview"))
        );
        // Forward slashes are fine.
        assert!(
            choose(Some(OsStr::new("A:/repo/target/test-data")), &r, &l)
                .unwrap()
                .is_test()
        );
    }

    #[test]
    fn appdata_and_odd_paths_are_refused() {
        let (r, l) = bases();
        for bad in [
            r"C:\Users\me\AppData\Roaming\cyex-deck",
            r"c:\users\ME\appdata\local\Packages\x",
            r"C:\Users\me\AppData\LocalLow\x",
            r"C:\Users\me\AppData",
            r"\\?\C:\Users\me\AppData\Roaming\x",
            r"target\test-data",
            r"A:\repo\..\Users\me",
        ] {
            assert!(choose(Some(OsStr::new(bad)), &r, &l).is_err(), "{bad}");
        }
        // A sibling whose name only starts the same is fine.
        assert!(choose(Some(OsStr::new(r"C:\Users\me\AppDataTest")), &r, &l).is_ok());
    }

    #[test]
    fn the_error_box_reads_plainly() {
        let p = Path::new(r"A:\data\roaming");
        assert_eq!(
            folder_message(Some(p), r"A file is in the way: A:\data"),
            "cyex deck can't open its settings folder:\nA:\\data\\roaming\n\nA file is in the way: A:\\data"
        );
        assert_eq!(
            folder_message(None, " settings aren't valid"),
            "cyex deck can't open its settings folder.\n\nSettings aren't valid"
        );
    }

    #[test]
    fn os_error_numbers_are_dropped() {
        assert_eq!(
            os_text("The system cannot find the path specified. (os error 3)"),
            "The system cannot find the path specified."
        );
        assert_eq!(os_text("odd (text)"), "odd (text)");
    }

    #[test]
    fn a_file_in_the_way_is_named() {
        let file = std::env::temp_dir().join(format!("cyex-paths-test-{}", std::process::id()));
        std::fs::write(&file, b"").unwrap();
        let dir = file.join("roaming");
        let err = std::fs::create_dir_all(&dir).unwrap_err();
        let text = why(&dir, &err);
        let _ = std::fs::remove_file(&file);
        assert_eq!(text, format!("A file is in the way: {}", file.display()));
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(
            why(Path::new(r"Q:\nowhere\x"), &denied),
            "No permission to write there."
        );
    }
}
