//! S5 `cyex.data`: files in the plugin's own `data` folder (FilesystemRead and
//! FilesystemWrite). Flat names only, so nothing reaches outside it.

use std::io::Write;
use std::path::PathBuf;

use mlua::{Lua, Table};

use crate::manifest::DATA;
use crate::perm::data_name;
use crate::state;

const MAX_FILE: u64 = 10 << 20;
const MAX_FILES: usize = 1000;

fn path(lua: &Lua, name: &str, write: bool) -> mlua::Result<PathBuf> {
    let name = data_name(name).map_err(mlua::Error::runtime)?;
    state::with(lua, |s| {
        let ok = if write {
            s.granted.fs_write
        } else {
            s.granted.fs_read
        };
        if !ok {
            let p = if write {
                "FilesystemWrite"
            } else {
                "FilesystemRead"
            };
            return Err(mlua::Error::runtime(format!("that needs the {p} permission")));
        }
        Ok(s.dir.join(DATA).join(name))
    })?
}

fn err(e: std::io::Error) -> mlua::Error {
    mlua::Error::runtime(e.to_string())
}

pub fn table(lua: &Lua) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    // The file's text, or nil when there's none.
    t.set(
        "read",
        lua.create_function(|lua, name: String| {
            let p = path(lua, &name, false)?;
            match std::fs::metadata(&p) {
                Ok(m) if m.len() > MAX_FILE => return Err(mlua::Error::runtime("the file is over 10 MB")),
                Ok(_) => {}
                Err(_) => return Ok(None),
            }
            let bytes = std::fs::read(&p).map_err(err)?;
            Ok(Some(lua.create_string(bytes)?))
        })?,
    )?;
    t.set(
        "write",
        lua.create_function(|lua, (name, text): (String, mlua::LuaString)| {
            let p = path(lua, &name, true)?;
            let bytes = text.as_bytes();
            if bytes.len() as u64 > MAX_FILE {
                return Err(mlua::Error::runtime("up to 10 MB per file"));
            }
            room_for(&p)?;
            std::fs::write(&p, &*bytes).map_err(err)
        })?,
    )?;
    t.set(
        "append",
        lua.create_function(|lua, (name, text): (String, mlua::LuaString)| {
            let p = path(lua, &name, true)?;
            let bytes = text.as_bytes();
            let now = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            if now + bytes.len() as u64 > MAX_FILE {
                return Err(mlua::Error::runtime("up to 10 MB per file"));
            }
            room_for(&p)?;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&p)
                .map_err(err)?;
            f.write_all(&bytes).map_err(err)
        })?,
    )?;
    t.set(
        "remove",
        lua.create_function(|lua, name: String| {
            let p = path(lua, &name, true)?;
            Ok(std::fs::remove_file(p).is_ok())
        })?,
    )?;
    t.set(
        "list",
        lua.create_function(|lua, ()| {
            let dir = state::with(lua, |s| {
                if s.granted.fs_read {
                    Ok(s.dir.join(DATA))
                } else {
                    Err(mlua::Error::runtime("that needs the FilesystemRead permission"))
                }
            })??;
            let mut names: Vec<String> = std::fs::read_dir(dir)
                .map(|it| {
                    it.filter_map(Result::ok)
                        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                        .filter_map(|e| e.file_name().into_string().ok())
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            lua.create_sequence_from(names)
        })?,
    )?;
    Ok(t)
}

/// A new file only while the folder holds fewer than [`MAX_FILES`].
fn room_for(p: &std::path::Path) -> mlua::Result<()> {
    if p.exists() {
        return Ok(());
    }
    let count = p
        .parent()
        .and_then(|d| std::fs::read_dir(d).ok())
        .map_or(0, Iterator::count);
    if count >= MAX_FILES {
        return Err(mlua::Error::runtime(format!("up to {MAX_FILES} files")));
    }
    Ok(())
}
