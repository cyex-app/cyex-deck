//! S5 `cyex.HTTPRequest`, as in Chatterino: `create(method, url)`, callbacks, `execute()`.
//! Needs the Network permission; the address must be on the plugin's list of hosts.

use mlua::{Function, Lua, RegistryKey, Table, UserData, UserDataMethods};

use crate::host::{HttpRequest, HttpResponse};
use crate::perm::check_url;
use crate::runtime::{CALLBACK, Msg, warn};
use crate::sandbox::Deadline;
use crate::state::{self, MAX_REQUESTS, Requested, call};

const METHODS: [&str; 6] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];
const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_BODY: usize = 1 << 20;
pub(crate) const MAX_HEADERS: usize = 30;
/// Headers the HTTP client sets itself.
const FIXED: [&str; 5] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "upgrade",
];

pub struct Request {
    req: HttpRequest,
    cb: Requested,
    sent: bool,
}

pub fn table(lua: &Lua) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set(
        "create",
        lua.create_function(|lua, (method, url): (String, String)| {
            let method = method.to_ascii_uppercase();
            if !METHODS.contains(&method.as_str()) {
                return Err(mlua::Error::runtime(format!("unknown method {method}")));
            }
            let hosts = state::with(lua, |s| s.granted.network.clone())?
                .ok_or_else(|| mlua::Error::runtime("requests need the Network permission"))?;
            let url = check_url(&url, &hosts).map_err(mlua::Error::runtime)?;
            Ok(Request {
                req: HttpRequest {
                    method,
                    url: url.to_string(),
                    headers: Vec::new(),
                    body: None,
                    timeout_ms: DEFAULT_TIMEOUT_MS,
                    hosts,
                },
                cb: Requested::default(),
                sent: false,
            })
        })?,
    )?;
    Ok(t)
}

/// A header a plugin may set: a plain name, one line, not one the client sets itself.
pub(crate) fn check_header(name: &str, value: &str) -> mlua::Result<()> {
    let lower = name.to_ascii_lowercase();
    let ok_name = !name.is_empty()
        && name.len() <= 100
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        && !FIXED.contains(&lower.as_str())
        && !lower.starts_with("sec-websocket");
    if !ok_name || value.len() > 2000 || value.contains(['\r', '\n']) {
        return Err(mlua::Error::runtime(format!("can’t set the header {name}")));
    }
    Ok(())
}

fn not_sent(r: &Request) -> mlua::Result<()> {
    if r.sent {
        Err(mlua::Error::runtime("this request was sent already"))
    } else {
        Ok(())
    }
}

fn key(lua: &Lua, f: Function) -> mlua::Result<Option<RegistryKey>> {
    lua.create_registry_value(f).map(Some)
}

impl UserData for Request {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut("on_success", |lua, this, f: Function| {
            not_sent(this)?;
            this.cb.success = key(lua, f)?;
            Ok(())
        });
        m.add_method_mut("on_error", |lua, this, f: Function| {
            not_sent(this)?;
            this.cb.error = key(lua, f)?;
            Ok(())
        });
        m.add_method_mut("finally", |lua, this, f: Function| {
            not_sent(this)?;
            this.cb.finally = key(lua, f)?;
            Ok(())
        });
        m.add_method_mut("set_timeout", |_, this, ms: u64| {
            not_sent(this)?;
            this.req.timeout_ms = ms.clamp(1, MAX_TIMEOUT_MS);
            Ok(())
        });
        m.add_method_mut("set_payload", |_, this, body: mlua::LuaString| {
            not_sent(this)?;
            let bytes = body.as_bytes();
            if bytes.len() > MAX_BODY {
                return Err(mlua::Error::runtime("the payload is over 1 MB"));
            }
            this.req.body = Some(bytes.to_vec());
            Ok(())
        });
        m.add_method_mut("set_header", |_, this, (name, value): (String, String)| {
            not_sent(this)?;
            check_header(&name, &value)?;
            if this.req.headers.len() >= MAX_HEADERS {
                return Err(mlua::Error::runtime(format!("up to {MAX_HEADERS} headers")));
            }
            this.req.headers.push((name, value));
            Ok(())
        });
        m.add_method_mut("execute", |lua, this, ()| {
            not_sent(this)?;
            this.sent = true;
            let cb = std::mem::take(&mut this.cb);
            let req = this.req.clone();
            state::with(lua, |s| {
                if s.requests.len() >= MAX_REQUESTS {
                    return Err(mlua::Error::runtime(format!(
                        "up to {MAX_REQUESTS} requests at once"
                    )));
                }
                let id = s.next_id();
                s.requests.insert(id, cb);
                let tx = s.tx.clone();
                s.host.http(
                    req,
                    Box::new(move |res| {
                        let _ = tx.send(Msg::Http { id, res });
                    }),
                );
                Ok(())
            })?
        });
    }
}

/// Chatterino's `HTTPResponse`.
struct Response(HttpResponse);

impl UserData for Response {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("data", |lua, this, ()| lua.create_string(&this.0.data));
        m.add_method("status", |_, this, ()| {
            Ok((this.0.status != 0).then_some(this.0.status))
        });
        m.add_method("error", |_, this, ()| Ok(this.0.error.clone()));
    }
}

/// An answer (or failure) came: success for statuses below 400, error otherwise, then
/// finally.
pub fn finish(lua: &Lua, deadline: &Deadline, id: u64, res: HttpResponse) {
    let Ok(Some(cb)) = state::with(lua, |s| s.requests.remove(&id)) else {
        return;
    };
    let ok = res.error.is_none() && (1..400).contains(&res.status);
    let Ok(arg) = lua.create_userdata(Response(res)) else {
        return;
    };
    let first = if ok { cb.success } else { cb.error };
    for key in [first, cb.finally].into_iter().flatten() {
        let f = lua.registry_value::<Function>(&key);
        let _ = lua.remove_registry_value(key);
        if let Ok(f) = f
            && let Err(e) = call::<()>(deadline, CALLBACK, &f, arg.clone())
        {
            warn(lua, &format!("HTTPRequest: {e}"));
        }
    }
}
