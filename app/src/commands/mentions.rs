//! V104-01, V104-02 The Mentions page: pattern checks, the firehose's status, and a
//! mention's own sound file.

use cyex_chat::mentions::{self, MentionCheck};
use cyex_settings::Partition;
use cyex_settings::schema::{Highlights, MentionRule, Mentions, SOUND_TYPES, is_sound_file};
use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::chat::global::{self, GlobalStatus};
use crate::settings::Settings;

type CmdResult<T> = Result<T, String>;

/// Sound files larger than this aren't played.
const MAX_SOUND: u64 = 5 * 1024 * 1024;

/// What to show under each pattern: why it doesn't work, or how it works differently.
#[tauri::command]
pub fn mentions_check(list: Vec<MentionRule>) -> Vec<MentionCheck> {
    list.iter().map(mentions::check).collect()
}

#[tauri::command]
pub fn mentions_global_status(app: AppHandle) -> GlobalStatus {
    global::status(&app)
}

/// Picks a sound file. Nothing when cancelled.
#[tauri::command]
pub async fn mentions_pick_sound(app: AppHandle) -> CmdResult<Option<String>> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_title("Pick a sound")
        .add_filter("Sounds", &SOUND_TYPES)
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

/// A mention's or highlight's sound file, for the main window to play. Only files your settings name.
#[tauri::command]
pub fn mentions_sound(settings: State<Settings>, path: String) -> CmdResult<Response> {
    let m: Mentions = settings.read(Partition::Mentions);
    let h: Highlights = settings.read(Partition::Highlights);
    let named = std::iter::once(&m.me.alert)
        .chain(m.list.iter().map(|r| &r.alert))
        .chain(m.global_list.iter().map(|r| &r.alert))
        .any(|a| a.sound_file == path)
        || h.phrases
            .iter()
            .map(|r| &r.alert)
            .chain(h.users.iter().map(|r| &r.alert))
            .chain(h.badges.iter().map(|r| &r.alert))
            .chain(h.apps.iter().map(|r| &r.alert))
            .chain([&h.first_message, &h.redeemed, &h.subs, &h.announcements, &h.watch_streaks, &h.own].map(|b| &b.alert))
            .any(|a| a.sound_file == path);
    if !named || !is_sound_file(&path) {
        return Err("Not a sound from your settings.".into());
    }
    let size = std::fs::metadata(&path)
        .map_err(|_| "The sound file is gone.".to_owned())?
        .len();
    if size > MAX_SOUND {
        return Err("The sound file is over 5 MB.".into());
    }
    let bytes = std::fs::read(&path).map_err(|_| "The sound file can't be read.".to_owned())?;
    Ok(Response::new(bytes))
}
