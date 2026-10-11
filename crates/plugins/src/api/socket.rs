//! S6 `cyex.WebSocket`, as in Chatterino: `new(url, options)`, `send_text`, `send_binary`,
//! `close`, and the `on_open`, `on_text`, `on_binary`, `on_close` handlers as fields. Needs
//! the Network permission; the host must be on the plugin's list. Up to 4 open at once and
//! 10 new ones a minute. It closes when the plugin stops or the object is collected: the
//! handlers live on the object, so a handler that uses its own socket doesn't keep it alive.

use std::sync::atomic::Ordering;

use mlua::{AnyUserData, Function, Lua, MultiValue, Table, UserData, UserDataFields, UserDataMethods, Value};
use tokio::sync::mpsc;

use super::http::{MAX_HEADERS, check_header};
use crate::host::{Emit, SocketEvent, SocketOut, SocketRequest};
use crate::perm::check_ws_url;
use crate::runtime::{CALLBACK, Msg, warn};
use crate::sandbox::Deadline;
use crate::socket::MAX_MESSAGE;
use crate::state::{self, CONNECT_LIMIT, call};

pub const MAX_SOCKETS: usize = 4;
/// Messages waiting to be sent, per socket.
const QUEUE: usize = 64;
/// Events waiting for the plugin's thread, all its sockets together.
const MAX_BACKLOG: usize = 1000;
const HANDLERS: [&str; 4] = ["on_open", "on_text", "on_binary", "on_close"];
/// The plugin's sockets by id, held weakly so collecting one closes it.
const OPEN: &str = "cyex.sockets";

pub struct Socket {
    id: u64,
    /// `None` once closed; dropping it closes the connection.
    tx: Option<mpsc::Sender<SocketOut>>,
}

pub fn table(lua: &Lua) -> mlua::Result<Table> {
    let weak = lua.create_table()?;
    let meta = lua.create_table()?;
    meta.set("__mode", "v")?;
    weak.set_metatable(Some(meta))?;
    lua.set_named_registry_value(OPEN, weak)?;
    let t = lua.create_table()?;
    t.set("new", lua.create_function(new)?)?;
    Ok(t)
}

fn new(lua: &Lua, (url, options): (String, Option<Table>)) -> mlua::Result<AnyUserData> {
    let (hosts, plugin, backlog) = state::with(lua, |s| {
        let hosts = s
            .granted
            .network
            .clone()
            .ok_or_else(|| mlua::Error::runtime("sockets need the Network permission"))?;
        if s.sockets.len() >= MAX_SOCKETS {
            return Err(mlua::Error::runtime(format!("up to {MAX_SOCKETS} open sockets")));
        }
        Ok((hosts, s.tx.clone(), s.socket_backlog.clone()))
    })??;
    let url = check_ws_url(&url, &hosts).map_err(mlua::Error::runtime)?;
    let handlers = lua.create_table()?;
    let mut headers = Vec::new();
    if let Some(o) = &options {
        if let Some(h) = o.get::<Option<Table>>("headers")? {
            for pair in h.pairs::<String, String>() {
                let (k, v) = pair?;
                check_header(&k, &v)?;
                headers.push((k, v));
            }
            if headers.len() > MAX_HEADERS {
                return Err(mlua::Error::runtime(format!("up to {MAX_HEADERS} headers")));
            }
        }
        for name in HANDLERS {
            handlers.set(name, o.get::<Option<Function>>(name)?)?;
        }
    }
    let id = state::with(lua, |s| {
        if !s.take_connect() {
            return Err(mlua::Error::runtime(format!("up to {CONNECT_LIMIT} new sockets a minute")));
        }
        let id = s.next_id();
        s.sockets.insert(id);
        Ok(id)
    })??;
    let (tx, rx) = mpsc::channel(QUEUE);
    let emit: Emit = Box::new(move |event| {
        let closing = matches!(event, SocketEvent::Closed { .. });
        if !closing && backlog.load(Ordering::Relaxed) >= MAX_BACKLOG {
            return false;
        }
        backlog.fetch_add(1, Ordering::Relaxed);
        plugin.send(Msg::Socket { id, event }).is_ok()
    });
    let ud = lua.create_userdata(Socket { id, tx: Some(tx) })?;
    ud.set_user_value(handlers)?;
    lua.named_registry_value::<Table>(OPEN)?.set(id, &ud)?;
    let req = SocketRequest {
        url: url.to_string(),
        headers,
    };
    state::with(lua, |s| s.host.socket(req, rx, emit))?;
    Ok(ud)
}

