//! S5 A fresh Lua 5.4 for one plugin: no `io`, no `debug`, `os` cut down to the clock,
//! `load` for text only, `require` inside the plugin's folder, and limits on memory and
//! time that Chatterino lacks.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::chunk::ChunkMode;
use mlua::{Function, HookTriggers, Lua, LuaOptions, MultiValue, StdLib, Table, Value, VmState};

use crate::manifest::DATA;

/// Memory a plugin may use.
pub const MEMORY: usize = 64 << 20;
/// The time check runs this often, in Lua instructions.
const CHECK_EVERY: u32 = 1000;

/// When the running call has to be done. Lua code past it gets an error.
#[derive(Clone)]
pub struct Deadline(Rc<Cell<Option<Instant>>>);

impl Deadline {
    pub fn start(&self, budget: Duration) {
        self.0.set(Some(Instant::now() + budget));
    }

    pub fn clear(&self) {
        self.0.set(None);
    }

    fn passed(&self) -> bool {
        self.0.get().is_some_and(|d| Instant::now() > d)
    }
}

pub fn new(dir: &Path) -> mlua::Result<(Lua, Deadline)> {
    let libs = StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH | StdLib::OS;
    let lua = Lua::new_with(libs, LuaOptions::default())?;
    let deadline = Deadline(Rc::new(Cell::new(None)));
    let d = deadline.clone();
    lua.set_global_hook(
        HookTriggers::new().every_nth_instruction(CHECK_EVERY),
        move |_, _| {
            if d.passed() {
                Err(mlua::Error::runtime("took too long"))
            } else {
                Ok(VmState::Continue)
            }
        },
    )?;
    trim(&lua)?;
    install_require(&lua, dir.to_path_buf())?;
    lua.set_memory_limit(MEMORY)?;
    Ok((lua, deadline))
}

fn trim(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    for name in ["dofile", "loadfile"] {
        g.raw_set(name, Value::Nil)?;
    }
    let string: Table = g.get("string")?;
    string.raw_set("dump", Value::Nil)?;
    // The clock and dates; nothing that touches files, programs or the environment.
    let os: Table = g.get("os")?;
    let keep = lua.create_table()?;
    for name in ["time", "clock", "date", "difftime"] {
        keep.raw_set(name, os.get::<Value>(name)?)?;
    }
    g.raw_set("os", keep)?;
    // Text chunks only: bytecode can break out of the sandbox.
    let load = lua.create_function(
        |lua, (chunk, name, _mode, env): (Value, Option<String>, Value, Option<Table>)| {
            let src = match chunk {
                Value::String(s) => s.as_bytes().to_vec(),
                _ => {
                    return Ok((
                        Value::Nil,
                        Value::String(lua.create_string("load takes a string")?),
                    ));
                }
            };
            let mut c = lua
                .load(src)
                .set_mode(ChunkMode::Text)
                .set_name(name.unwrap_or_else(|| "=(load)".into()));
            if let Some(env) = env {
                c = c.set_environment(env);
            }
            match c.into_function() {
                Ok(f) => Ok((Value::Function(f), Value::Nil)),
                Err(e) => Ok((Value::Nil, Value::String(lua.create_string(e.to_string())?))),
            }
        },
    )?;
    g.raw_set("load", load)?;
    // `collectgarbage("stop")` would only run into the memory limit.
    let gc: Function = g.get("collectgarbage")?;
    let guarded = lua.create_function(move |_, args: MultiValue| {
        let opt = match args.front() {
            Some(Value::String(s)) => s.to_str().map(|s| s.to_owned()).unwrap_or_default(),
            _ => "collect".into(),
        };
        if !matches!(opt.as_str(), "collect" | "count" | "step" | "isrunning") {
            return Err(mlua::Error::runtime(format!(
                "collectgarbage(\"{opt}\") isn't available"
            )));
        }
        gc.call::<MultiValue>(args)
    })?;
    g.raw_set("collectgarbage", guarded)?;
    Ok(())
}

