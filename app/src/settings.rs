//! Owns the settings store: debounced saves, change events to every window (2.8, 2.9).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use cyex_auth::secrets::{KeyringStore, MemoryStore, SecretStore};
use cyex_settings::{BackupStatus, Bundle, Partition, Schema, Store, SystemClock};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use time::Date;

const SAVE_DELAY: Duration = Duration::from_millis(400);
/// V107-A3 Wait before trying a failed save again (a file held by a scanner, say).
const RETRY_DELAY: Duration = Duration::from_secs(5);
const CHANGED_EVENT: &str = "settings:changed";
/// V110-F82 Tells the UI that a partition keeps failing to save (payload: the partitions).
const SAVE_FAILING_EVENT: &str = "settings:save-failing";
/// V110-F82 Failed saves in a row (5 s apart) before the user hears of it.
const FAILS_BEFORE_NOTICE: u32 = 3;

pub struct Settings {
    store: Mutex<Store>,
    save_queued: AtomicBool,
    /// Serializes flushes; the disk work runs without the store lock.
    flushing: Mutex<()>,
    reset_on_load: Mutex<LoadNotice>,
    /// V110-F82 Appearance was reset, restored or not opened at this start: what no theme names
    /// may still be the user's wallpapers.
    appearance_untrusted: bool,
    /// V110-F82 Which partitions keep failing to save, and which the user was told about.
    save_watch: Mutex<SaveWatch>,
    /// V105-11 The uploader's headers live here, not in the settings files or their backups.
    secrets: Arc<dyn SecretStore>,
    /// V110-A-idea A file or share code waiting for Apply in the import review.
    pending_import: Mutex<Option<Bundle>>,
}

/// What the first window tells about loading: files that went back to default, and (V107-N10a)
/// files that came back from a daily backup, with that day as `yyyy/mm/dd`. Taken once.
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadNotice {
    reset: Vec<Partition>,
    restored: Vec<Partition>,
    restored_from: Option<String>,
    /// V110-F82 Files that couldn't be opened: left as they are, not saved this session.
    blocked: Vec<Partition>,
    /// V110-A44 How many tabs were kept from a tabs file that was cut short.
    rescued_tabs: Option<usize>,
}

/// V110-A43 What an import did: the partitions it replaced, and lines to tell the user (lists
/// cut at their cap, a wallpaper that didn't come along).
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub partitions: Vec<Partition>,
    pub notes: Vec<String>,
}

/// V110-F82 Counts failed saves per partition; tells once per partition per session when
/// a partition has failed [`FAILS_BEFORE_NOTICE`] times in a row.
#[derive(Default)]
struct SaveWatch {
    streak: std::collections::BTreeMap<Partition, u32>,
    told: std::collections::BTreeSet<Partition>,
}

impl SaveWatch {
    /// A flush ended; `still_dirty` are the partitions that didn't reach disk. Returns the
    /// partitions to tell the user about now.
    fn after_flush(&mut self, still_dirty: &[Partition]) -> Vec<Partition> {
        self.streak.retain(|p, _| still_dirty.contains(p));
        let mut now = Vec::new();
        for &p in still_dirty {
            let n = self.streak.entry(p).or_insert(0);
            *n += 1;
            if *n >= FAILS_BEFORE_NOTICE && self.told.insert(p) {
                now.push(p);
            }
        }
        now
    }
}

impl LoadNotice {
    fn of(report: &cyex_settings::LoadReport) -> Self {
        let day = time::macros::format_description!("[year]/[month]/[day]");
        Self {
            reset: report.reset.clone(),
            restored: report.restored.iter().map(|(p, _)| *p).collect(),
            restored_from: one_day(report.restored.iter().map(|(_, d)| *d)).and_then(|d| d.format(day).ok()),
            blocked: report.blocked.clone(),
            rescued_tabs: report.rescued.iter().find(|(p, _)| *p == Partition::Tabs).map(|(_, n)| *n),
        }
    }
}

/// V110-F82 `p` didn't load as the file had it (reset, brought back from a backup, or not opened).
fn untrusted(report: &cyex_settings::LoadReport, p: Partition) -> bool {
    report.reset.contains(&p) || report.restored.iter().any(|(r, _)| *r == p) || report.blocked.contains(&p)
}

