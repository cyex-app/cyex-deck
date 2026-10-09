//! Owns the settings store: debounced saves, change events to every window (2.8, 2.9).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

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
    let dir = app.path().config_dir()?.join("cyex-viewer").join("settings");
    let (store, report) = Store::open(dir, Arc::new(SystemClock))?;
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
