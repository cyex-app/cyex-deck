//! S7, S10 `cyex.Message.new`, `cyex.LinkType`, `cyex.Image` and `cyex.ImageSet`: lines a
//! plugin builds from pieces, in Chatterino's element format, for `Channel:add_message`.
//! Plugin lines always carry the plugin's mark and never a sender or badges. Pictures load
//! straight from the web (https, the plugin's websites), at most 112 px high.

use mlua::{Lua, Table, Value};

use super::names;
use crate::convert::color;
use crate::host::{LineLink, NewLine, Piece, PieceColor, PieceStyle};
use crate::perm::{Granted, check_url};
use crate::state;

const MAX_ELEMENTS: usize = 50;
const MAX_TEXT: usize = 1000;
const MAX_TIP: usize = 200;
const MAX_LINK: usize = 2000;
const MAX_IMAGES: usize = 20;
const MAX_SIDE: u16 = 512;
const LINK_TYPES: [&str; 7] =
    ["Url", "UserInfo", "UserAction", "JumpToChannel", "CopyToClipboard", "JumpToMessage", "InsertText"];

pub fn install(lua: &Lua, cyex: &Table) -> mlua::Result<()> {
    let message = lua.create_table()?;
    message.set(
        "new",
        lua.create_function(|lua, init: Table| {
            let granted = state::with(lua, |s| s.granted.clone())?;
            read(&init, &granted).map_err(mlua::Error::runtime)?;
            init.set("__message", true)?;
            Ok(init)
        })?,
    )?;
    cyex.set("Message", message)?;
    cyex.set("LinkType", names(lua, &LINK_TYPES)?)?;
    let image = lua.create_table()?;
    image.set(
        "from_url",
        lua.create_function(|lua, (url, scale, size): (String, Option<f64>, Option<Table>)| {
            let t = lua.create_table()?;
            t.set("__image", true)?;
            t.set("url", picture_url(lua, &url)?)?;
            let scale = scale.filter(|s| *s > 0.0).unwrap_or(1.0);
            t.set("scale", scale)?;
            if let Some(size) = size {
                let (w, h): (f64, f64) = (size.get(1)?, size.get(2)?);
                t.set("width", w * scale)?;
                t.set("height", h * scale)?;
                t.set("size", size)?;
            }
            Ok(t)
        })?,
    )?;
    image.set(
        "empty",
        lua.create_function(|lua, ()| {
            let t = lua.create_table()?;
            t.set("__image", true)?;
            t.set("empty", true)?;
            Ok(t)
        })?,
    )?;
    cyex.set("Image", image)?;
    let set = lua.create_table()?;
    set.set(
        "new",
        lua.create_function(|lua, list: mlua::Variadic<Value>| {
            let t = lua.create_table()?;
            t.set("__imageset", true)?;
            for (i, v) in list.into_iter().take(3).enumerate() {
                let img = match v {
                    Value::String(s) => {
                        let one = lua.create_table()?;
                        one.set("__image", true)?;
                        one.set("url", picture_url(lua, &s.to_str()?)?)?;
                        one.set("scale", 1.0 / [1.0, 2.0, 4.0][i])?;
                        Value::Table(one)
                    }
                    Value::Table(t) if t.get::<bool>("__image").unwrap_or(false) => Value::Table(t),
                    Value::Nil => continue,
                    _ => return Err(mlua::Error::runtime("an ImageSet takes images or https addresses")),
                };
                t.set(i + 1, img)?;
            }
            Ok(t)
        })?,
    )?;
    cyex.set("ImageSet", set)?;
    Ok(())
}

/// A picture's address: Network, https only (the chat only shows those), a listed host.
fn picture_url(lua: &Lua, url: &str) -> mlua::Result<String> {
    let hosts = state::with(lua, |s| s.granted.network.clone())?
        .ok_or_else(|| mlua::Error::runtime("pictures need the Network permission"))?;
    if url.len() > MAX_LINK {
        return Err(mlua::Error::runtime("the address is too long"));
    }
    let u = check_url(url, &hosts).map_err(mlua::Error::runtime)?;
    if u.scheme() != "https" {
        return Err(mlua::Error::runtime("pictures need an https address"));
    }
    Ok(u.into())
}

