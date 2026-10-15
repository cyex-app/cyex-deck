//! V105-16 Highlights from Chatterino: pick its settings.json, preview, import. Only the highlight lists are read; the rest of the file (account tokens) is never kept.

use std::sync::Mutex;

use cyex_settings::Partition;
use cyex_settings::chatterino::{
    Found, HighlightImport, HighlightPreview, apply_highlights, parse_highlights, plan_highlights,
};
use cyex_settings::schema::Highlights;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::settings::Settings;
use crate::tabs::import as chatterino;

type CmdResult<T> = Result<T, String>;

/// Settings files are small; a file this big isn't one.
const MAX_BYTES: u64 = 16 * 1024 * 1024;
const NOT_SETTINGS: &str = "That file isn’t Chatterino’s settings.json.";

/// What the last pick found, until the import or the next pick.
#[derive(Default)]
pub struct Picked(Mutex<Option<Found>>);

pub fn init(app: &AppHandle) {
    app.manage(Picked::default());
}

/// Reads only the highlights; the text is dropped before this returns.
fn read(path: &std::path::Path) -> CmdResult<Found> {
    let size = std::fs::metadata(path)
        .map_err(|_| "Couldn’t open that file.")?
        .len();
    if size > MAX_BYTES {
        return Err(NOT_SETTINGS.into());
    }
    let bytes = std::fs::read(path).map_err(|_| "Couldn’t open that file.")?;
    let text = String::from_utf8(bytes).map_err(|_| NOT_SETTINGS)?;
    let found = parse_highlights(&text, cyex_chat::rules::import_regex).map_err(|_| NOT_SETTINGS);
    drop(text);
    found.map_err(Into::into)
}

/// Picks Chatterino's settings.json and says what importing its highlights would do.
/// `None` = cancelled.
#[tauri::command]
pub async fn highlights_chatterino_preview(
    app: AppHandle,
    settings: State<'_, Settings>,
) -> CmdResult<Option<HighlightPreview>> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Import highlights from Chatterino")
        .add_filter("Chatterino settings", &["json"])
        .add_filter("All files", &["*"]);
    if let Some(dir) = chatterino::folder(&app) {
        dialog = dialog.set_directory(dir);
    }
    let Some(source) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let path = source.into_path().map_err(|e| e.to_string())?;
    let found = read(&path)?;
    let current: Highlights = settings.read(Partition::Highlights);
    let preview = plan_highlights(&found, &current);
    *lock(&app) = Some(found);
    Ok(Some(preview))
}

/// Adds the picked highlights; duplicates keep yours or take Chatterino's.
#[tauri::command]
pub fn highlights_chatterino_import(
    app: AppHandle,
    settings: State<Settings>,
    keep_mine: bool,
) -> CmdResult<HighlightImport> {
    let found = lock(&app).take().ok_or("Pick the file again.")?;
    let mut result = HighlightImport::default();
    settings.update::<Highlights>(&app, Partition::Highlights, |h| {
        result = apply_highlights(found, h, keep_mine);
        result.added + result.replaced > 0
    })?;
    Ok(result)
}

fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, Option<Found>> {
    app.state::<Picked>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r##"{
        "accounts": {"uid1": {"oauthToken": "FAKE-TOKEN-XYZ-123"}},
        "highlighting": {
            "highlights": [
                {"pattern": "hello there", "color": "#ff2288ee"},
                {"pattern": "(?<!@)cyexword", "regex": true, "alert": false, "showInMentions": false},
                {"pattern": "chime", "sound": true, "case": true, "soundUrl": "file:///C:/Windows/Media/chimes.wav"}
            ],
            "users": [{"pattern": "SomeUser"}],
            "badges": [{"name": "moderator,vip"}]
        }
    }"##;

    const BAD: &str = r##"{"highlighting": {"highlights": [
        {"pattern": "(abc", "regex": true},
        {"pattern": "\\X+", "regex": true},
        {"pattern": "\\Qa.b\\E\\h", "regex": true}
    ]}}"##;

    fn parse(text: &str) -> Found {
        parse_highlights(text, cyex_chat::rules::import_regex).expect("parses")
    }

    #[test]
    fn imports_with_the_real_converter_and_keeps_no_token() {
        let found = parse(GOOD);
        let mut h = Highlights::default();
        let preview = plan_highlights(&found, &h);
        assert_eq!((preview.phrases, preview.users, preview.badges), (3, 1, 2));
        let r = apply_highlights(found.clone(), &mut h, true);
        assert_eq!((r.added, r.kept, r.replaced), (6, 0, 0));
        assert_eq!(h.phrases[2].alert.sound_file, r"C:\Windows\Media\chimes.wav");
        assert!(h.phrases[2].case_sensitive && !h.phrases[1].alert.flash);
        assert!(!serde_json::to_string(&h).unwrap().contains("FAKE-TOKEN"));
        // Again: all six are there already.
        let again = plan_highlights(&found, &h);
        assert_eq!((again.phrases, again.duplicates.len()), (0, 6));
        h.phrases[0].alert.color = "#000000".into();
        let r = apply_highlights(found, &mut h, false);
        assert_eq!((r.added, r.replaced), (0, 6));
        assert_eq!(h.phrases[0].alert.color, "#2288eeff");
    }

    #[test]
    fn lists_what_cannot_be_imported() {
        let found = parse(BAD);
        assert_eq!(found.phrases.len(), 1);
        assert!(
            found.phrases[0].pattern.starts_with(r"a\.b[") && found.phrases[0].pattern.contains(r"\x{20}")
        );
        let reasons: Vec<_> = found
            .invalid
            .iter()
            .map(|s| (s.pattern.as_str(), s.reason.as_str()))
            .collect();
        assert_eq!(reasons[1], (r"\X+", r"Uses \X, which isn’t supported here."));
        assert_eq!(reasons[0].0, "(abc");
        assert!(parse_highlights("not json", cyex_chat::rules::import_regex).is_err());
    }
}
