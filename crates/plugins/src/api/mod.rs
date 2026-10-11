//! S5 The `cyex` table. Names follow Chatterino's `c2` where they mean the same, so a
//! plugin ports by renaming; lines are our own (see `docs/plugins.md`).

pub mod channel;
mod data;
pub mod events;
pub mod http;
mod json;
mod message;
pub mod socket;

use std::time::{Duration, Instant};

use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};

use crate::host::Level;
use crate::runtime::{Event, MenuExport, MenuPlace};
use crate::state::{self, MAX_TIMERS};

const MAX_COMMANDS: usize = 50;
const MAX_MENU: usize = 20;
const MAX_LOG: usize = 2000;

pub fn install(lua: &Lua) -> mlua::Result<()> {
    let cyex = lua.create_table()?;
    cyex.set("LogLevel", names(lua, &["Debug", "Info", "Warning", "Critical"])?)?;
    cyex.set("EventType", names(lua, &Event::ALL.map(Event::name))?)?;
    cyex.set(
        "log",
        lua.create_function(|lua, (level, args): (String, MultiValue)| log(lua, &level, args))?,
    )?;
    cyex.set("register_command", lua.create_function(register_command)?)?;
    cyex.set("register_callback", lua.create_function(register_callback)?)?;
    cyex.set("register_menu_entry", lua.create_function(register_menu_entry)?)?;
    cyex.set("later", lua.create_function(later)?)?;
    cyex.set("current_account", lua.create_function(current_account)?)?;
    cyex.set(
        "get_layout",
        lua.create_function(|lua, ()| {
            let layout = state::with(lua, |s| s.host.layout())?;
            lua.to_value(&layout)
        })?,
    )?;
    cyex.set("get_setting", lua.create_function(get_setting)?)?;
    cyex.set("open_channel", lua.create_function(open_channel)?)?;
    cyex.set("select_tab", lua.create_function(select_tab)?)?;
    cyex.set("Channel", channel::table(lua)?)?;
    cyex.set("HTTPRequest", http::table(lua)?)?;
    cyex.set("WebSocket", socket::table(lua)?)?;
    message::install(lua, &cyex)?;
    cyex.set("json", json::table(lua)?)?;
    cyex.set("data", data::table(lua)?)?;
    let g = lua.globals();
    g.set("cyex", cyex)?;
    g.set(
        "print",
        lua.create_function(|lua, args: MultiValue| log(lua, "Debug", args))?,
    )?;
    Ok(())
}

/// An enum as a table of its names: `cyex.LogLevel.Info == "Info"`.
fn names(lua: &Lua, list: &[&str]) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for n in list {
        t.set(*n, *n)?;
    }
    Ok(t)
}

fn log(lua: &Lua, level: &str, args: MultiValue) -> mlua::Result<()> {
    let level = match level {
        "Debug" => Level::Debug,
        "Info" => Level::Info,
        "Warning" => Level::Warning,
        "Critical" => Level::Critical,
        other => return Err(mlua::Error::runtime(format!("unknown log level \"{other}\""))),
    };
    let tostring: Function = lua.globals().get("tostring")?;
    let mut parts = Vec::with_capacity(args.len());
    for v in args {
        parts.push(tostring.call::<String>(v)?);
    }
    let mut text = parts.join(" ");
    if text.len() > MAX_LOG {
        let cut = (0..=MAX_LOG)
            .rev()
            .find(|i| text.is_char_boundary(*i))
            .unwrap_or(0);
        text.truncate(cut);
        text.push('…');
    }
    state::with(lua, |s| s.log(level, &text))
}

fn register_command(lua: &Lua, (name, f): (String, Function)) -> mlua::Result<bool> {
    let name = name.trim_start_matches('/').to_ascii_lowercase();
    let ok = !name.is_empty()
        && name.len() <= 30
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
    if !ok {
        return Err(mlua::Error::runtime(format!(
            "\"{name}\" can't be a command: letters, digits, _ and -"
        )));
    }
    let key = lua.create_registry_value(f)?;
    state::with(lua, |s| {
        if s.host.reserved(&name) {
            return Err(mlua::Error::runtime(format!(
                "/{name} belongs to the app or Twitch"
            )));
        }
        if !s.commands.contains_key(&name) && s.commands.len() >= MAX_COMMANDS {
            return Err(mlua::Error::runtime(format!("up to {MAX_COMMANDS} commands")));
        }
        s.commands.insert(name.clone(), key);
        let mut names: Vec<String> = s.commands.keys().cloned().collect();
        names.sort();
        s.publish(|x| x.commands = names);
        Ok(true)
    })?
}

fn register_callback(lua: &Lua, (event, f): (String, Function)) -> mlua::Result<()> {
    let e = Event::ALL
        .into_iter()
        .find(|e| e.name() == event)
        .ok_or_else(|| mlua::Error::runtime(format!("unknown event \"{event}\"")))?;
    let key = lua.create_registry_value(f)?;
    state::with(lua, |s| {
        let allowed = match e {
            Event::Message
            | Event::MessageAppended
            | Event::MessageReplaced
            | Event::MessageDeleted
            | Event::ChatCleared => s.granted.read_chat,
            Event::Sending => s.granted.send,
            Event::CompletionRequested
            | Event::SettingsChanged
            | Event::ChannelRenamed
            | Event::LiveChanged
            | Event::LayoutChanged => true,
        };
        if !allowed {
            let need = if e == Event::Sending {
                "SendMessages"
            } else {
                "ReadChat or ChangeChat"
            };
            return Err(mlua::Error::runtime(format!(
                "{} needs the {need} permission",
                e.name()
            )));
        }
        s.callbacks.insert(e, key);
        s.publish_events();
        Ok(())
    })?
}