/// The day all of them share, else `None` (the notice then just says "a backup").
fn one_day(mut days: impl Iterator<Item = Date>) -> Option<Date> {
    let first = days.next()?;
    days.all(|d| d == first).then_some(first)
}

/// Credential Manager key of the image uploader's headers.
const UPLOADER_HEADERS: &str = "uploader.headers";

fn blank_headers(v: &mut Value) {
    if v.get("headers").is_some_and(|h| h != "") {
        v["headers"] = Value::String(String::new());
    }
}

/// Moves the uploader's headers out of the partition into `secrets` and blanks them in the
/// store, so neither the files nor their backups hold them. Also migrates old plain values.
fn scrub_uploader(store: &mut Store, secrets: &dyn SecretStore) -> Result<bool, String> {
    let mut value = store.get(Partition::Uploader).clone();
    let plain = value.get("headers").and_then(Value::as_str).unwrap_or("").to_owned();
    if plain.is_empty() {
        return Ok(false);
    }
    value["headers"] = Value::String(String::new());
    store.set(Partition::Uploader, value).map_err(|e| e.to_string())?;
    secrets.save(UPLOADER_HEADERS, &plain)?;
    Ok(true)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Changed<'a> {
    partition: Partition,
    value: &'a Value,
}

pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let dir = crate::paths::config(app)?.join("settings");
    // V106-08 Chat saved before presence was turned on for everyone; sanitize turns it on.
    let presence_was = presence_pending(&dir.join(Partition::Chat.file_name()));
    // A settings folder it can't open ends the start with a message box, not a bare panic.
    let (mut store, report) = Store::open(&dir, Arc::new(SystemClock)).unwrap_or_else(|e| {
        let reason = match &e {
            cyex_settings::Error::Io(io) => crate::paths::why(&dir, io),
            e => e.to_string(),
        };
        crate::paths::fail(Some(&dir), &reason)
    });
    if let Some(r) = &report.merged {
        eprintln!(
            "settings: V106-33 mentions merged into highlights ({} from mentions, {} from highlights, {} global, {} look the same, {} over the limit); V106-09 your name plays a sound now",
            r.from_mentions, r.from_highlights, r.global, r.duplicates, r.cut
        );
    }
    for e in &report.io_errors {
        eprintln!("settings: V107-N10a {e}");
    }
    for p in &report.blocked {
        eprintln!("settings: V110-F82 {} couldn't be opened, left untouched and not saved this session", p.file_name());
    }
    if report.old_mentions_unreadable {
        eprintln!("settings: mentions.json unreadable, left as is");
    }
    if let Some(was_on) = presence_was {
        store.mark_dirty(Partition::Chat);
        // R21 This first save also writes the Highlights merge.
        if let Err(e) = store.flush() {
            eprintln!("settings: first save failed: {e}");
        }
        eprintln!(
            "settings: V106-08 presence to 7TV and BTTV turned on once for everyone (it was {})",
            if was_on { "on" } else { "off" }
        );
    }
    let secrets: Arc<dyn SecretStore> = if crate::paths::webview(app).is_some() {
        Arc::new(MemoryStore::default())
    } else {
        Arc::new(KeyringStore)
    };
    match scrub_uploader(&mut store, secrets.as_ref()) {
        Ok(true) => {
            let _ = store.flush();
            let _ = store.scrub_backups(Partition::Uploader, blank_headers);
        }
        Ok(false) => {}
        Err(e) => eprintln!("settings: couldn't move the uploader headers: {e}"),
    }
    app.manage(Settings {
        store: Mutex::new(store),
        save_queued: AtomicBool::new(false),
        flushing: Mutex::new(()),
        reset_on_load: Mutex::new(LoadNotice::of(&report)),
        save_watch: Mutex::default(),
        appearance_untrusted: untrusted(&report, Partition::Appearance),
        secrets,
        pending_import: Mutex::new(None),
    });
    Ok(())
}

/// V106-08 `Some(sharePresence)` when the saved chat file predates the one-time switch-on.
fn presence_pending(file: &Path) -> Option<bool> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    (v.get("presenceOnOnce").and_then(Value::as_bool) != Some(true))
        .then(|| v.get("sharePresence").and_then(Value::as_bool).unwrap_or(true))
}

pub fn flush_now(app: &AppHandle) {
    crate::chat::run::save_counts(app);
    if let Some(settings) = app.try_state::<Settings>() {
        settings.flush(app);
    }
}

