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
/// V110-F85 Longest text and pattern the pattern functions take (see [`pattern_check`]).
const MAX_SUBJECT: usize = 64 << 10;
const MAX_PATTERN: usize = 512;
/// V110-F85 Longest text `string.rep` builds.
const MAX_REP: usize = 1 << 20;
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
    guard_strings(lua, &string)?;
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
                "collectgarbage(\"{opt}\") isn’t available"
            )));
        }
        gc.call::<MultiValue>(args)
    })?;
    g.raw_set("collectgarbage", guarded)?;
    Ok(())
}

/// V110-F85 The count hook can't stop one long C call, and a pattern with several `.-` or
/// `.*` on a long text backtracks for hours inside `string.find`. These refuse such calls
/// with a Lua error before they start, and cap what `string.rep` builds.
fn guard_strings(lua: &Lua, string: &Table) -> mlua::Result<()> {
    for name in ["find", "match", "gmatch", "gsub"] {
        let orig: Function = string.get(name)?;
        let guarded = lua.create_function(move |_, args: MultiValue| {
            // `find(s, p, init, true)` is a plain search: no pattern to backtrack.
            let plain = name == "find" && args.get(3).is_some_and(|v| !matches!(v, Value::Nil | Value::Boolean(false)));
            if !plain && let (Some(Value::String(s)), Some(Value::String(p))) = (args.front(), args.get(1)) {
                pattern_check(s.as_bytes().len(), &p.as_bytes(), name != "gmatch").map_err(mlua::Error::runtime)?;
            }
            orig.call::<MultiValue>(args)
        })?;
        string.raw_set(name, guarded)?;
    }
    let rep: Function = string.get("rep")?;
    let guarded = lua.create_function(move |_, args: MultiValue| {
        if let (Some(Value::String(s)), Some(n)) = (args.front(), args.get(1)) {
            let n = match n {
                Value::Integer(i) => *i as f64,
                Value::Number(f) => *f,
                _ => 0.0,
            };
            let sep = match args.get(2) {
                Some(Value::String(sep)) => sep.as_bytes().len(),
                _ => 0,
            };
            if rep_too_long(s.as_bytes().len(), sep, n) {
                return Err(mlua::Error::runtime("string.rep: the text would be over 1 MB"));
            }
        }
        rep.call::<MultiValue>(args)
    })?;
    string.raw_set("rep", guarded)
}

/// V110-F85 `string.rep(s, n, sep)` would be over [`MAX_REP`] bytes.
fn rep_too_long(len: usize, sep: usize, n: f64) -> bool {
    n >= 1.0 && (len as f64) * n + (sep as f64) * (n - 1.0) > MAX_REP as f64
}

/// V110-F85 Whether the pattern functions may run on a text of `subject` bytes with this
/// pattern. Backtracking grows with the power of the repetitions (`*`, `+`, `-`), so the
/// allowed text shrinks with their number: up to 1 repetition 64 KB, 2: 4 KB, 3: 512 bytes,
/// more than 3: refused. A leading `^` takes one repetition off.
fn pattern_check(subject: usize, pattern: &[u8], anchor_counts: bool) -> Result<(), &'static str> {
    if pattern.len() > MAX_PATTERN {
        return Err("the pattern is over 512 bytes");
    }
    // A `^` start spares the walk over every start position: one repetition less (not in gmatch).
    let anchored = usize::from(anchor_counts && pattern.first() == Some(&b'^'));
    let limit = match repetitions(pattern).saturating_sub(anchored) {
        0 | 1 => MAX_SUBJECT,
        2 => 4 << 10,
        3 => 512,
        _ => return Err("the pattern has too many repetitions (*, + or -); split it up"),
    };
    if subject > limit {
        return Err(if limit == MAX_SUBJECT {
            "the text is over 64 KB"
        } else {
            "the text is too long for a pattern with this many repetitions (*, + or -)"
        });
    }
    Ok(())
}

