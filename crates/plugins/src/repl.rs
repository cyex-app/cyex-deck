//! S5 The console: runs a line in the plugin's own Lua and shows what came back. Like
//! Chatterino's REPL it tries `return <line>` first, so `1 + 1` prints 2.

use mlua::chunk::ChunkMode;
use mlua::{Function, Lua, MultiValue, Value};

use crate::runtime::REPL;
use crate::sandbox::Deadline;
use crate::state::describe;

const MAX_OUT: usize = 4000;
const MAX_ITEMS: usize = 30;

pub fn eval(lua: &Lua, deadline: &Deadline, code: &str) -> Result<String, String> {
    let code = code.trim();
    if code.is_empty() {
        return Ok(String::new());
    }
    let f = lua
        .load(format!("return {code}"))
        .set_name("=console")
        .set_mode(ChunkMode::Text)
        .into_function()
        .or_else(|_| {
            lua.load(code)
                .set_name("=console")
                .set_mode(ChunkMode::Text)
                .into_function()
        })
        .map_err(|e| describe(&e))?;
    deadline.start(REPL);
    let r = f.call::<MultiValue>(());
    deadline.clear();
    let values = r.map_err(|e| describe(&e))?;
    let mut out = values
        .iter()
        .map(|v| show(lua, v, 0))
        .collect::<Vec<_>>()
        .join("\t");
    if out.len() > MAX_OUT {
        let cut = (0..=MAX_OUT)
            .rev()
            .find(|i| out.is_char_boundary(*i))
            .unwrap_or(0);
        out.truncate(cut);
        out.push('…');
    }
    Ok(out)
}

/// A value as text: strings quoted, tables two levels deep.
fn show(lua: &Lua, v: &Value, depth: usize) -> String {
    match v {
        Value::Nil => "nil".into(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("{:?}", s.to_string_lossy()),
        Value::Table(t) if depth < 2 => {
            let mut parts = Vec::new();
            let mut more = false;
            for (i, pair) in t.clone().pairs::<Value, Value>().enumerate() {
                if i == MAX_ITEMS {
                    more = true;
                    break;
                }
                let Ok((k, v)) = pair else { continue };
                let key = match &k {
                    Value::String(s) => s.to_string_lossy(),
                    other => format!("[{}]", show(lua, other, 2)),
                };
                parts.push(format!("{key} = {}", show(lua, &v, depth + 1)));
            }
            if more {
                parts.push("…".into());
            }
            format!("{{ {} }}", parts.join(", "))
        }
        Value::Table(_) => "{…}".into(),
        Value::Function(_) => "function".into(),
        // Channels and errors say what they are.
        Value::UserData(_) | Value::Error(_) => lua
            .globals()
            .get::<Function>("tostring")
            .and_then(|f| f.call::<String>(v.clone()))
            .unwrap_or_else(|_| v.type_name().into()),
        Value::LightUserData(l) if l.0.is_null() => "null".into(),
        other => other.type_name().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_values_and_runs_statements() {
        let lua = Lua::new();
        let d = crate::sandbox::new(std::path::Path::new(".")).unwrap().1;
        assert_eq!(eval(&lua, &d, "1 + 1"), Ok("2".into()));
        assert_eq!(eval(&lua, &d, "'a', true"), Ok("\"a\"\ttrue".into()));
        assert_eq!(eval(&lua, &d, "x = 5"), Ok(String::new()));
        assert_eq!(eval(&lua, &d, "x"), Ok("5".into()));
        assert_eq!(eval(&lua, &d, "{ 1, 2 }"), Ok("{ [1] = 1, [2] = 2 }".into()));
        assert!(eval(&lua, &d, "error('boom')").unwrap_err().contains("boom"));
        assert!(eval(&lua, &d, "if then").is_err());
    }
}