impl Socket {
    fn send(&self, msg: SocketOut, len: usize) -> mlua::Result<()> {
        if len > MAX_MESSAGE {
            return Err(mlua::Error::runtime("a message can be up to 1 MB"));
        }
        let tx = self.tx.as_ref().ok_or_else(|| mlua::Error::runtime("the socket is closed"))?;
        tx.try_send(msg).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                mlua::Error::runtime(format!("up to {QUEUE} messages can wait to be sent"))
            }
            mpsc::error::TrySendError::Closed(_) => mlua::Error::runtime("the socket is closed"),
        })
    }
}

impl UserData for Socket {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        for name in HANDLERS {
            f.add_field_function_get(name, move |_, ud: AnyUserData| ud.user_value::<Table>()?.get::<Value>(name));
            f.add_field_function_set(name, move |_, ud: AnyUserData, v: Option<Function>| {
                ud.user_value::<Table>()?.set(name, v)
            });
        }
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("send_text", |_, this, text: mlua::LuaString| {
            let text = text.to_str()?.to_owned();
            let len = text.len();
            this.send(SocketOut::Text(text), len)
        });
        m.add_method("send_binary", |_, this, data: mlua::LuaString| {
            let data = data.as_bytes().to_vec();
            let len = data.len();
            this.send(SocketOut::Binary(data), len)
        });
        m.add_method_mut("close", |_, this, ()| {
            this.tx = None;
            Ok(())
        });
        m.add_meta_method("__tostring", |_, this, ()| Ok(format!("WebSocket({})", this.id)));
    }
}

/// A socket's event on the plugin's thread: its handler runs, if it has one and the object
/// is still around. A closed socket no longer counts toward the limit.
pub fn deliver(lua: &Lua, deadline: &Deadline, id: u64, event: SocketEvent) {
    let _ = state::with(lua, |s| s.socket_backlog.fetch_sub(1, Ordering::Relaxed));
    let (name, args): (&str, MultiValue) = match event {
        SocketEvent::Open => ("on_open", MultiValue::new()),
        SocketEvent::Text(t) => match lua.create_string(t) {
            Ok(s) => ("on_text", MultiValue::from_iter([Value::String(s)])),
            Err(_) => return,
        },
        SocketEvent::Binary(b) => match lua.create_string(b) {
            Ok(s) => ("on_binary", MultiValue::from_iter([Value::String(s)])),
            Err(_) => return,
        },
        SocketEvent::Closed { code, reason } => {
            let _ = state::with(lua, |s| s.sockets.remove(&id));
            let reason = lua.create_string(reason).map(Value::String).unwrap_or(Value::Nil);
            let code = code.map_or(Value::Nil, |c| Value::Integer(c.into()));
            ("on_close", MultiValue::from_iter([code, reason]))
        }
    };
    let handler = lua
        .named_registry_value::<Table>(OPEN)
        .and_then(|open| open.get::<Option<AnyUserData>>(id))
        .ok()
        .flatten()
        .and_then(|ud| ud.user_value::<Table>().ok())
        .and_then(|h| h.get::<Option<Function>>(name).ok().flatten());
    if let Some(f) = handler
        && let Err(e) = call::<()>(deadline, CALLBACK, &f, args)
    {
        warn(lua, &format!("WebSocket {name}: {e}"));
    }
}
