//! S5 `cyex.json`: `parse`, `stringify`, and `null` for JSON's null.

use mlua::{Lua, LuaSerdeExt, Table, Value};

pub fn table(lua: &Lua) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|lua, text: String| {
            let v: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| mlua::Error::runtime(format!("bad JSON: {e}")))?;
            lua.to_value(&v)
        })?,
    )?;
    t.set(
        "stringify",
        lua.create_function(|lua, (v, pretty): (Value, Option<bool>)| {
            let v: serde_json::Value = lua.from_value(v)?;
            let s = if pretty.unwrap_or(false) {
                serde_json::to_string_pretty(&v)
            } else {
                serde_json::to_string(&v)
            };
            s.map_err(|e| mlua::Error::runtime(e.to_string()))
        })?,
    )?;
    t.set("null", lua.null())?;
    Ok(t)
}
