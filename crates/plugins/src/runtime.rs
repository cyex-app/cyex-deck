//! S5 One thread per plugin. Its Lua never leaves that thread; the app talks to it through
//! messages and gets answers through callbacks, so a chat can wait on a line callback with
//! a deadline and walk on when the plugin is slow.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mlua::chunk::ChunkMode;
use mlua::{Lua, Value};
use serde::Serialize;
use serde_json::{Map, Value as Json};
use ts_rs::TS;

use crate::api;
use crate::convert;
use crate::host::{
    Change, ChatEvent, Completion, CompletionRequest, Host, HttpResponse, Level, Line, NameUser, Sent, SocketEvent,
};
use crate::manifest::{DATA, ENTRY};
use crate::perm::Granted;
use crate::repl;
use crate::sandbox::{self, Deadline};
use crate::state::{self, State, call, describe, function, is_timeout};

/// Time limits per kind of call.
pub const INIT: Duration = Duration::from_secs(2);
pub const LINE: Duration = Duration::from_millis(10);
pub const QUICK: Duration = Duration::from_millis(100);
pub const CALLBACK: Duration = Duration::from_millis(500);
pub const REPL: Duration = Duration::from_secs(2);
/// Lines waiting for a plugin that reads chat; more are skipped.
const MAX_QUEUED_LINES: usize = 500;
/// S8 Chat events waiting for a plugin; more are skipped.
const MAX_QUEUED_EVENTS: usize = 1000;
/// Too-slow line callbacks in a row before they're paused.
const STRIKES: u32 = 3;

pub type Reply<T> = Box<dyn FnOnce(T) + Send>;

/// Chatterino's `c2.EventType`, plus line hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, TS)]
#[ts(export, rename = "PluginEvent")]
pub enum Event {
    CompletionRequested,
    /// A line arrived; the callback may change or hide it (ChangeChat).
    Message,
    /// The user sends a line; the callback may change or stop it.
    Sending,
    SettingsChanged,
    /// S8 A line is in (ReadChat).
    MessageAppended,
    /// S8 A line was replaced (ReadChat).
    MessageReplaced,
    /// S8 Lines were deleted (ReadChat).
    MessageDeleted,
    /// S8 A mod cleared a chat (ReadChat).
    ChatCleared,
    /// S8 A tab's channel was renamed on Twitch.
    ChannelRenamed,
    /// S12 A tab's channel went live or offline.
    LiveChanged,
    /// S12 The tabs changed.
    LayoutChanged,
}

impl Event {
    pub const ALL: [Event; 11] = [
        Event::CompletionRequested,
        Event::Message,
        Event::Sending,
        Event::SettingsChanged,
        Event::MessageAppended,
        Event::MessageReplaced,
        Event::MessageDeleted,
        Event::ChatCleared,
        Event::ChannelRenamed,
        Event::LiveChanged,
        Event::LayoutChanged,
    ];

    /// S8 The events rooms report after the fact.
    pub const CHAT: [Event; 5] = [
        Event::MessageAppended,
        Event::MessageReplaced,
        Event::MessageDeleted,
        Event::ChatCleared,
        Event::ChannelRenamed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Event::CompletionRequested => "CompletionRequested",
            Event::Message => "Message",
            Event::Sending => "Sending",
            Event::SettingsChanged => "SettingsChanged",
            Event::MessageAppended => "MessageAppended",
            Event::MessageReplaced => "MessageReplaced",
            Event::MessageDeleted => "MessageDeleted",
            Event::ChatCleared => "ChatCleared",
            Event::ChannelRenamed => "ChannelRenamed",
            Event::LiveChanged => "LiveChanged",
            Event::LayoutChanged => "LayoutChanged",
        }
    }
}

