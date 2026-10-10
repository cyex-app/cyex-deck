//! S5 `cyex.Channel`: an open chat by login, with Chatterino's method names.

use mlua::{Lua, LuaSerdeExt, Table, UserData, UserDataMethods, Value};

use crate::host::ChannelInfo;
use crate::state::{self, State};

const MAX_SYSTEM: usize = 500;
const MAX_SNAPSHOT: usize = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelRef(String);

impl ChannelRef {
    pub fn new(login: &str) -> Self {
        ChannelRef(login.to_ascii_lowercase())
    }
}

pub fn table(lua: &Lua) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    // nil when no open chat shows it. Inside a Message callback it can't be checked, so
    // the channel comes back either way.
    t.set(
        "by_name",
        lua.create_function(|lua, login: String| {
            let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
            let open = state::with(lua, |s| s.in_line || s.host.channel(&login).is_some())?;
            Ok(open.then(|| ChannelRef::new(&login)))
        })?,
    )?;
    Ok(t)
}

/// Chat state can be read except while a line waits on this plugin.
fn info(lua: &Lua, login: &str) -> mlua::Result<ChannelInfo> {
    state::with(lua, |s| {
        if s.in_line {
            return Err(mlua::Error::runtime(
                "can't read channel state inside a Message callback",
            ));
        }
        s.host
            .channel(login)
            .ok_or_else(|| mlua::Error::runtime(format!("the chat of {login} isn't open")))
    })?
}

fn need_read(s: &State) -> mlua::Result<()> {
    if s.granted.read_chat {
        Ok(())
    } else {
        Err(mlua::Error::runtime("reading chat needs the ReadChat permission"))
    }
}

impl UserData for ChannelRef {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("get_name", |_, this, ()| Ok(this.0.clone()));
        m.add_method("get_display_name", |lua, this, ()| Ok(info(lua, &this.0)?.name));
        m.add_method("get_twitch_id", |lua, this, ()| Ok(info(lua, &this.0)?.id));
        m.add_method("is_broadcaster", |lua, this, ()| {
            Ok(info(lua, &this.0)?.broadcaster)
        });
        m.add_method("is_mod", |lua, this, ()| Ok(info(lua, &this.0)?.moderator));
        m.add_method("is_vip", |lua, this, ()| Ok(info(lua, &this.0)?.vip));
        m.add_method("get_room_modes", |lua, this, ()| {
            lua.to_value(&info(lua, &this.0)?.modes)
        });
        m.add_method("get_stream_status", |lua, this, ()| {
            match info(lua, &this.0)?.stream {
                Some(s) => lua.to_value(&s),
                None => Ok(Value::Nil),
            }
        });
        m.add_method("send_message", |lua, this, text: String| {
            state::with(lua, |s| {
                if !s.granted.send {
                    return Err(mlua::Error::runtime("sending needs the SendMessages permission"));
                }
                if !s.take_send() {
                    return Err(mlua::Error::runtime(format!(
                        "up to {} messages in {} s",
                        state::SEND_LIMIT,
                        state::SEND_WINDOW.as_secs()
                    )));
                }
                s.host.send(&s.id, &this.0, &text).map_err(mlua::Error::runtime)
            })?
        });
        m.add_method("add_system_message", |lua, this, text: String| {
            let text: String = text.chars().take(MAX_SYSTEM).collect();
            state::with(lua, |s| s.host.system(&s.id, &this.0, &text))
        });
        m.add_method("message_snapshot", |lua, this, n: Option<usize>| {
            let lines = state::with(lua, |s| {
                need_read(s)?;
                if s.in_line {
                    return Err(mlua::Error::runtime("can't read chat inside a Message callback"));
                }
                Ok(s.host.lines(&this.0, n.unwrap_or(50).min(MAX_SNAPSHOT)))
            })??;
            let out = lua.create_table()?;
            for l in &lines {
                out.push(crate::convert::line(lua, l)?)?;
            }
            Ok(out)
        });
        m.add_method("last_message", |lua, this, ()| {
            let line = state::with(lua, |s| {
                need_read(s)?;
                if s.in_line {
                    return Err(mlua::Error::runtime("can't read chat inside a Message callback"));
                }
                Ok(s.host.lines(&this.0, 1).pop())
            })??;
            match line {
                Some(l) => crate::convert::line(lua, &l),
                None => Ok(Value::Nil),
            }
        });
        m.add_meta_method("__tostring", |_, this, ()| Ok(format!("Channel({})", this.0)));
        m.add_meta_method("__eq", |_, this, other: mlua::AnyUserData| {
            Ok(other.borrow::<ChannelRef>().is_ok_and(|o| *o == *this))
        });
    }
}