/// A message table as a line. Errors say what's wrong in a few words.
pub fn read(t: &Table, granted: &Granted) -> Result<NewLine, String> {
    let get = |k: &str| t.get::<Value>(k).map_err(|e| e.to_string());
    let id = match get("id")? {
        Value::Nil => String::new(),
        Value::String(s) => {
            let s = s.to_str().map_err(|e| e.to_string())?.to_owned();
            let ok = s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-:".contains(c));
            if !ok {
                return Err("id takes letters, digits, . _ - and :, up to 64".into());
            }
            s
        }
        _ => return Err("id must be a string".into()),
    };
    let time = match get("time")? {
        Value::Nil => None,
        Value::Integer(n) => Some(n),
        Value::Number(n) => Some(n as i64),
        _ => return Err("time must be unix milliseconds".into()),
    };
    let highlight = match get("highlight_color")?.as_string().map(|s| s.to_string_lossy()) {
        Some(c) => Some(color(&c).ok_or("highlight_color must be a #rrggbb color")?),
        None => matches!(get("highlight")?, Value::Boolean(true)).then(String::new),
    };
    let elements: Table = t.get("elements").map_err(|_| "elements must be a list")?;
    let mut pieces = Vec::new();
    let (mut chars, mut images) = (0, 0);
    for e in elements.sequence_values::<Table>() {
        let e = e.map_err(|_| "each element must be a table")?;
        if pieces.len() >= MAX_ELEMENTS {
            return Err(format!("up to {MAX_ELEMENTS} elements"));
        }
        let kind: String = e.get("type").map_err(|_| "an element needs a type")?;
        let tip = text_of(&e, "tooltip", MAX_TIP)?;
        let piece = match kind.as_str() {
            "text" | "single-line-text" => {
                let text: String = e.get("text").map_err(|_| "a text element needs text")?;
                chars += text.chars().count();
                Piece::Text {
                    color: piece_color(&e)?,
                    style: match text_of(&e, "style", 20)?.as_deref() {
                        None => None,
                        Some("bold") => Some(PieceStyle::Bold),
                        Some("italic") => Some(PieceStyle::Italic),
                        Some("button") => Some(PieceStyle::Button),
                        Some(other) => return Err(format!("style \"{other}\" isn’t bold, italic or button")),
                    },
                    parse: e.get::<Option<bool>>("parse").map_err(|e| e.to_string())?.unwrap_or(false),
                    link: link(&e, granted)?,
                    tip,
                    text,
                }
            }
            "mention" => {
                let text: String = e.get("display_name").map_err(|_| "a mention needs display_name")?;
                let login: String = e.get("login_name").map_err(|_| "a mention needs login_name")?;
                chars += text.chars().count();
                Piece::Mention {
                    text,
                    login: login.trim_start_matches('@').to_ascii_lowercase(),
                }
            }
            "linebreak" => Piece::Break,
            "image" | "circular-image" | "scaling-image" => {
                let urls = if kind == "scaling-image" {
                    let set: Table = e.get("images").map_err(|_| "a scaling-image needs images (an ImageSet)")?;
                    let mut urls = Vec::new();
                    for img in set.sequence_values::<Table>() {
                        urls.extend(image(&img.map_err(|e| e.to_string())?)?);
                    }
                    urls
                } else {
                    let img: Table = e.get("image").map_err(|_| "an image element needs an image")?;
                    image(&img)?.into_iter().collect()
                };
                let Some(first) = urls.first() else { continue };
                images += 1;
                Piece::Image {
                    size: first.2,
                    urls: urls.into_iter().map(|(u, d, _)| (u, d)).collect(),
                    round: kind == "circular-image",
                    link: link(&e, granted)?,
                    tip,
                }
            }
            "timestamp" | "twitch-moderation" | "reply-curve" => return Err(format!("{kind} isn’t in cyex")),
            other => return Err(format!("unknown element type \"{other}\"")),
        };
        pieces.push(piece);
    }
    if chars > MAX_TEXT {
        return Err(format!("up to {MAX_TEXT} characters a line"));
    }
    if images > MAX_IMAGES {
        return Err(format!("up to {MAX_IMAGES} pictures a line"));
    }
    if pieces.is_empty() {
        return Err("the line has no elements".into());
    }
    Ok(NewLine {
        id,
        time,
        highlight,
        pieces,
    })
}

