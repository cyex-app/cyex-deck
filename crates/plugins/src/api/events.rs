//! S8 Chat events after the fact: a line came in or was replaced, lines were deleted, a
//! chat was cleared, a channel was renamed. Through `cyex.register_callback` for every chat,
//! or Chatterino's per-channel `ch:on_message_appended(fn)` and friends, which hand back a
//! handle to disconnect or block. A plugin never hears about its own lines. S12 Live and
//! offline and layout changes come the same way, through `cyex.register_callback` only.

use mlua::{Function, Lua, LuaSerdeExt, RegistryKey, UserData, UserDataMethods, Value};

use crate::host::ChatEvent;
use crate::runtime::{Event, QUICK, warn};
use crate::sandbox::Deadline;
use crate::state::{self, call, function};

/// Per-channel handlers a plugin may hold.
const MAX_WATCHES: usize = 100;

/// A per-channel handler.
pub struct Watch {
    pub(crate) id: u64,
    pub(crate) channel: String,
    pub(crate) event: Event,
    pub(crate) key: RegistryKey,
    pub(crate) blocked: bool,
}

/// Chatterino's `ConnectionHandle`.
pub struct Handle(u64);

/// Adds a per-channel handler. ReadChat, except for renames.
pub fn watch(lua: &Lua, channel: &str, event: Event, f: Function) -> mlua::Result<Handle> {
    let key = lua.create_registry_value(f)?;
    state::with(lua, |s| {
        if event != Event::ChannelRenamed && !s.granted.read_chat {
            return Err(mlua::Error::runtime("chat events need the ReadChat permission"));
        }
        if s.watches.len() >= MAX_WATCHES {
            return Err(mlua::Error::runtime(format!("up to {MAX_WATCHES} handlers")));
        }
        let id = s.next_id();
        s.watches.push(Watch {
            id,
            channel: channel.to_owned(),
            event,
            key,
            blocked: false,
        });
        s.publish_events();
        Ok(Handle(id))
    })?
}

impl UserData for Handle {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("disconnect", |lua, this, ()| {
            let gone = state::with(lua, |s| {
                let at = s.watches.iter().position(|w| w.id == this.0);
                let gone = at.map(|i| s.watches.remove(i));
                s.publish_events();
                gone
            })?;
            if let Some(w) = gone {
                lua.remove_registry_value(w.key)?;
            }
            Ok(())
        });
        m.add_method("block", |lua, this, ()| set_blocked(lua, this.0, true));
        m.add_method("unblock", |lua, this, ()| set_blocked(lua, this.0, false));
        m.add_method("is_blocked", |lua, this, ()| {
            state::with(lua, |s| s.watches.iter().any(|w| w.id == this.0 && w.blocked))
        });
        m.add_method("is_connected", |lua, this, ()| {
            state::with(lua, |s| s.watches.iter().any(|w| w.id == this.0))
        });
    }
}

fn set_blocked(lua: &Lua, id: u64, blocked: bool) -> mlua::Result<()> {
    state::with(lua, |s| {
        if let Some(w) = s.watches.iter_mut().find(|w| w.id == id) {
            w.blocked = blocked;
        }
    })
}

impl ChatEvent {
    pub fn kind(&self) -> Event {
        match self {
            ChatEvent::Appended(_) => Event::MessageAppended,
            ChatEvent::Replaced(_) => Event::MessageReplaced,
            ChatEvent::Deleted { .. } => Event::MessageDeleted,
            ChatEvent::Cleared { .. } => Event::ChatCleared,
            ChatEvent::Renamed { .. } => Event::ChannelRenamed,
            ChatEvent::Live(_) => Event::LiveChanged,
            ChatEvent::Layout(_) => Event::LayoutChanged,
        }
    }

    pub fn channel(&self) -> &str {
        match self {
            ChatEvent::Appended(l) | ChatEvent::Replaced(l) => &l.channel,
            ChatEvent::Deleted { channel, .. } | ChatEvent::Cleared { channel } | ChatEvent::Renamed { channel, .. } => {
                channel
            }
            ChatEvent::Live(c) => &c.channel,
            ChatEvent::Layout(_) => "",
        }
    }
}

/// The event as Lua sees it: a line table, the layout, or `{ channel, … }`.
fn to_lua(lua: &Lua, e: &ChatEvent) -> mlua::Result<Value> {
    let t = lua.create_table()?;
    match e {
        ChatEvent::Appended(l) | ChatEvent::Replaced(l) => return crate::convert::line(lua, l),
        ChatEvent::Live(c) => return lua.to_value(c),
        ChatEvent::Layout(l) => return lua.to_value(l),
        ChatEvent::Deleted { channel, id, user_id } => {
            t.set("channel", channel.as_str())?;
            t.set("id", id.as_deref())?;
            t.set("user_id", user_id.as_deref())?;
        }
        ChatEvent::Cleared { channel } => t.set("channel", channel.as_str())?,
        ChatEvent::Renamed { channel, old, name } => {
            t.set("channel", channel.as_str())?;
            t.set("old", old.as_str())?;
            t.set("name", name.as_str())?;
        }
    }
    Ok(Value::Table(t))
}

/// A batch of events on the plugin's thread: the callback for each kind, then the
/// channel's own handlers. Chatterino's per-channel handlers for clears and renames get
/// no arguments.
pub fn deliver(lua: &Lua, deadline: &Deadline, events: Vec<ChatEvent>) {
    for e in &events {
        let kind = e.kind();
        let (global, watchers) = state::with(lua, |s| {
            let global = s.callbacks.get(&kind).and_then(|k| function(lua, k));
            let watchers: Vec<Function> = s
                .watches
                .iter()
                .filter(|w| w.event == kind && !w.blocked && w.channel == e.channel())
                .filter_map(|w| function(lua, &w.key))
                .collect();
            (global, watchers)
        })
        .unwrap_or((None, Vec::new()));
        if global.is_none() && watchers.is_empty() {
            continue;
        }
        let Ok(arg) = to_lua(lua, e) else { continue };
        let bare = matches!(kind, Event::ChatCleared | Event::ChannelRenamed);
        let run = |f: &Function, with_arg: bool| {
            let r = if with_arg {
                call::<()>(deadline, QUICK, f, arg.clone())
            } else {
                call::<()>(deadline, QUICK, f, ())
            };
            if let Err(err) = r {
                warn(lua, &format!("{}: {err}", kind.name()));
            }
        };
        if let Some(f) = &global {
            run(f, true);
        }
        for f in &watchers {
            run(f, !bare);
        }
    }
}
