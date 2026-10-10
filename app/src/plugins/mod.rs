//! S5 Lua plugins in the app. Each is a folder in `%APPDATA%\cyex-viewer\plugins`. One
//! runs when plugins are on, the plugin is on and the user agreed to what its info file
//! asks for. `--safe-mode` lists them without running any. The Lua side is
//! `cyex_plugins`; here are the folder, settings, chat hooks and the console's log.

pub mod calls;
mod hook;
mod host;
pub mod logs;

pub use hook::to_line as hook_line;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use cyex_chat::LineHook;
use cyex_plugins::{Config, Event, Exports, Granted, Level, Manifest, Plugin};
use cyex_settings::Partition;
use cyex_settings::schema::{PluginEntry, Plugins};
use serde::Serialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::settings::Settings;

pub const CHANGED_EVENT: &str = "plugins:changed";
pub const SAFE_MODE_ARG: &str = "--safe-mode";

pub struct PluginState {
    dir: PathBuf,
    safe_mode: bool,
    found: RwLock<Vec<Found>>,
    /// One sync at a time.
    syncing: Mutex<()>,
    /// Some running plugin has a Message callback.
    wants_lines: AtomicBool,
    pub logs: logs::Logs,
    http: cyex_plugins::fetch::Client,
}

struct Found {
    id: String,
    dir: PathBuf,
    manifest: Result<Manifest, String>,
    running: Option<Arc<Plugin>>,
    /// Why it didn't start; it waits for a reload or to be turned on again.
    error: Option<String>,
    /// Settings values it was last given.
    values: Map<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PluginStatus {
    Off,
    Running,
    /// It failed to start; `error` says why.
    Failed,
    /// On, but its info file asks for permissions the user hasn't agreed to.
    NeedsOk,
    /// The folder isn't a plugin; `error` says why.
    Invalid,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginInfo {
    /// The folder's name.
    pub id: String,
    pub info: Option<Manifest>,
    pub status: PluginStatus,
    pub error: Option<String>,
    pub exports: Exports,
    /// The values its settings show, defaults filled in.
    #[ts(type = "Record<string, unknown>")]
    pub values: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginsView {
    pub folder: String,
    pub safe_mode: bool,
    pub list: Vec<PluginInfo>,
}

pub fn init(app: &AppHandle) {
    let dir = app
        .path()
        .config_dir()
        .map(|d| d.join("cyex-viewer").join("plugins"))
        .unwrap_or_else(|_| PathBuf::from("plugins"));
    let _ = std::fs::create_dir_all(&dir);
    let safe_mode = std::env::args().any(|a| a == SAFE_MODE_ARG);
    let found = scan(&dir);
    app.manage(PluginState {
        dir,
        safe_mode,
        found: RwLock::new(found),
        syncing: Mutex::new(()),
        wants_lines: AtomicBool::new(false),
        logs: logs::Logs::default(),
        http: cyex_plugins::fetch::client(),
    });
    sync_later(app);
}

/// After the plugins settings changed.
pub fn on_settings(app: &AppHandle) {
    sync_later(app);
}

fn sync_later(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || sync(&app));
}

/// The hook every chat room gets; it costs nothing while no plugin reads chat.
pub fn line_hook(app: &AppHandle) -> Option<Arc<dyn LineHook>> {
    app.try_state::<PluginState>()?;
    Some(Arc::new(hook::PluginHook { app: app.clone() }))
}

/// Running plugins, by folder name.
pub fn running(app: &AppHandle) -> Vec<Arc<Plugin>> {
    let Some(state) = app.try_state::<PluginState>() else {
        return Vec::new();
    };
    let found = state.found.read().unwrap_or_else(|p| p.into_inner());
    found.iter().filter_map(|f| f.running.clone()).collect()
}

pub fn get(app: &AppHandle, id: &str) -> Option<Arc<Plugin>> {
    let state = app.try_state::<PluginState>()?;
    let found = state.found.read().unwrap_or_else(|p| p.into_inner());
    found.iter().find(|f| f.id == id).and_then(|f| f.running.clone())
}

/// The plugin's name from its info file, else its folder.
pub fn name_of(app: &AppHandle, id: &str) -> String {
    app.try_state::<PluginState>()
        .and_then(|s| {
            let found = s.found.read().unwrap_or_else(|p| p.into_inner());
            found
                .iter()
                .find(|f| f.id == id)
                .and_then(|f| f.manifest.as_ref().ok().map(|m| m.name.clone()))
        })
        .unwrap_or_else(|| id.to_owned())
}

fn wants_lines(app: &AppHandle) -> bool {
    app.try_state::<PluginState>()
        .is_some_and(|s| s.wants_lines.load(Ordering::Relaxed))
}

/// After a plugin registered or dropped something.
pub fn exports_changed(app: &AppHandle) {
    if let Some(state) = app.try_state::<PluginState>() {
        let any = running(app).iter().any(|p| p.has(Event::Message));
        state.wants_lines.store(any, Ordering::Relaxed);
    }
    let _ = app.emit(CHANGED_EVENT, ());
}

/// Each folder in the plugins folder, by name.
fn scan(dir: &std::path::Path) -> Vec<Found> {
    let mut out: Vec<Found> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .filter_map(|e| Some((e.file_name().into_string().ok()?, e.path())))
                .map(|(id, dir)| Found {
                    manifest: if valid_id(&id) {
                        Manifest::load(&dir)
                    } else {
                        Err("Folder names take letters, digits, ., _ and -.".into())
                    },
                    id,
                    dir,
                    running: None,
                    error: None,
                    values: Map::new(),
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// A plugin's settings values: what's stored, made to fit its fields.
fn values_for(m: &Manifest, entry: Option<&PluginEntry>) -> Map<String, Value> {
    m.settings
        .iter()
        .map(|f| (f.key.clone(), f.value(entry.and_then(|e| e.values.get(&f.key)))))
        .collect()
}

/// Starts and stops plugins to match the settings. Starting waits on `init.lua`, so it
/// runs off the async threads and outside the list's lock.
fn sync(app: &AppHandle) {
    let Some(state) = app.try_state::<PluginState>() else {
        return;
    };
    let _one = state.syncing.lock().unwrap_or_else(|p| p.into_inner());
    let cfg: Plugins = app.state::<Settings>().read(Partition::Plugins);
    let mut to_start = Vec::new();
    let mut stopped = Vec::new();
    {
        let mut found = state.found.write().unwrap_or_else(|p| p.into_inner());
        for f in found.iter_mut() {
            let entry = cfg.list.get(&f.id);
            let Ok(m) = &f.manifest else { continue };
            let want = !state.safe_mode && cfg.enabled && entry.is_some_and(|e| e.on);
            let granted = entry.and_then(|e| Granted::new(m, &e.granted));
            let values = values_for(m, entry);
            if !want {
                f.error = None;
            }
            match (&f.running, granted.filter(|_| want)) {
                (Some(_), None) => {
                    f.running = None;
                    stopped.push(f.id.clone());
                }
                (Some(p), Some(g)) if p.granted != g => {
                    f.running = None;
                    to_start.push((f.id.clone(), f.dir.clone(), g, values));
                }
                (Some(p), Some(_)) => {
                    if f.values != values {
                        p.set_settings(values.clone());
                        f.values = values;
                    }
                }
                (None, Some(g)) if f.error.is_none() => {
                    to_start.push((f.id.clone(), f.dir.clone(), g, values))
                }
                _ => {}
            }
        }
    }
    for id in stopped {
        logs::push(app, &id, Level::Info, "Stopped.");
    }
    for (id, dir, granted, values) in to_start {
        let started = Plugin::start(Config {
            id: id.clone(),
            dir,
            granted,
            settings: values.clone(),
            host: Arc::new(host::AppHost { app: app.clone() }),
        });
        match &started {
            Ok(_) => logs::push(app, &id, Level::Info, "Started."),
            Err(e) => logs::push(app, &id, Level::Critical, &format!("Didn't start: {e}")),
        }
        let mut found = state.found.write().unwrap_or_else(|p| p.into_inner());
        if let Some(f) = found.iter_mut().find(|f| f.id == id) {
            match started {
                Ok(p) => {
                    f.running = Some(Arc::new(p));
                    f.values = values;
                    f.error = None;
                }
                Err(e) => f.error = Some(e),
            }
        }
    }
    exports_changed(app);
}

/// What Settings → Plugins shows.
pub fn view(app: &AppHandle) -> PluginsView {
    let state = app.state::<PluginState>();
    let cfg: Plugins = app.state::<Settings>().read(Partition::Plugins);
    let found = state.found.read().unwrap_or_else(|p| p.into_inner());
    let list = found
        .iter()
        .map(|f| {
            let entry = cfg.list.get(&f.id);
            let (info, status, error, values) = match &f.manifest {
                Err(e) => (None, PluginStatus::Invalid, Some(e.clone()), Map::new()),
                Ok(m) => {
                    let status = if f.running.is_some() {
                        PluginStatus::Running
                    } else if f.error.is_some() {
                        PluginStatus::Failed
                    } else if entry.is_some_and(|e| e.on)
                        && entry.and_then(|e| Granted::new(m, &e.granted)).is_none()
                    {
                        PluginStatus::NeedsOk
                    } else {
                        PluginStatus::Off
                    };
                    (Some(m.clone()), status, f.error.clone(), values_for(m, entry))
                }
            };
            PluginInfo {
                id: f.id.clone(),
                info,
                status,
                error,
                exports: f.running.as_ref().map(|p| p.exports()).unwrap_or_default(),
                values,
            }
        })
        .collect();
    PluginsView {
        folder: state.dir.to_string_lossy().into_owned(),
        safe_mode: state.safe_mode,
        list,
    }
}

/// Turns a plugin on, agreeing to what its info file asks for now, or off.
pub fn set_on(app: &AppHandle, id: &str, on: bool) -> Result<(), String> {
    let keys = {
        let state = app.state::<PluginState>();
        let found = state.found.read().unwrap_or_else(|p| p.into_inner());
        let f = found.iter().find(|f| f.id == id).ok_or("That plugin is gone.")?;
        f.manifest.as_ref().map_err(Clone::clone)?.keys()
    };
    app.state::<Settings>()
        .update::<Plugins>(app, Partition::Plugins, |p| {
            let e = p.list.entry(id.to_owned()).or_default();
            e.on = on;
            if on {
                e.granted = keys;
            }
            true
        })
}

/// One of a plugin's own settings.
pub fn set_value(app: &AppHandle, id: &str, key: &str, value: Value) -> Result<(), String> {
    let value = {
        let state = app.state::<PluginState>();
        let found = state.found.read().unwrap_or_else(|p| p.into_inner());
        let f = found.iter().find(|f| f.id == id).ok_or("That plugin is gone.")?;
        let m = f.manifest.as_ref().map_err(Clone::clone)?;
        let field = m
            .settings
            .iter()
            .find(|f| f.key == key)
            .ok_or("No such setting.")?;
        field.value(Some(&value))
    };
    app.state::<Settings>()
        .update::<Plugins>(app, Partition::Plugins, |p| {
            p.list
                .entry(id.to_owned())
                .or_default()
                .values
                .insert(key.to_owned(), value);
            true
        })
}

/// Reads a plugin's folder again and starts it fresh. `None`: every folder.
pub fn reload(app: &AppHandle, id: Option<&str>) {
    let state = app.state::<PluginState>();
    let fresh = scan(&state.dir);
    {
        let mut found = state.found.write().unwrap_or_else(|p| p.into_inner());
        let mut old = std::mem::take(&mut *found);
        *found = fresh
            .into_iter()
            .map(|mut f| {
                let kept = old.iter().position(|o| o.id == f.id).map(|i| old.swap_remove(i));
                if let Some(o) = kept
                    && id.is_some_and(|id| id != f.id)
                {
                    // Others keep running as they are.
                    f.manifest = o.manifest;
                    f.running = o.running;
                    f.error = o.error;
                    f.values = o.values;
                }
                f
            })
            .collect();
    }
    if let Some(id) = id {
        logs::push(app, id, Level::Info, "Reloading.");
    }
    sync_later(app);
    let _ = app.emit(CHANGED_EVENT, ());
}

pub fn folder(app: &AppHandle, id: Option<&str>) -> PathBuf {
    let dir = app.state::<PluginState>().dir.clone();
    match id {
        Some(id) if valid_id(id) => dir.join(id),
        _ => dir,
    }
}