/// What a plugin registered, for the app to read without asking the thread.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "PluginExports")]
pub struct Exports {
    /// Without the slash.
    pub commands: Vec<String>,
    pub events: Vec<Event>,
    /// Menu entries, in order (S9: on messages or the chat's ⋯ menu, maybe in a group;
    /// S11 on names).
    pub menu: Vec<MenuExport>,
    /// The line callback was too slow too often; a reload brings it back.
    pub paused: bool,
}

/// S9 Where a menu entry shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "PluginMenuPlace")]
pub enum MenuPlace {
    /// A line's right-click menu.
    #[default]
    Message,
    /// The chat's ⋯ menu.
    Chat,
    /// S11 A name's right-click menu, in chat and the viewer list.
    Name,
}

/// S9 A menu entry a plugin registered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "PluginMenuExport")]
pub struct MenuExport {
    pub label: String,
    pub place: MenuPlace,
    /// Entries with the same group share a submenu.
    pub group: Option<String>,
    /// Message entries: the kinds of line that show it (`chat`, `action`, `notice`,
    /// `plugin`); empty for all.
    pub lines: Vec<String>,
}

pub enum Msg {
    Line {
        line: Line,
        reply: Option<Reply<Option<Change>>>,
    },
    Sending {
        channel: String,
        text: String,
        reply: Reply<Sent>,
    },
    Complete {
        req: CompletionRequest,
        reply: Reply<Completion>,
    },
    Command {
        name: String,
        words: Vec<String>,
        channel: String,
        reply: Reply<Result<Option<String>, String>>,
    },
    Menu {
        index: usize,
        channel: String,
        line: Option<Line>,
        /// S11 For name entries.
        user: Option<NameUser>,
        reply: Reply<Result<(), String>>,
    },
    Repl {
        code: String,
        reply: Reply<Result<String, String>>,
    },
    Settings(Map<String, Json>),
    Http {
        id: u64,
        res: HttpResponse,
    },
    /// S6
    Socket {
        id: u64,
        event: SocketEvent,
    },
    /// S8
    Chat(Vec<ChatEvent>),
    Stop,
}

pub struct Config {
    pub id: String,
    pub dir: PathBuf,
    pub granted: Granted,
    pub settings: Map<String, Json>,
    pub host: Arc<dyn Host>,
}

/// A running plugin. Dropping it stops the thread.
pub struct Plugin {
    pub id: String,
    pub granted: Granted,
    tx: Sender<Msg>,
    exports: Arc<Mutex<Exports>>,
    queued_lines: Arc<AtomicUsize>,
    /// S8 Chat events on their way, and whether some were skipped since it caught up.
    queued_events: Arc<AtomicUsize>,
    skipped: Arc<AtomicUsize>,
}

