//! S5 `cyex.Channel`: an open chat by login, with Chatterino's method names. S7 Plugins add
//! lines of their own, swap them, change any line (ChangeChat) and look one up (ReadChat).
//! S14 They take their own lines back and clear a chat on screen (ChangeChat).

use mlua::{Lua, LuaSerdeExt, Table, UserData, UserDataMethods, Value};

use super::{events, message};
use crate::host::ChannelInfo;
use crate::runtime::Event;
use crate::state::{self, LINE_LIMIT, State};

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

/// S7 Room in the plugin's line limit.
fn take_line(s: &mut State) -> mlua::Result<()> {
    if s.take_line() {
        Ok(())
    } else {
        Err(mlua::Error::runtime(format!("up to {LINE_LIMIT} lines a second")))
    }
}

/// S7 A plugin's own line ids start with this.
fn own_prefix(s: &State) -> String {
    format!("p:{}:", s.id)
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
        m.add_method("add_message", |lua, this, msg: Table| {
            state::with(lua, |s| {
                take_line(s)?;
                let mut line = message::read(&msg, &s.granted).map_err(mlua::Error::runtime)?;
                let own = if line.id.is_empty() {
                    s.next_id().to_string()
                } else {
                    line.id.clone()
                };
                line.id = format!("{}{own}", own_prefix(s));
                let id = line.id.clone();
                s.host.add_line(&s.id, &this.0, line);
                Ok(id)
            })?
        });
        m.add_method("replace_message", |lua, this, (id, msg): (String, Table)| {
            state::with(lua, |s| {
                if !id.starts_with(&own_prefix(s)) {
                    return Err(mlua::Error::runtime("only lines this plugin added can be replaced"));
                }
                take_line(s)?;
                let mut line = message::read(&msg, &s.granted).map_err(mlua::Error::runtime)?;
                line.id = id;
                s.host.replace_line(&s.id, &this.0, line);
                Ok(())
            })?
        });
        m.add_method("remove_message", |lua, this, id: String| {
            state::with(lua, |s| {
                if !id.starts_with(&own_prefix(s)) {
                    return Err(mlua::Error::runtime("only lines this plugin added can be taken back"));
                }
                take_line(s)?;
                s.host.remove_line(&s.id, &this.0, &id);
                Ok(())
            })?
        });
        // Chatterino's name. Only on the user's screen, never on Twitch.
        m.add_method("clear_messages", |lua, this, ()| {
            state::with(lua, |s| {
                if !s.granted.change_chat {
                    return Err(mlua::Error::runtime("clearing a chat needs the ChangeChat permission"));
                }
                take_line(s)?;
                s.host.clear_lines(&s.id, &this.0);
                Ok(())
            })?
        });
        m.add_method("change_message", |lua, this, (id, change): (String, Value)| {
            let change = crate::convert::change(&change)
                .map_err(mlua::Error::runtime)?
                .unwrap_or_default();
            state::with(lua, |s| {
                if !s.granted.change_chat {
                    return Err(mlua::Error::runtime("changing lines needs the ChangeChat permission"));
                }
                take_line(s)?;
                s.host.change_line(&s.id, &this.0, &id, change);
                Ok(())
            })?
        });
        m.add_method("find_message_by_id", |lua, this, id: String| {
            let line = state::with(lua, |s| {
                need_read(s)?;
                if s.in_line {
                    return Err(mlua::Error::runtime("can't read chat inside a Message callback"));
                }
                Ok(s.host.find_line(&this.0, &id))
            })??;
            match line {
                Some(l) => crate::convert::line(lua, &l),
                None => Ok(Value::Nil),
            }
        });
        // S8 Chatterino's per-channel handlers; each hands back a handle.
        for (name, event) in [
            ("on_message_appended", Event::MessageAppended),
            ("on_message_replaced", Event::MessageReplaced),
            ("on_messages_cleared", Event::ChatCleared),
            ("on_display_name_changed", Event::ChannelRenamed),
        ] {
            m.add_method(name, move |lua, this, f: mlua::Function| events::watch(lua, &this.0, event, f));
        }
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