/// An entry in a line's right-click menu, or (S9) the chat's ⋯ menu with
/// `{ where = "chat" }`, or (S11) a name's with `{ where = "name" }`. `group` puts entries
/// in a submenu; `lines` limits message entries to some kinds of line. Returns its index.
fn register_menu_entry(lua: &Lua, (label, f, options): (String, Function, Option<Table>)) -> mlua::Result<usize> {
    let label = menu_label(&label).ok_or_else(|| mlua::Error::runtime("the label is empty"))?;
    let mut entry = MenuExport {
        label,
        ..MenuExport::default()
    };
    if let Some(o) = options {
        entry.place = match o.get::<Option<String>>("where")?.as_deref() {
            None | Some("message") => MenuPlace::Message,
            Some("chat") => MenuPlace::Chat,
            Some("name") => MenuPlace::Name,
            Some(other) => {
                return Err(mlua::Error::runtime(format!("where \"{other}\" isn't message, chat or name")));
            }
        };
        entry.group = o.get::<Option<String>>("group")?.and_then(|g| menu_label(&g));
        if let Some(lines) = o.get::<Option<Vec<String>>>("lines")? {
            for l in &lines {
                if !LINE_KINDS.contains(&l.as_str()) {
                    return Err(mlua::Error::runtime(format!("lines takes chat, action, notice and plugin, not \"{l}\"")));
                }
            }
            entry.lines = lines;
        }
    }
    let key = lua.create_registry_value(f)?;
    state::with(lua, |s| {
        if s.menu.len() >= MAX_MENU {
            return Err(mlua::Error::runtime(format!("up to {MAX_MENU} menu entries")));
        }
        s.menu.push(key);
        let index = s.menu.len() - 1;
        s.publish(|x| x.menu.push(entry));
        Ok(index)
    })?
}

/// S9 The kinds of line a message entry can be limited to.
const LINE_KINDS: [&str; 4] = ["chat", "action", "notice", "plugin"];

/// A label as menus write them (V101-30): trimmed, up to 40, a capital first, no period.
fn menu_label(raw: &str) -> Option<String> {
    let s: String = raw.trim().chars().take(40).collect();
    let s = s.trim_end_matches('.').trim_end();
    let mut chars = s.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

/// Calls `f` once after `ms` milliseconds.
fn later(lua: &Lua, (f, ms): (Function, i64)) -> mlua::Result<()> {
    if !(1..=86_400_000).contains(&ms) {
        return Err(mlua::Error::runtime("later takes 1 ms to 24 h"));
    }
    let key = lua.create_registry_value(f)?;
    state::with(lua, |s| {
        if s.timers.len() >= MAX_TIMERS {
            return Err(mlua::Error::runtime(format!("up to {MAX_TIMERS} waiting timers")));
        }
        let id = s.next_id();
        s.timers
            .push((Instant::now() + Duration::from_millis(ms as u64), id, key));
        Ok(())
    })?
}

/// `{ login, id, name }`, or nil when anonymous.
fn current_account(lua: &Lua, (): ()) -> mlua::Result<Value> {
    let Some(a) = state::with(lua, |s| s.host.account())? else {
        return Ok(Value::Nil);
    };
    let t = lua.create_table()?;
    t.set("login", a.login)?;
    t.set("id", a.id)?;
    t.set("name", a.name)?;
    Ok(Value::Table(t))
}

/// S13 Room in the Tabs permission and its limit.
fn take_tab(s: &mut state::State) -> mlua::Result<()> {
    if !s.granted.tabs {
        return Err(mlua::Error::runtime("tabs need the Tabs permission"));
    }
    if !s.take_tab() {
        return Err(mlua::Error::runtime(format!(
            "up to {} tab switches a minute",
            state::TAB_LIMIT
        )));
    }
    Ok(())
}

/// S13 Switches to the first tab showing the channel, or opens a tab for it.
fn open_channel(lua: &Lua, login: String) -> mlua::Result<()> {
    let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
    let ok = (1..=25).contains(&login.len()) && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(mlua::Error::runtime(format!("\"{login}\" isn't a Twitch login")));
    }
    state::with(lua, |s| {
        take_tab(s)?;
        s.host.open_channel(&s.id, &login);
        Ok(())
    })?
}

/// S13 Switches to a tab by its id from `get_layout`.
fn select_tab(lua: &Lua, id: String) -> mlua::Result<()> {
    if id.is_empty() || id.len() > 64 {
        return Err(mlua::Error::runtime("that isn't a tab id"));
    }
    state::with(lua, |s| {
        take_tab(s)?;
        s.host.select_tab(&s.id, &id);
        Ok(())
    })?
}

fn get_setting(lua: &Lua, key: String) -> mlua::Result<Value> {
    let v = state::with(lua, |s| s.settings.get(&key).cloned())?;
    match v {
        Some(v) => lua.to_value(&v),
        None => Ok(Value::Nil),
    }
}