impl Plugin {
    /// Loads `init.lua` on a new thread and waits for it, up to [`INIT`] and a bit.
    pub fn start(cfg: Config) -> Result<Plugin, String> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let exports = Arc::new(Mutex::new(Exports::default()));
        let queued_lines = Arc::new(AtomicUsize::new(0));
        let queued_events = Arc::new(AtomicUsize::new(0));
        let skipped = Arc::new(AtomicUsize::new(0));
        let plugin = Plugin {
            id: cfg.id.clone(),
            granted: cfg.granted.clone(),
            tx: tx.clone(),
            exports: exports.clone(),
            queued_lines: queued_lines.clone(),
            queued_events: queued_events.clone(),
            skipped: skipped.clone(),
        };
        let name = format!("plugin {}", cfg.id);
        std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                let Some((lua, deadline)) = boot(cfg, tx, exports, &ready_tx) else {
                    return;
                };
                serve(&lua, &deadline, &rx, &queued_lines, (&queued_events, &skipped));
            })
            .map_err(|e| e.to_string())?;
        match ready_rx.recv_timeout(INIT + Duration::from_secs(1)) {
            Ok(Ok(())) => Ok(plugin),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("Took too long to start.".into()),
        }
    }

    pub fn exports(&self) -> Exports {
        self.exports.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn has(&self, e: Event) -> bool {
        let x = self.exports.lock().unwrap_or_else(|p| p.into_inner());
        x.events.contains(&e) && !(e == Event::Message && x.paused)
    }

    /// A line for the Message callback. Without `reply` it's only read. Skipped when the
    /// plugin is far behind. `false`: not taken.
    pub fn line(&self, line: Line, reply: Option<Reply<Option<Change>>>) -> bool {
        if self.queued_lines.load(Ordering::Relaxed) >= MAX_QUEUED_LINES {
            return false;
        }
        self.queued_lines.fetch_add(1, Ordering::Relaxed);
        self.send(Msg::Line { line, reply })
    }

    /// Lines are still waiting for it: a chat shouldn't wait on it now.
    pub fn behind(&self) -> bool {
        self.queued_lines.load(Ordering::Relaxed) > 0
    }

    /// S8 Chat events, skipped (and counted) while the plugin is far behind.
    pub fn chat(&self, events: Vec<ChatEvent>) -> bool {
        let n = events.len();
        if self.queued_events.load(Ordering::Relaxed) + n > MAX_QUEUED_EVENTS {
            self.skipped.fetch_add(n, Ordering::Relaxed);
            return false;
        }
        self.queued_events.fetch_add(n, Ordering::Relaxed);
        self.send(Msg::Chat(events))
    }

    pub fn sending(&self, channel: &str, text: &str, reply: Reply<Sent>) -> bool {
        self.send(Msg::Sending {
            channel: channel.to_owned(),
            text: text.to_owned(),
            reply,
        })
    }

    pub fn complete(&self, req: CompletionRequest, reply: Reply<Completion>) -> bool {
        self.send(Msg::Complete { req, reply })
    }

    pub fn command(
        &self,
        name: &str,
        words: Vec<String>,
        channel: &str,
        reply: Reply<Result<Option<String>, String>>,
    ) -> bool {
        self.send(Msg::Command {
            name: name.to_owned(),
            words,
            channel: channel.to_owned(),
            reply,
        })
    }

    pub fn menu(
        &self,
        index: usize,
        channel: &str,
        (line, user): (Option<Line>, Option<NameUser>),
        reply: Reply<Result<(), String>>,
    ) -> bool {
        self.send(Msg::Menu {
            index,
            channel: channel.to_owned(),
            line,
            user,
            reply,
        })
    }

    pub fn repl(&self, code: &str, reply: Reply<Result<String, String>>) -> bool {
        self.send(Msg::Repl {
            code: code.to_owned(),
            reply,
        })
    }

    pub fn set_settings(&self, values: Map<String, Json>) {
        self.send(Msg::Settings(values));
    }

    fn send(&self, msg: Msg) -> bool {
        self.tx.send(msg).is_ok()
    }
}

impl Drop for Plugin {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
    }
}

fn boot(
    cfg: Config,
    tx: Sender<Msg>,
    exports: Arc<Mutex<Exports>>,
    ready: &mpsc::SyncSender<Result<(), String>>,
) -> Option<(Lua, Deadline)> {
    let fail = |e: String| {
        let _ = ready.send(Err(e));
        None
    };
    let (lua, deadline) = match sandbox::new(&cfg.dir) {
        Ok(l) => l,
        Err(e) => return fail(describe(&e)),
    };
    if cfg.granted.fs_read || cfg.granted.fs_write {
        let _ = std::fs::create_dir_all(cfg.dir.join(DATA));
    }
    lua.set_app_data(State {
        id: cfg.id,
        dir: cfg.dir.clone(),
        granted: cfg.granted,
        host: cfg.host,
        exports,
        tx,
        commands: HashMap::new(),
        callbacks: HashMap::new(),
        menu: Vec::new(),
        timers: Vec::new(),
        requests: HashMap::new(),
        settings: cfg.settings,
        next_id: 0,
        sends: VecDeque::new(),
        sockets: std::collections::HashSet::new(),
        connects: VecDeque::new(),
        line_ops: VecDeque::new(),
        tab_ops: VecDeque::new(),
        watches: Vec::new(),
        socket_backlog: Arc::default(),
        in_line: false,
        ready: false,
    });
    if let Err(e) = api::install(&lua) {
        return fail(describe(&e));
    }
    let src = match std::fs::read(cfg.dir.join(ENTRY)) {
        Ok(s) => s,
        Err(_) => return fail(format!("Can’t read {ENTRY}.")),
    };
    deadline.start(INIT);
    let r = lua
        .load(src)
        .set_mode(ChunkMode::Text)
        .set_name(format!("@{ENTRY}"))
        .exec();
    deadline.clear();
    if let Err(e) = r {
        return fail(describe(&e));
    }
    let _ = state::with(&lua, |s| s.ready = true);
    let _ = ready.send(Ok(()));
    Some((lua, deadline))
}