/// V110-F85 Repetition marks in a pattern, counted generously: `%`-escapes and the inside
/// of `[...]` sets don't count, a leading mark is a plain character.
fn repetitions(p: &[u8]) -> usize {
    let (mut i, mut n) = (0, 0);
    while i < p.len() {
        match p[i] {
            b'%' => i += if p.get(i + 1) == Some(&b'b') { 4 } else { 2 },
            b'[' => {
                i += 1;
                if p.get(i) == Some(&b'^') {
                    i += 1;
                }
                // The first character of a set is always part of it, even a `]`.
                i += 1;
                while i < p.len() && p[i] != b']' {
                    i += if p[i] == b'%' { 2 } else { 1 };
                }
                i += 1;
            }
            b'*' | b'+' | b'-' => {
                n += usize::from(i > 0);
                i += 1;
            }
            _ => i += 1,
        }
    }
    n
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
    fn v110_f85_a_catastrophic_pattern_is_refused_before_it_runs() {
        let dir = temp();
        let (lua, _) = lua_in(&dir);
        let t = Instant::now();
        for code in [
            "return string.find(string.rep('a', 60000), '.-.-.-.-.-b')",
            "return (string.rep('a', 60000)):match('.-.-.-.-b')",
            "return string.gsub(string.rep('a', 5000), '.-.-b', 'x')",
            "for _ in string.gmatch(string.rep('a', 60000), '(.-)(.-)(.-)b') do end",
            "return string.find('abc', string.rep('a', 600))",
            "return string.find(string.rep('a', 70000), 'b')",
            "return string.rep('abc', 1000000)",
            "return string.rep('a', 2e6)",
            "return ('x'):rep(600000, 'yy')",
        ] {
            let err = lua.load(code).exec().unwrap_err();
            assert!(!err.to_string().contains("took too long"), "{code}");
        }
        assert!(t.elapsed() < Duration::from_secs(2));
        // Everyday patterns and sizes still work.
        let ok: bool = lua
            .load(
                "local s = string.rep('a', 60000) .. 'b'
                 return s:find('b', 1, true) == 60001
                   and s:find('a-b') == 1
                   and ('key=value'):match('^(%w+)=(.-)$') == 'key'
                   and ('a,b,c'):gsub(',', ';') == 'a;b;c'
                   and ('x[y]'):find('[%]]') == 4
                   and #string.rep('ab', 1000, ',') == 2999
                   and string.rep('x', 0) == ''
                   and string.rep('x', -3) == ''
                   and #string.rep('a', 1000000) == 1000000",
            )
            .eval()
            .unwrap();
        assert!(ok);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn v110_f85_pattern_check_tiers() {
        assert_eq!(repetitions(b"%s*=%s*(.-)%s*$"), 4);
        assert_eq!(repetitions(b"[a-z]+"), 1);
        assert_eq!(repetitions(b"[]-]x%-"), 0);
        assert_eq!(repetitions(b"-x*"), 1);
        assert!(pattern_check(64 << 10, b"%d+", true).is_ok());
        assert!(pattern_check((64 << 10) + 1, b"x", true).is_err());
        assert!(pattern_check(4 << 10, b"(%w+)=(%w+)", true).is_ok());
        assert!(pattern_check((4 << 10) + 1, b"(%w+)=(%w+)", true).is_err());
        assert!(pattern_check(512, b".-.-.-b", true).is_ok());
        assert!(pattern_check(513, b".-.-.-b", true).is_err());
        assert!(pattern_check(5, b".-.-.-.-b", true).is_err());
        assert!(pattern_check(5, &[b'a'; 513], true).is_err());
        assert!(pattern_check(512, b"^.-.-.-b", true).is_ok());
        assert!(pattern_check(5, b"^.-.-.-.-b", true).is_ok());
        assert!(pattern_check(5, b"^.-.-.-.-.-b", true).is_err());
        assert!(pattern_check(5, b"^.-.-.-.-b", false).is_err());
        assert!(rep_too_long(1, 0, 1_048_577.0));
        assert!(!rep_too_long(1, 0, 1_048_576.0));
        assert!(!rep_too_long(5, 5, 0.0));
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
