//! S5 What a plugin's thread keeps next to its Lua: callbacks by kind, timers, requests
//! in flight, settings values, and the time limit of the call that's running.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mlua::{FromLuaMulti, Function, IntoLuaMulti, Lua, RegistryKey};
use serde_json::{Map, Value};

use crate::host::{Host, Level};
use crate::perm::Granted;
use crate::runtime::{Event, Exports, Msg};
use crate::sandbox::Deadline;

/// Messages a plugin may send in [`SEND_WINDOW`].
pub const SEND_LIMIT: usize = 10;
pub const SEND_WINDOW: Duration = Duration::from_secs(30);
pub const MAX_TIMERS: usize = 100;
pub const MAX_REQUESTS: usize = 10;

pub struct State {
    pub id: String,
    pub dir: PathBuf,
    pub granted: Granted,
    pub host: Arc<dyn Host>,
    pub exports: Arc<Mutex<Exports>>,
    pub tx: std::sync::mpsc::Sender<Msg>,
    pub commands: HashMap<String, RegistryKey>,
    pub callbacks: HashMap<Event, RegistryKey>,
    pub menu: Vec<RegistryKey>,
    pub timers: Vec<(Instant, u64, RegistryKey)>,
    pub requests: HashMap<u64, Requested>,
    pub settings: Map<String, Value>,
    pub next_id: u64,
    pub sends: VecDeque<Instant>,
    /// A line callback is running: chat state is locked by whoever waits on it.
    pub in_line: bool,
    /// Done loading: later changes are announced.
    pub ready: bool,
}

/// Callbacks of a request that's on its way.
#[derive(Default)]
pub struct Requested {
    pub success: Option<RegistryKey>,
    pub error: Option<RegistryKey>,
    pub finally: Option<RegistryKey>,
}

impl State {
    pub fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// The exports changed; tells the app once the plugin is up.
    pub fn publish(&self, f: impl FnOnce(&mut Exports)) {
        f(&mut self.exports.lock().unwrap_or_else(|p| p.into_inner()));
        if self.ready {
            self.host.exports_changed(&self.id);
        }
    }

    pub fn log(&self, level: Level, text: &str) {
        self.host.log(&self.id, level, text);
    }

    /// Room for one more message in the plugin's send limit.
    pub fn take_send(&mut self) -> bool {
        let now = Instant::now();
        while self
            .sends
            .front()
            .is_some_and(|t| now.duration_since(*t) > SEND_WINDOW)
        {
            self.sends.pop_front();
        }
        if self.sends.len() >= SEND_LIMIT {
            return false;
        }
        self.sends.push_back(now);
        true
    }
}

/// Runs `f` with the plugin's state borrowed. Errors instead of panicking when it's
/// already in use (the release build aborts on panics).
pub fn with<R>(lua: &Lua, f: impl FnOnce(&mut State) -> R) -> mlua::Result<R> {
    let mut s = lua
        .try_app_data_mut::<State>()
        .map_err(|_| mlua::Error::runtime("busy"))?
        .ok_or_else(|| mlua::Error::runtime("plugin state is gone"))?;
    Ok(f(&mut s))
}

/// A Lua callback stored in the registry.
pub fn function(lua: &Lua, key: &RegistryKey) -> Option<Function> {
    lua.registry_value::<Function>(key).ok()
}

/// Calls into the plugin with a time limit. Errors come back as short text.
pub fn call<R: FromLuaMulti>(
    deadline: &Deadline,
    budget: Duration,
    f: &Function,
    args: impl IntoLuaMulti,
) -> Result<R, String> {
    deadline.start(budget);
    let r = f.call::<R>(args);
    deadline.clear();
    r.map_err(|e| describe(&e))
}

/// The useful part of a Lua error: no "runtime error:" prefix, no traceback.
pub fn describe(e: &mlua::Error) -> String {
    match e {
        mlua::Error::CallbackError { cause, .. } => describe(cause),
        mlua::Error::WithContext { cause, .. } => describe(cause),
        mlua::Error::RuntimeError(m) | mlua::Error::SyntaxError { message: m, .. } => {
            m.lines().next().unwrap_or(m).trim().to_owned()
        }
        mlua::Error::MemoryError(_) => "ran out of memory".into(),
        other => {
            let s = other.to_string();
            s.lines().next().unwrap_or(&s).trim().to_owned()
        }
    }
}

pub fn is_timeout(e: &str) -> bool {
    e.ends_with("took too long")
}