/// `require("a.b")` loads `a/b.lua` or `a/b/init.lua` from the plugin's folder, once.
fn install_require(lua: &Lua, dir: PathBuf) -> mlua::Result<()> {
    let loaded = lua.create_table()?;
    let require = lua.create_function(move |lua, name: String| {
        if let Some(v) = loaded.raw_get::<Option<Value>>(name.as_str())? {
            return Ok(v);
        }
        let path = module_path(&dir, &name)
            .ok_or_else(|| mlua::Error::runtime(format!("module '{name}' not found")))?;
        let src =
            std::fs::read(&path).map_err(|_| mlua::Error::runtime(format!("module '{name}' not found")))?;
        let rel = path
            .strip_prefix(&dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let v: Value = lua
            .load(src)
            .set_mode(ChunkMode::Text)
            .set_name(format!("@{rel}"))
            .call(name.as_str())?;
        let v = if v.is_nil() { Value::Boolean(true) } else { v };
        loaded.raw_set(name.as_str(), v.clone())?;
        Ok(v)
    })?;
    lua.globals().raw_set("require", require)
}

fn module_path(dir: &Path, name: &str) -> Option<PathBuf> {
    let ok = !name.is_empty()
        && name.len() <= 100
        && name
            .split('.')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    if !ok || name.split('.').next() == Some(DATA) {
        return None;
    }
    let base = dir.join(name.replace('.', "/"));
    [base.with_extension("lua"), base.join("init.lua")]
        .into_iter()
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua_in(dir: &Path) -> (Lua, Deadline) {
        new(dir).unwrap()
    }

    fn temp() -> PathBuf {
        let dir = crate::tests::temp_dir();
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::create_dir_all(dir.join(DATA)).unwrap();
        dir
    }

    #[test]
    fn dangerous_parts_are_gone() {
        let dir = temp();
        let (lua, _) = lua_in(&dir);
        let gone: bool = lua
            .load("return io == nil and debug == nil and dofile == nil and loadfile == nil and package == nil and string.dump == nil and os.execute == nil and os.getenv == nil and os.remove == nil and os.exit == nil")
            .eval()
            .unwrap();
        assert!(gone);
        assert!(
            lua.load("return os.time() > 0 and type(os.date('%Y')) == 'string'")
                .eval::<bool>()
                .unwrap()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_takes_text_only() {
        let dir = temp();
        let (lua, _) = lua_in(&dir);
        assert_eq!(
            lua.load("return load('return 1 + 1')()").eval::<i64>().unwrap(),
            2
        );
        // A bytecode header is refused.
        let refused: bool = lua
            .load("local f, err = load('\\27Lua') return f == nil and err ~= nil")
            .eval()
            .unwrap();
        assert!(refused);
        assert!(lua.load("collectgarbage('stop')").exec().is_err());
        assert!(lua.load("return collectgarbage('count')").eval::<f64>().unwrap() > 0.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn require_stays_in_the_folder() {
        let dir = temp();
        std::fs::write(
            dir.join("lib/util.lua"),
            "local M = {} M.n = (M.n or 0) + 1 function M.twice(x) return x * 2 end return M",
        )
        .unwrap();
        std::fs::write(dir.join("helper.lua"), "counter = (counter or 0) + 1").unwrap();
        std::fs::write(dir.join(DATA).join("evil.lua"), "return 'loaded'").unwrap();
        let (lua, _) = lua_in(&dir);
        assert_eq!(
            lua.load("return require('lib.util').twice(21)")
                .eval::<i64>()
                .unwrap(),
            42
        );
        // Loaded once.
        lua.load("require('helper') require('helper')").exec().unwrap();
        assert_eq!(lua.load("return counter").eval::<i64>().unwrap(), 1);
        for bad in ["data.evil", "../x", "lib/util", "", "nope"] {
            assert!(lua.load(format!("require('{bad}')")).exec().is_err(), "{bad}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn endless_loops_and_huge_strings_stop() {
        let dir = temp();
        let (lua, deadline) = lua_in(&dir);
        deadline.start(Duration::from_millis(50));
        let t = Instant::now();
        let err = lua.load("while true do end").exec().unwrap_err();
        assert!(err.to_string().contains("took too long"), "{err}");
        assert!(t.elapsed() < Duration::from_secs(2));
        // Coroutines are checked too.
        deadline.start(Duration::from_millis(50));
        assert!(
            lua.load("coroutine.wrap(function() while true do end end)()")
                .exec()
                .is_err()
        );
        deadline.clear();
        let err = lua
            .load("local t = {} for i = 1, 1e9 do t[i] = string.rep('x', 1000) .. i end")
            .exec()
            .unwrap_err();
        assert!(matches!(err, mlua::Error::MemoryError(_)), "{err}");
        // Still usable afterwards.
        lua.gc_collect().unwrap();
        assert_eq!(lua.load("return 1").eval::<i64>().unwrap(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
