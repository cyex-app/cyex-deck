//! V108-N02 Saving a clip as an MP4: the file the player plays, picked in a save dialog,
//! streamed to disk with progress, and stoppable. The user's own action; any clip that plays
//! in the app can be saved. The link is fetched again here, so it is always a fresh one.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use cyex_twitch::{DownloadError, gql};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use ts_rs::TS;

use crate::auth::Auth;

/// The event the UI listens to.
const EVENT: &str = "player:clip-save";
/// Progress is told at most this often.
const EVERY: Duration = Duration::from_millis(150);

/// How far a save has come.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipSaveProgress {
    pub slug: String,
    #[ts(type = "number")]
    pub done: u64,
    /// `null` when the server doesn't say how big it is.
    #[ts(type = "number | null")]
    pub total: Option<u64>,
}

/// Saves running now, by clip slug; setting the flag stops one.
fn running() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static RUNNING: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    RUNNING.get_or_init(Default::default)
}

/// One save per clip at a time; dropping it frees the clip again.
struct Slot {
    slug: String,
    stop: Arc<AtomicBool>,
}

impl Slot {
    fn take(slug: &str) -> Option<Slot> {
        take_in(running(), slug)
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        if let Ok(mut m) = running().lock() {
            m.remove(&self.slug);
        }
    }
}

fn take_in(map: &Mutex<HashMap<String, Arc<AtomicBool>>>, slug: &str) -> Option<Slot> {
    let mut m = map.lock().ok()?;
    if m.contains_key(slug) {
        return None;
    }
    let stop = Arc::new(AtomicBool::new(false));
    m.insert(slug.to_owned(), stop.clone());
    Some(Slot {
        slug: slug.to_owned(),
        stop,
    })
}

/// Stops the save of `slug`, if one runs. True when it did.
pub fn cancel(slug: &str) -> bool {
    running()
        .lock()
        .ok()
        .and_then(|m| m.get(slug).cloned())
        .map(|f| f.store(true, Ordering::Relaxed))
        .is_some()
}

/// Whether enough time passed since the last progress told: the first and the last always do.
fn due(last: Option<Instant>, now: Instant, finished: bool) -> bool {
    finished || last.is_none_or(|t| now.duration_since(t) >= EVERY)
}

/// Asks where to save, then saves the clip's best file (or the one of `height`). `None`:
/// the user cancelled, in the dialog or while it saved. Otherwise the path it was saved to.
pub async fn save(app: &AppHandle, slug: &str, height: Option<u32>) -> Result<Option<String>, String> {
    let slot = Slot::take(slug).ok_or("Already saving this clip.")?;
    let http = app.state::<Auth>().http.clone();
    let playback = gql::clip(&http, slug)
        .await
        .map_err(|e| {
            if e.is_network() {
                "Couldn’t reach Twitch.".to_owned()
            } else {
                e.to_string()
            }
        })?
        .ok_or("This clip is gone.")?;
    let (url, name) = playback
        .download(height)
        .ok_or("This clip has no file to save.")?;

    let asked = app.clone();
    let picked = tokio::task::spawn_blocking(move || {
        asked
            .dialog()
            .file()
            .set_title("Save clip")
            .set_file_name(name)
            .add_filter("MP4 video", &["mp4"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(target) = picked else { return Ok(None) };
    let path: PathBuf = target.into_path().map_err(|e| e.to_string())?;

    let mut last: Option<Instant> = None;
    let told = app.clone();
    let id = slug.to_owned();
    let result = http
        .download_to(&url, &path, &slot.stop, |done, total| {
            let now = Instant::now();
            if due(last, now, total == Some(done)) {
                last = Some(now);
                let _ = told.emit(
                    EVENT,
                    ClipSaveProgress {
                        slug: id.clone(),
                        done,
                        total,
                    },
                );
            }
        })
        .await;
    match result {
        Ok(_) => Ok(Some(path.to_string_lossy().into_owned())),
        Err(DownloadError::Cancelled) => Ok(None),
        Err(e) => Err(saving_error(&e)),
    }
}

fn saving_error(e: &DownloadError) -> String {
    match e {
        DownloadError::Network(_) => "Couldn’t reach Twitch.".into(),
        DownloadError::Status(_) => "Twitch no longer has this file. Open the clip again.".into(),
        DownloadError::Stalled => "Twitch stopped sending. Try again.".into(),
        DownloadError::Disk(_) => "Couldn’t write the file. Is the disk full?".into(),
        DownloadError::Replace(_) => "Couldn’t replace the file. Is it open somewhere?".into(),
        DownloadError::Cancelled => "Stopped.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_save_per_clip_until_it_ends() {
        let map = Mutex::new(HashMap::new());
        let first = take_in(&map, "Funny-AbC").unwrap();
        assert!(take_in(&map, "Funny-AbC").is_none());
        assert!(take_in(&map, "Other-XyZ").is_some());
        assert!(!first.stop.load(Ordering::Relaxed));
    }

    #[test]
    fn stopping_sets_the_flag_of_that_clip_only() {
        let a = Slot::take("v108n02-a").unwrap();
        let b = Slot::take("v108n02-b").unwrap();
        assert!(cancel("v108n02-a"));
        assert!(a.stop.load(Ordering::Relaxed));
        assert!(!b.stop.load(Ordering::Relaxed));
        assert!(!cancel("v108n02-none"));
        drop(a);
        assert!(Slot::take("v108n02-a").is_some());
    }

    #[test]
    fn progress_is_told_a_few_times_a_second() {
        let t0 = Instant::now();
        assert!(due(None, t0, false));
        assert!(!due(Some(t0), t0 + Duration::from_millis(50), false));
        assert!(due(Some(t0), t0 + Duration::from_millis(200), false));
        assert!(due(Some(t0), t0 + Duration::from_millis(1), true));
    }

    #[test]
    fn errors_read_short() {
        assert_eq!(
            saving_error(&DownloadError::Status(403)),
            "Twitch no longer has this file. Open the clip again."
        );
        assert!(saving_error(&DownloadError::Disk(std::io::Error::other("x"))).contains("disk"));
        assert!(saving_error(&DownloadError::Replace(std::io::Error::other("x"))).contains("open somewhere"));
        assert!(saving_error(&DownloadError::Stalled).contains("stopped"));
    }
}
