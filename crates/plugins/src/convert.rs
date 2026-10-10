//! S5 Values between Rust and Lua: lines and contexts in, changes and answers out.

use mlua::{Lua, LuaSerdeExt, Table, Value};
use serde::Serialize;

use crate::api::channel::ChannelRef;
use crate::host::{Change, Completion, Line, Sent};
use crate::state;

/// Longest text a callback may put in a line.
const MAX_LINE: usize = 1000;
const MAX_COMPLETIONS: usize = 50;

pub fn to_lua<T: Serialize>(lua: &Lua, v: &T) -> Result<Value, String> {
    lua.to_value(v).map_err(|e| state::describe(&e))
}

pub fn line(lua: &Lua, line: &Line) -> mlua::Result<Value> {
    lua.to_value(line)
}

/// `nil`: as is. `{ hide = true }`, `{ text = "…" }`, `{ highlight = true | "#rrggbb" }`.
pub fn change(v: &Value) -> Result<Option<Change>, String> {
    let t = match v {
        Value::Nil => return Ok(None),
        Value::Table(t) => t,
        _ => return Err("return nil or a table like { hide = true }".into()),
    };
    let mut c = Change {
        hide: t
            .get::<Option<bool>>("hide")
            .map_err(|_| "hide must be true or false")?
            .unwrap_or(false),
        ..Change::default()
    };
    if let Some(text) = t
        .get::<Option<String>>("text")
        .map_err(|_| "text must be a string")?
    {
        c.text = Some(text.chars().take(MAX_LINE).collect());
    }
    match t.get::<Value>("highlight").map_err(|e| e.to_string())? {
        Value::Nil | Value::Boolean(false) => {}
        Value::Boolean(true) => c.highlight = Some(String::new()),
        Value::String(s) => {
            let s = s.to_str().map_err(|e| e.to_string())?;
            c.highlight =
                Some(color(&s).ok_or_else(|| format!("highlight \"{}\" isn't a #rrggbb color", &*s))?);
        }
        _ => return Err("highlight must be true or a color".into()),
    }
    Ok(Some(c))
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa`, lowercase and long. The line mixes it in like the
/// user's own highlight colors.
pub fn color(s: &str) -> Option<String> {
    let hex = s.strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hex = hex.to_ascii_lowercase();
    match hex.len() {
        3 => {
            let full: String = hex.chars().flat_map(|c| [c, c]).collect();
            Some(format!("#{full}"))
        }
        6 | 8 => Some(format!("#{hex}")),
        _ => None,
    }
}

/// `nil`: send as is. A string: send that. `false`: don't send.
pub fn sent(v: &Value) -> Result<Sent, String> {
    match v {
        Value::Nil | Value::Boolean(true) => Ok(Sent::Unchanged),
        Value::Boolean(false) => Ok(Sent::Stop),
        Value::String(s) => {
            let s = s.to_str().map_err(|e| e.to_string())?;
            if s.trim().is_empty() {
                Ok(Sent::Stop)
            } else {
                Ok(Sent::Text(s.chars().take(MAX_LINE).collect()))
            }
        }
        _ => Err("return nil, false or a string".into()),
    }
}

/// `{ values = { … }, hide_others = bool }`.
pub fn completion(v: &Value) -> Result<Completion, String> {
    let t = match v {
        Value::Nil => return Ok(Completion::default()),
        Value::Table(t) => t,
        _ => return Err("return a table like { values = {}, hide_others = false }".into()),
    };
    let values: Option<Vec<String>> = t.get("values").map_err(|_| "values must be a list of strings")?;
    Ok(Completion {
        values: values
            .unwrap_or_default()
            .into_iter()
            .filter(|v| !v.is_empty() && v.chars().count() <= 100)
            .take(MAX_COMPLETIONS)
            .collect(),
        hide_others: t
            .get::<Option<bool>>("hide_others")
            .map_err(|e| e.to_string())?
            .unwrap_or(false),
    })
}

/// Chatterino's command context: `words` (the command first) and `channel`.
pub fn command_ctx(lua: &Lua, words: Vec<String>, channel: &str) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("words", lua.create_sequence_from(words)?)?;
    t.set("channel", ChannelRef::new(channel))?;
    Ok(t)
}

pub fn menu_ctx(lua: &Lua, channel: &str, message: Option<&Line>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("channel", ChannelRef::new(channel))?;
    if let Some(m) = message {
        t.set("message", line(lua, m)?)?;
    }
    Ok(t)
}

pub fn sending(lua: &Lua, channel: &str, text: &str) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("channel", ChannelRef::new(channel))?;
    t.set("text", text)?;
    Ok(t)
}

/// The plugin's settings as a table.
pub fn settings(lua: &Lua) -> mlua::Result<Value> {
    let values = state::with(lua, |s| s.settings.clone())?;
    lua.to_value(&values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_normalized() {
        assert_eq!(color("#F00").as_deref(), Some("#ff0000"));
        assert_eq!(color("#00ff00").as_deref(), Some("#00ff00"));
        assert_eq!(color("#00ff0080").as_deref(), Some("#00ff0080"));
        for bad in ["red", "#12", "#gggggg", "00ff00", "#1234567"] {
            assert_eq!(color(bad), None, "{bad}");
        }
    }

    #[test]
    fn answers_from_lua() {
        let lua = Lua::new();
        let eval = |code: &str| lua.load(code).eval::<Value>().unwrap();
        assert_eq!(change(&eval("return nil")), Ok(None));
        let c = change(&eval("return { hide = true, text = 'x', highlight = '#abc' }"))
            .unwrap()
            .unwrap();
        assert!(c.hide);
        assert_eq!(c.text.as_deref(), Some("x"));
        assert_eq!(c.highlight.as_deref(), Some("#aabbcc"));
        assert_eq!(
            change(&eval("return { highlight = true }"))
                .unwrap()
                .unwrap()
                .highlight
                .as_deref(),
            Some("")
        );
        assert!(change(&eval("return 5")).is_err());
        assert!(change(&eval("return { highlight = 'blue' }")).is_err());
        assert_eq!(sent(&eval("return nil")), Ok(Sent::Unchanged));
        assert_eq!(sent(&eval("return false")), Ok(Sent::Stop));
        assert_eq!(sent(&eval("return 'hi'")), Ok(Sent::Text("hi".into())));
        let c = completion(&eval("return { values = { 'a', 'b' }, hide_others = true }")).unwrap();
        assert_eq!(c.values, ["a", "b"]);
        assert!(c.hide_others);
    }
}
