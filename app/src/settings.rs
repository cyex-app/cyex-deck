//! Owns the settings store: debounced saves, change events to every window (2.8, 2.9).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use cyex_auth::secrets::{KeyringStore, MemoryStore, SecretStore};
use cyex_settings::schema::{Highlights, Mentions};
use cyex_settings::{Bundle, Partition, Schema, Store, SystemClock};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use time::Date;

const SAVE_DELAY: Duration = Duration::from_millis(400);
const CHANGED_EVENT: &str = "settings:changed";

pub struct Settings {
    store: Mutex<Store>,
    save_queued: AtomicBool,
    reset_on_load: Mutex<Vec<Partition>>,
    /// V105-11 The uploader's headers live here, not in the settings files or their backups.
    secrets: Arc<dyn SecretStore>,
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
    // V104-01 Mentions got their own file; the first start with it takes your name's
    // options from Highlights.
    let first_mentions = !dir.join(Partition::Mentions.file_name()).exists();
    let (mut store, report) = Store::open(dir, Arc::new(SystemClock))?;
    if first_mentions {
        let h: Highlights = store.read(Partition::Highlights);
        let _ = store.set(Partition::Mentions, serde_json::to_value(Mentions::from_highlights(&h))?);
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
        reset_on_load: Mutex::new(report.reset),
        secrets,
    });
    Ok(())
}

pub fn flush_now(app: &AppHandle) {
    if let Some(settings) = app.try_state::<Settings>() {
        settings.flush();
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
        Ok(value)
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

    pub fn import(&self, app: &AppHandle, bundle: Bundle) -> Result<Vec<Partition>, String> {
        let changed = self.lock().import(bundle).map_err(|e| e.to_string())?;
        self.broadcast(app, &changed);
        Ok(changed)
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

    /// V1052-03 Saves the current settings on disk before a restore; returns the folder name.
    pub fn snapshot_now(&self) -> Result<Option<String>, String> {
        self.lock().snapshot_now().map_err(|e| e.to_string())
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

    pub fn take_load_report(&self) -> Vec<Partition> {
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
        }
        if changed.iter().any(|(p, _)| *p == Partition::Chat) {
            crate::emotes::on_settings(app);
        }
        if changed
            .iter()
            .any(|(p, _)| matches!(p, Partition::Chat | Partition::Highlights | Partition::Mentions))
        {
            crate::chat::on_settings(app);
        }
        if changed.iter().any(|(p, _)| *p == Partition::Mentions) {
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
        }
        if changed.iter().any(|(p, _)| *p == Partition::Plugins) {
            crate::plugins::on_settings(app);
        }
        if notify {
            for (partition, value) in changed {
                let _ = app.emit(
                    CHANGED_EVENT,
                    Changed {
                        partition: *partition,
                        value,
                    },
                );
            }
        }
        self.schedule_save(app);
    }

    fn schedule_save(&self, app: &AppHandle) {
        if self.save_queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(SAVE_DELAY);
            let settings = app.state::<Settings>();
            settings.save_queued.store(false, Ordering::Release);
            settings.flush();
        });
    }

    fn flush(&self) {
        if let Err(e) = self.lock().flush() {
            eprintln!("settings: save failed: {e}");
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
}
