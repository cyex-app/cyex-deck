//! S5 The `cyex` table. Names follow Chatterino's `c2` where they mean the same, so a
//! plugin ports by renaming; lines are our own (see `docs/plugins.md`).

pub mod channel;
mod data;
pub mod http;
mod json;

use std::time::{Duration, Instant};

use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};

use crate::host::Level;
use crate::runtime::Event;
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
    cyex.set("get_setting", lua.create_function(get_setting)?)?;
    cyex.set("Channel", channel::table(lua)?)?;
    cyex.set("HTTPRequest", http::table(lua)?)?;
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
            Event::Message => s.granted.read_chat,
            Event::Sending => s.granted.send,
            Event::CompletionRequested | Event::SettingsChanged => true,
        };
        if !allowed {
            let need = if e == Event::Message {
                "ReadChat or ChangeChat"
            } else {
                "SendMessages"
            };
            return Err(mlua::Error::runtime(format!(
                "{} needs the {need} permission",
                e.name()
            )));
        }
        s.callbacks.insert(e, key);
        let mut events: Vec<Event> = s.callbacks.keys().copied().collect();
        events.sort_by_key(|e| e.name());
        s.publish(|x| x.events = events);
        Ok(())
    })?
}

/// An entry in the message right-click menu. Returns its index.
fn register_menu_entry(lua: &Lua, (label, f): (String, Function)) -> mlua::Result<usize> {
    let label: String = label.trim().chars().take(40).collect();
    if label.is_empty() {
        return Err(mlua::Error::runtime("the label is empty"));
    }
    let key = lua.create_registry_value(f)?;
    state::with(lua, |s| {
        if s.menu.len() >= MAX_MENU {
            return Err(mlua::Error::runtime(format!("up to {MAX_MENU} menu entries")));
        }
        s.menu.push(key);
        let index = s.menu.len() - 1;
        s.publish(|x| x.menu.push(label));
        Ok(index)
    })?
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

fn get_setting(lua: &Lua, key: String) -> mlua::Result<Value> {
    let v = state::with(lua, |s| s.settings.get(&key).cloned())?;
    match v {
        Some(v) => lua.to_value(&v),
        None => Ok(Value::Nil),
    }
}