fn serve(
    lua: &Lua,
    deadline: &Deadline,
    rx: &mpsc::Receiver<Msg>,
    queued_lines: &AtomicUsize,
    (queued_events, skipped): (&AtomicUsize, &AtomicUsize),
) {
    let mut strikes = 0;
    loop {
        let next = state::with(lua, |s| s.timers.iter().map(|t| t.0).min())
            .ok()
            .flatten();
        let msg = match next {
            Some(at) => match rx.recv_timeout(at.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            },
        };
        match msg {
            Some(Msg::Stop) => return,
            Some(Msg::Line { line, reply }) => {
                queued_lines.fetch_sub(1, Ordering::Relaxed);
                let change = on_line(lua, deadline, line, reply.is_some(), &mut strikes);
                if let Some(reply) = reply {
                    reply(change);
                }
            }
            Some(Msg::Chat(events)) => {
                queued_events.fetch_sub(events.len(), Ordering::Relaxed);
                let missed = skipped.swap(0, Ordering::Relaxed);
                if missed > 0 {
                    warn(lua, &format!("Skipped {missed} chat events: the plugin fell behind."));
                }
                api::events::deliver(lua, deadline, events);
            }
            Some(m) => handle(lua, deadline, m),
            None => {}
        }
        run_timers(lua, deadline);
    }
}

fn on_line(
    lua: &Lua,
    deadline: &Deadline,
    line: Line,
    may_change: bool,
    strikes: &mut u32,
) -> Option<Change> {
    let (f, paused) = state::with(lua, |s| {
        let paused = s.exports.lock().unwrap_or_else(|p| p.into_inner()).paused;
        (
            s.callbacks.get(&Event::Message).and_then(|k| function(lua, k)),
            paused,
        )
    })
    .ok()?;
    let f = f.filter(|_| !paused)?;
    let arg = convert::line(lua, &line).ok()?;
    let _ = state::with(lua, |s| s.in_line = true);
    let r = call::<Value>(deadline, LINE, &f, arg);
    let _ = state::with(lua, |s| s.in_line = false);
    match r {
        Ok(v) => {
            *strikes = 0;
            if !may_change {
                return None;
            }
            match convert::change(&v) {
                Ok(c) => c.filter(|c| !c.is_empty()),
                Err(e) => {
                    warn(lua, &format!("Message callback: {e}"));
                    None
                }
            }
        }
        Err(e) => {
            warn(lua, &format!("Message callback: {e}"));
            if is_timeout(&e) {
                *strikes += 1;
                if *strikes >= STRIKES {
                    let _ = state::with(lua, |s| s.publish(|x| x.paused = true));
                    warn(
                        lua,
                        "Paused the Message callback: it was too slow 3 times in a row. Reload to try again.",
                    );
                }
            }
            None
        }
    }
}

