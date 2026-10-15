//! Owns the settings store: debounced saves, change events to every window (2.8, 2.9).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

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
    app.manage(Settings {
        store: Mutex::new(store),
        save_queued: AtomicBool::new(false),
        reset_on_load: Mutex::new(report.reset),
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
            store.patch(p, patch).map_err(|e| e.to_string())?.clone()
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
        self.after_change(app, &[(p, &value)], true);
        value
    }

    pub fn import(&self, app: &AppHandle, bundle: Bundle) -> Result<Vec<Partition>, String> {
        let changed = self.lock().import(bundle).map_err(|e| e.to_string())?;
        self.broadcast(app, &changed);
        Ok(changed)
    }

    pub fn restore_backup(&self, app: &AppHandle, date: Date) -> Result<Vec<Partition>, String> {
        let changed = self.lock().restore_backup(date).map_err(|e| e.to_string())?;
        self.broadcast(app, &changed);
        Ok(changed)
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