impl Settings {
    fn lock(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn with<T>(&self, f: impl FnOnce(&Store) -> T) -> T {
        f(&self.lock())
    }

    pub fn read<T: Schema>(&self, p: Partition) -> T {
        self.lock().read(p)
    }

    /// Applies a merge patch and saves soon. `notify` broadcasts the new value to every
    /// window; window geometry updates skip it to avoid flooding the UI while dragging.
    pub fn patch(&self, app: &AppHandle, p: Partition, patch: &Value, notify: bool) -> Result<Value, String> {
        let value = {
            let mut store = self.lock();
            store.patch(p, patch).map_err(|e| e.to_string())?;
            if p == Partition::Uploader {
                scrub_uploader(&mut store, self.secrets.as_ref())?;
            }
            store.get(p).clone()
        };
        self.after_change(app, &[(p, &value)], notify);
        // A side effect may have saved this partition again (global mentions' name row).
        Ok(self.lock().get(p).clone())
    }

    /// Reads a partition typed, lets `f` change it and, if `f` returns true, saves it and
    /// tells every window.
    pub fn update<T: Schema>(
        &self,
        app: &AppHandle,
        p: Partition,
        f: impl FnOnce(&mut T) -> bool,
    ) -> Result<(), String> {
        let value = {
            let mut store = self.lock();
            let mut typed: T = store.read(p);
            if !f(&mut typed) {
                return Ok(());
            }
            let value = serde_json::to_value(typed).map_err(|e| e.to_string())?;
            store.set(p, value).map_err(|e| e.to_string())?.clone()
        };
        self.after_change(app, &[(p, &value)], true);
        Ok(())
    }

    pub fn reset(&self, app: &AppHandle, p: Partition) -> Value {
        let value = self.lock().reset(p).clone();
        if p == Partition::Uploader {
            let _ = self.secrets.delete(UPLOADER_HEADERS);
        }
        self.after_change(app, &[(p, &value)], true);
        value
    }

    pub fn import(&self, app: &AppHandle, bundle: Bundle) -> Result<Imported, String> {
        let (changed, notes) = self.lock().import_noted(bundle).map_err(|e| e.to_string())?;
        self.broadcast(app, &changed);
        Ok(Imported { partitions: changed, notes })
    }

    /// V110-A-idea Holds `bundle` for the review; what it brings. Replaces an earlier one.
    pub fn hold_import(&self, bundle: Bundle) -> cyex_settings::Review {
        let review = bundle.review();
        *self.pending_import.lock().unwrap_or_else(|p| p.into_inner()) = Some(bundle);
        review
    }

    /// V110-A-idea Applies the held file or code (Apply in the review).
    pub fn apply_held_import(&self, app: &AppHandle) -> Result<Imported, String> {
        let held = self.pending_import.lock().unwrap_or_else(|p| p.into_inner()).take();
        self.import(app, held.ok_or("There is nothing to import.")?)
    }

    /// V110-A-idea Cancel in the review.
    pub fn drop_held_import(&self) {
        *self.pending_import.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

    pub fn restore_backup(&self, app: &AppHandle, date: Date) -> Result<Vec<Partition>, String> {
        let changed = {
            let mut store = self.lock();
            let changed = store.restore_backup(date).map_err(|e| e.to_string())?;
            if changed.contains(&Partition::Uploader) && scrub_uploader(&mut store, self.secrets.as_ref())? {
                let _ = store.flush();
            }
            changed
        };
        self.broadcast(app, &changed);
        Ok(changed)
    }

    /// V110-A44 The files set aside as broken, newest first.
    pub fn broken_files(&self) -> Vec<cyex_settings::BrokenFile> {
        self.lock().broken_files()
    }

    /// V110-A44 Puts one of them back (the UI keeps a safety copy first, like any restore).
    pub fn restore_broken(&self, app: &AppHandle, name: &str) -> Result<Partition, String> {
        let p = self.lock().restore_broken(name).map_err(|e| e.to_string())?;
        self.broadcast(app, &[p]);
        Ok(p)
    }

    /// V1052-03 Saves the current settings on disk before a restore; returns the folder name.
    pub fn snapshot_now(&self) -> Result<Option<String>, String> {
        self.lock().snapshot_now().map_err(|e| e.to_string())
    }

    /// V110-U55 When the last daily copy was made and why the latest try failed.
    pub fn backup_status(&self) -> BackupStatus {
        self.lock().backup_status()
    }

    /// V110-U55 Tries today's copy now; the status after.
    pub fn backup_now(&self) -> Result<BackupStatus, String> {
        self.lock().backup_now().map_err(|e| e.to_string())
    }

    pub fn restore_snapshot(&self, app: &AppHandle, name: &str) -> Result<Vec<Partition>, String> {
        let changed = self.lock().restore_snapshot(name).map_err(|e| e.to_string())?;
        self.broadcast(app, &changed);
        Ok(changed)
    }

    /// The image uploader's `Name: value; …` headers.
    pub fn uploader_headers(&self) -> String {
        self.secrets.load(UPLOADER_HEADERS).ok().flatten().unwrap_or_default()
    }

    /// V110-F82 See `appearance_untrusted`.
    pub fn appearance_untrusted(&self) -> bool {
        self.appearance_untrusted
    }

    pub fn take_load_report(&self) -> LoadNotice {
        std::mem::take(&mut *self.reset_on_load.lock().unwrap_or_else(|p| p.into_inner()))
    }

    fn broadcast(&self, app: &AppHandle, partitions: &[Partition]) {
        let values: Vec<(Partition, Value)> = {
            let store = self.lock();
            partitions.iter().map(|p| (*p, store.get(*p).clone())).collect()
        };
        let refs: Vec<(Partition, &Value)> = values.iter().map(|(p, v)| (*p, v)).collect();
        self.after_change(app, &refs, true);
    }

    fn after_change(&self, app: &AppHandle, changed: &[(Partition, &Value)], notify: bool) {
        if changed.iter().any(|(p, _)| *p == Partition::Tabs) {
            crate::tabs::on_changed(app);
            crate::chat::shown::tabs_changed(app);
        }
        if changed
            .iter()
            .any(|(p, _)| matches!(p, Partition::Chat | Partition::Emotes))
        {
            crate::emotes::on_settings(app);
        }
        if changed
            .iter()
            .any(|(p, _)| matches!(p, Partition::Chat | Partition::Highlights))
        {
            crate::chat::on_settings(app);
        }
        // V107-Mq Accounts too: global mentions look for your names.
        if changed
            .iter()
            .any(|(p, _)| matches!(p, Partition::Highlights | Partition::Accounts))
        {
            crate::chat::global::sync(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::Miner) {
            crate::miner::on_settings(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::Commands) {
            crate::chat::run::rebuild(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::General) {
            crate::streamer::on_settings(app);
            crate::sound::on_settings(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::Hotkeys) {
            crate::hotkeys::apply(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::Appearance) {
            crate::theme::on_settings(app);
            crate::windows::zoom::apply_all(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::Plugins) {
            crate::plugins::on_settings(app);
        }
        if notify {
            // The store's value now, not the one passed in: a side effect above may have
            // saved the partition again, and its newer value must not be sent over.
            for (partition, _) in changed {
                let value = self.lock().get(*partition).clone();
                let _ = app.emit(
                    CHANGED_EVENT,
                    Changed {
                        partition: *partition,
                        value: &value,
                    },
                );
            }
        }
        self.schedule_save(app);
    }

    fn schedule_save(&self, app: &AppHandle) {
        self.schedule_save_in(app, SAVE_DELAY);
    }

    fn schedule_save_in(&self, app: &AppHandle, delay: Duration) {
        if self.save_queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(delay);
            let settings = app.state::<Settings>();
            settings.save_queued.store(false, Ordering::Release);
            // V107-A3 Parts that didn't reach disk are tried again, at most every 5 s.
            if settings.flush(&app) {
                settings.schedule_save_in(&app, RETRY_DELAY);
            }
        });
    }

    /// Writes what changed. True when it failed and something is still unsaved.
    fn flush(&self, app: &AppHandle) -> bool {
        // The disk work runs outside the store lock, so reads and patches don't wait on it.
        // One flush at a time, so two never interleave writes.
        let _flushing = self.flushing.lock().unwrap_or_else(|p| p.into_inner());
        let Some(job) = self.lock().prepare_flush() else { return false };
        let done = job.run();
        let mut store = self.lock();
        let result = store.commit_flush(done);
        // V110-F82 Retried silently every 5 s forever; the user hears of it once. Only a failed
        // save counts: after a good one, what is dirty again is just a newer edit.
        let failed = if result.is_err() { store.dirty_partitions() } else { Vec::new() };
        let tell = self.save_watch.lock().unwrap_or_else(|p| p.into_inner()).after_flush(&failed);
        if !tell.is_empty() {
            let _ = app.emit_to("main", SAVE_FAILING_EVENT, tell);
        }
        match result {
            Ok(()) => false,
            Err(e) => {
                eprintln!("settings: save failed: {e}");
                store.is_dirty()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn files_hold(dir: &std::path::Path, needle: &str) -> bool {
        std::fs::read_dir(dir).unwrap().flatten().any(|e| {
            let p = e.path();
            if p.is_dir() {
                files_hold(&p, needle)
            } else {
                std::fs::read_to_string(&p).is_ok_and(|t| t.contains(needle))
            }
        })
    }

    /// V110-F82 The user is told once per partition, after three failed saves in a row.
    #[test]
    fn save_watch_tells_once_after_three_failures() {
        let mut w = SaveWatch::default();
        let both = [Partition::Chat, Partition::Tabs];
        assert!(w.after_flush(&both).is_empty());
        assert!(w.after_flush(&both).is_empty());
        assert_eq!(w.after_flush(&both), both);
        assert!(w.after_flush(&both).is_empty(), "once");
        // One recovers, its streak starts over; the other stays told.
        assert!(w.after_flush(&[Partition::Tabs]).is_empty());
        assert!(w.after_flush(&both).is_empty());
        assert!(w.after_flush(&[]).is_empty());
    }

    #[test]
    fn v110_f82_a_partition_that_did_not_load_whole_is_untrusted() {
        let mut r = cyex_settings::LoadReport::default();
        assert!(!untrusted(&r, Partition::Appearance));
        r.reset.push(Partition::Appearance);
        assert!(untrusted(&r, Partition::Appearance));
        assert!(!untrusted(&r, Partition::Chat));
        let mut r = cyex_settings::LoadReport::default();
        r.blocked.push(Partition::Appearance);
        assert!(untrusted(&r, Partition::Appearance));
        let mut r = cyex_settings::LoadReport::default();
        r.restored.push((Partition::Appearance, time::Date::MIN));
        assert!(untrusted(&r, Partition::Appearance));
    }

    #[test]
    fn plain_headers_move_to_the_secret_store() {
        let dir = std::env::temp_dir().join(format!("cyex-uploader-secret-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (mut store, _) = Store::open(&dir, Arc::new(SystemClock)).unwrap();
        store
            .patch(Partition::Uploader, &json!({ "url": "https://x/up", "headers": "Authorization: s3cr3t-key" }))
            .unwrap();
        store.flush().unwrap();
        assert!(files_hold(&dir, "s3cr3t-key"), "the old plain value is on disk");
        let secrets = MemoryStore::default();
        assert!(scrub_uploader(&mut store, &secrets).unwrap());
        store.flush().unwrap();
        store.scrub_backups(Partition::Uploader, blank_headers).unwrap();
        assert!(!files_hold(&dir, "s3cr3t-key"), "files and backups are clean");
        assert_eq!(secrets.load(UPLOADER_HEADERS).unwrap().as_deref(), Some("Authorization: s3cr3t-key"));
        assert_eq!(store.get(Partition::Uploader)["headers"], "");
        assert_eq!(store.get(Partition::Uploader)["url"], "https://x/up");
        // Exports and the exported bundle never carry them either.
        assert!(!store.export(&Partition::ALL).to_json().contains("s3cr3t-key"));
        // A second pass changes nothing; a later flush (and its backup) stays clean.
        assert!(!scrub_uploader(&mut store, &secrets).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn presence_pending_reads_old_chat_files() {
        let dir = std::env::temp_dir().join(format!("cyex-presence-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("chat.json");
        assert_eq!(presence_pending(&file), None, "no file");
        std::fs::write(&file, r#"{"sharePresence":false}"#).unwrap();
        assert_eq!(presence_pending(&file), Some(false));
        std::fs::write(&file, r#"{"presenceOnOnce":true,"sharePresence":false}"#).unwrap();
        assert_eq!(presence_pending(&file), None);
        std::fs::write(&file, "not json").unwrap();
        assert_eq!(presence_pending(&file), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