fn handle(lua: &Lua, deadline: &Deadline, msg: Msg) {
    match msg {
        Msg::Sending { channel, text, reply } => {
            let Some(f) = callback(lua, Event::Sending) else {
                return reply(Sent::Unchanged);
            };
            let ctx = convert::sending(lua, &channel, &text);
            let out = ctx
                .map_err(|e| describe(&e))
                .and_then(|ctx| call::<Value>(deadline, QUICK, &f, ctx));
            reply(match out.and_then(|v| convert::sent(&v)) {
                Ok(s) => s,
                Err(e) => {
                    warn(lua, &format!("Sending callback: {e}"));
                    Sent::Unchanged
                }
            });
        }
        Msg::Complete { req, reply } => {
            let Some(f) = callback(lua, Event::CompletionRequested) else {
                return reply(Completion::default());
            };
            let out = convert::to_lua(lua, &req).and_then(|ev| call::<Value>(deadline, QUICK, &f, ev));
            reply(match out.and_then(|v| convert::completion(&v)) {
                Ok(c) => c,
                Err(e) => {
                    warn(lua, &format!("CompletionRequested callback: {e}"));
                    Completion::default()
                }
            });
        }
        Msg::Command {
            name,
            words,
            channel,
            reply,
        } => {
            let f = state::with(lua, |s| s.commands.get(&name).and_then(|k| function(lua, k)))
                .ok()
                .flatten();
            let Some(f) = f else {
                return reply(Err(format!("/{name} is gone.")));
            };
            let out = convert::command_ctx(lua, words, &channel)
                .map_err(|e| describe(&e))
                .and_then(|ctx| call::<Value>(deadline, CALLBACK, &f, ctx));
            reply(out.map(|v| {
                match v {
                    Value::String(s) => s
                        .to_str()
                        .ok()
                        .map(|s| s.to_owned())
                        .filter(|s| !s.trim().is_empty()),
                    _ => None,
                }
            }));
        }
        Msg::Menu {
            index,
            channel,
            line,
            user,
            reply,
        } => {
            let f = state::with(lua, |s| s.menu.get(index).and_then(|k| function(lua, k)))
                .ok()
                .flatten();
            let Some(f) = f else {
                return reply(Err("That entry is gone.".into()));
            };
            let out = convert::menu_ctx(lua, &channel, line.as_ref(), user.as_ref())
                .map_err(|e| describe(&e))
                .and_then(|ctx| call::<Value>(deadline, CALLBACK, &f, ctx));
            reply(out.map(drop));
        }
        Msg::Repl { code, reply } => reply(repl::eval(lua, deadline, &code)),
        Msg::Settings(values) => {
            let _ = state::with(lua, |s| s.settings = values);
            if let Some(f) = callback(lua, Event::SettingsChanged)
                && let Err(e) = convert::settings(lua)
                    .map_err(|e| describe(&e))
                    .and_then(|t| call::<()>(deadline, CALLBACK, &f, t))
            {
                warn(lua, &format!("SettingsChanged callback: {e}"));
            }
        }
        Msg::Http { id, res } => api::http::finish(lua, deadline, id, res),
        Msg::Socket { id, event } => api::socket::deliver(lua, deadline, id, event),
        Msg::Line { .. } | Msg::Chat(_) | Msg::Stop => {}
    }
}

fn callback(lua: &Lua, e: Event) -> Option<mlua::Function> {
    state::with(lua, |s| s.callbacks.get(&e).and_then(|k| function(lua, k)))
        .ok()
        .flatten()
}

fn run_timers(lua: &Lua, deadline: &Deadline) {
    let now = Instant::now();
    let due = state::with(lua, |s| {
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut s.timers)
            .into_iter()
            .partition(|t| t.0 <= now);
        s.timers = keep;
        due
    })
    .unwrap_or_default();
    for (_, _, key) in due {
        let f = function(lua, &key);
        let _ = lua.remove_registry_value(key);
        if let Some(f) = f
            && let Err(e) = call::<()>(deadline, CALLBACK, &f, ())
        {
            warn(lua, &format!("later: {e}"));
        }
    }
}

pub fn warn(lua: &Lua, text: &str) {
    let _ = state::with(lua, |s| s.log(Level::Warning, text));
}