fn text_of(e: &Table, k: &str, max: usize) -> Result<Option<String>, String> {
    let v: Option<String> = e.get(k).map_err(|_| format!("{k} must be a string"))?;
    match v {
        Some(s) if s.chars().count() > max => Err(format!("{k} is over {max} characters")),
        other => Ok(other.filter(|s| !s.is_empty())),
    }
}

fn piece_color(e: &Table) -> Result<Option<PieceColor>, String> {
    Ok(match text_of(e, "color", 20)?.as_deref() {
        None | Some("text") => None,
        Some("link") => Some(PieceColor::Link),
        Some("system") => Some(PieceColor::System),
        Some(c) => Some(PieceColor::Rgb(color(c).ok_or_else(|| format!("color \"{c}\" isn’t a #rrggbb color"))?)),
    })
}

/// `{ type = cyex.LinkType.X, value = "…" }`.
fn link(e: &Table, granted: &Granted) -> Result<Option<LineLink>, String> {
    let Some(l) = e.get::<Option<Table>>("link").map_err(|_| "link must be a table")? else {
        return Ok(None);
    };
    let kind: String = l.get("type").map_err(|_| "a link needs a type")?;
    let value: String = l.get("value").map_err(|_| "a link needs a value")?;
    if value.chars().count() > MAX_LINK {
        return Err(format!("a link’s value is over {MAX_LINK} characters"));
    }
    Ok(Some(match kind.as_str() {
        "Url" => {
            let lower = value.to_ascii_lowercase();
            if !(lower.starts_with("https://") || lower.starts_with("http://")) {
                return Err("a Url link needs an http or https address".into());
            }
            LineLink::Url(value)
        }
        "UserInfo" => LineLink::User(value.trim_start_matches('@').to_ascii_lowercase()),
        "UserAction" => {
            // Text that isn't a command would go out as a chat message.
            if !value.starts_with('/') && !granted.send {
                return Err("a UserAction that sends a message needs the SendMessages permission".into());
            }
            LineLink::Run(value)
        }
        "CopyToClipboard" => LineLink::Copy(value),
        "JumpToMessage" => LineLink::Jump(value),
        "InsertText" => LineLink::Insert(value),
        "JumpToChannel" => return Err("JumpToChannel isn’t in cyex".into()),
        other => return Err(format!("unknown link type \"{other}\"")),
    }))
}

/// A picture: its address, how many times sharper than 1x it is, and its size at 1x.
type Picture = (String, u8, Option<(u16, u16)>);

/// An `Image` table as a [`Picture`]; `None` for `Image.empty()`.
fn image(t: &Table) -> Result<Option<Picture>, String> {
    if t.get::<bool>("empty").unwrap_or(false) {
        return Ok(None);
    }
    if !t.get::<bool>("__image").unwrap_or(false) {
        return Err("use cyex.Image.from_url for pictures".into());
    }
    let url: String = t.get("url").map_err(|e| e.to_string())?;
    let scale: f64 = t.get::<Option<f64>>("scale").map_err(|e| e.to_string())?.unwrap_or(1.0);
    let density = (1.0 / scale).round().clamp(1.0, 4.0) as u8;
    let size = match t.get::<Option<Table>>("size").map_err(|e| e.to_string())? {
        Some(s) => {
            let side = |i: usize| -> Result<u16, String> {
                let v: f64 = s.get(i).map_err(|_| "size is { width, height }")?;
                Ok((v * scale).round().clamp(1.0, f64::from(MAX_SIDE)) as u16)
            };
            Some((side(1)?, side(2)?))
        }
        None => None,
    };
    Ok(Some((url, density, size)))
}
