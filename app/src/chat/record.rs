//! X6 Chat recordings (D23): a chat's live events from the moment you press record, with
//! the emote, badge and GIF pictures they use saved next to them, so a replay works
//! offline. Folder per recording under `recordings\`: `meta.json`, `events.jsonl`
//! (`{"t": ms since start, "e": event}` per line), `assets.json` and the pictures.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cyex_chat::model::{ChatMessage, Fragment, Provider, RoomEvent};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::{RoomKey, now_ms};

/// Pictures bigger than this are skipped (moltorino's cap).
const MAX_ASSET: usize = 8 * 1024 * 1024;
const EVENT: &str = "chat:recording";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct RecordingMeta {
    pub id: String,
    pub channel: String,
    #[ts(type = "number")]
    pub started_ms: i64,
    #[ts(type = "number")]
    pub ended_ms: i64,
    pub messages: u32,
}

struct Rec {
    meta: RecordingMeta,
    dir: PathBuf,
    out: BufWriter<File>,
    /// Asset keys already saved or on their way.
    seen: HashSet<String>,
}

#[derive(Default)]
pub struct Recorder {
    live: Mutex<HashMap<RoomKey, Rec>>,
    /// Asset key → file name, per recording id.
    assets: Mutex<HashMap<String, HashMap<String, String>>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Recorder::default());
}

pub fn dir(app: &AppHandle) -> Option<PathBuf> {
    crate::paths::config(app).ok().map(|d| d.join("recordings"))
}

/// Recording ids are folder names: letters, digits, `-` and `_`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn is_recording(app: &AppHandle, key: &RoomKey) -> bool {
    app.state::<Recorder>()
        .live
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains_key(key)
}

pub fn start(app: &AppHandle, key: &RoomKey) -> Result<String, String> {
    let base = dir(app).ok_or("No folder for recordings.")?;
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let id = format!(
        "{}-{:04}{:02}{:02}-{:02}{:02}{:02}",
        key.1,
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    let dir = base.join(&id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = BufWriter::new(File::create(dir.join("events.jsonl")).map_err(|e| e.to_string())?);
    let meta = RecordingMeta {
        id: id.clone(),
        channel: key.1.clone(),
        started_ms: now_ms(),
        ..RecordingMeta::default()
    };
    write_meta(&dir, &meta)?;
    let rec = Rec {
        meta,
        dir,
        out,
        seen: HashSet::new(),
    };
    app.state::<Recorder>()
        .live
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(key.clone(), rec);
    app.state::<Recorder>()
        .assets
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(id.clone(), HashMap::new());
    let _ = app.emit(EVENT, json!({ "channel": key.1, "recording": true }));
    Ok(id)
}

pub fn stop(app: &AppHandle, key: &RoomKey) -> Option<RecordingMeta> {
    let mut rec = app
        .state::<Recorder>()
        .live
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(key)?;
    let _ = rec.out.flush();
    rec.meta.ended_ms = now_ms();
    let _ = write_meta(&rec.dir, &rec.meta);
    let _ = app.emit(EVENT, json!({ "channel": key.1, "recording": false }));
    Some(rec.meta)
}

fn write_meta(dir: &std::path::Path, meta: &RecordingMeta) -> Result<(), String> {
    let text = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(dir.join("meta.json"), text).map_err(|e| e.to_string())
}

/// New events in a room: written down when it's being recorded.
pub fn events(app: &AppHandle, key: &RoomKey, events: &[RoomEvent]) {
    let mut wanted: Vec<(String, String, PathBuf, String)> = Vec::new();
    {
        let recorder = app.state::<Recorder>();
        let mut live = recorder.live.lock().unwrap_or_else(|p| p.into_inner());
        let Some(rec) = live.get_mut(key) else { return };
        let t = now_ms() - rec.meta.started_ms;
        for e in events {
            let keep = matches!(
                e,
                RoomEvent::Message { .. }
                    | RoomEvent::Update { .. }
                    | RoomEvent::Delete { .. }
                    | RoomEvent::ClearUser { .. }
            );
            if !keep {
                continue;
            }
            if let RoomEvent::Message { message } = e {
                rec.meta.messages += 1;
                for (k, url) in assets_of(app, &key.1, message) {
                    if rec.seen.insert(k.clone()) {
                        wanted.push((rec.meta.id.clone(), k, rec.dir.clone(), url));
                    }
                }
            }
            let line = json!({ "t": t, "e": e });
            let _ = writeln!(rec.out, "{line}");
        }
        if rec.meta.messages % 50 == 0 {
            let _ = rec.out.flush();
            let _ = write_meta(&rec.dir, &rec.meta);
        }
    }
    for (id, k, dir, url) in wanted {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { save_asset(&app, &id, &k, &dir, &url).await });
    }
}

/// Pictures a message uses, as (key, URL). Keys match what the replay asks for.
fn assets_of(app: &AppHandle, channel: &str, m: &Arc<ChatMessage>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in &m.body {
        match f {
            Fragment::Emote {
                provider,
                id,
                stack,
                look,
                ..
            } => {
                out.push(emote_asset(*provider, id, look.animated.unwrap_or(false)));
                for l in stack.iter().flatten() {
                    out.push(emote_asset(l.provider, &l.id, l.look.animated.unwrap_or(false)));
                }
            }
            Fragment::Gif { id, url } => out.push((format!("gif:{id}"), url.replace(".gif?", ".webp?"))),
            _ => {}
        }
    }
    if let Some(s) = &m.sender {
        for b in &s.badges {
            if let Some(url) = super::catalog::badge_url(app, channel, &b.set, &b.version) {
                out.push((format!("badge:{}/{}", b.set, b.version), url));
            }
        }
    }
    out
}

fn emote_asset(provider: Provider, id: &str, animated: bool) -> (String, String) {
    let e: String = url::form_urlencoded::byte_serialize(id.as_bytes()).collect();
    let (name, url) = match provider {
        Provider::Twitch => (
            "twitch",
            format!("https://static-cdn.jtvnw.net/emoticons/v2/{e}/default/dark/2.0"),
        ),
        Provider::Seventv => ("seventv", format!("https://cdn.7tv.app/emote/{e}/2x.webp")),
        Provider::Bttv => ("bttv", format!("https://cdn.betterttv.net/emote/{e}/2x.webp")),
        Provider::Ffz => (
            "ffz",
            format!(
                "https://cdn.frankerfacez.com/emote/{e}/{}2",
                if animated { "animated/" } else { "" }
            ),
        ),
    };
    (format!("emote:{name}:{id}"), url)
}

async fn save_asset(app: &AppHandle, id: &str, key: &str, dir: &std::path::Path, url: &str) {
    let http = app.state::<crate::auth::Auth>().http.clone();
    let Ok(bytes) = http.get_bytes(url, MAX_ASSET).await else {
        return;
    };
    let ext = sniff(&bytes);
    let file = format!(
        "{}.{ext}",
        key.replace([':', '/'], "_")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || "_-.".contains(*c))
            .collect::<String>()
    );
    if fs::write(dir.join(&file), &bytes).is_err() {
        return;
    }
    let map = {
        let recorder = app.state::<Recorder>();
        let mut assets = recorder.assets.lock().unwrap_or_else(|p| p.into_inner());
        let map = assets.entry(id.to_owned()).or_default();
        map.insert(key.to_owned(), file);
        map.clone()
    };
    if let Ok(text) = serde_json::to_string(&map) {
        let _ = fs::write(dir.join("assets.json"), text);
    }
}

fn sniff(b: &[u8]) -> &'static str {
    match b {
        [0x89, b'P', b'N', b'G', ..] => "png",
        [b'G', b'I', b'F', ..] => "gif",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "webp",
        [0xFF, 0xD8, ..] => "jpg",
        _ => "img",
    }
}

/// Every recording, newest first.
pub fn list(app: &AppHandle) -> Vec<RecordingMeta> {
    let Some(base) = dir(app) else { return Vec::new() };
    let mut out: Vec<RecordingMeta> = fs::read_dir(base)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|d| serde_json::from_str(&fs::read_to_string(d.path().join("meta.json")).ok()?).ok())
        .collect();
    out.sort_by_key(|m| std::cmp::Reverse(m.started_ms));
    out
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Recording {
    pub meta: RecordingMeta,
    /// (ms since start, event).
    #[ts(type = "Array<[number, unknown]>")]
    pub events: Vec<(i64, Value)>,
    /// Asset key → file name, served at `<cyexhls>/rec/<id>/<file>`.
    pub assets: HashMap<String, String>,
}

pub fn load(app: &AppHandle, id: &str) -> Result<Recording, String> {
    if !valid_id(id) {
        return Err("No such recording.".into());
    }
    let d = dir(app).ok_or("No such recording.")?.join(id);
    let meta: RecordingMeta =
        serde_json::from_str(&fs::read_to_string(d.join("meta.json")).map_err(|_| "No such recording.")?)
            .map_err(|e| e.to_string())?;
    let events = fs::read_to_string(d.join("events.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            Some((v["t"].as_i64()?, v.get("e")?.clone()))
        })
        .collect();
    let assets = fs::read_to_string(d.join("assets.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    Ok(Recording { meta, events, assets })
}

pub fn delete(app: &AppHandle, id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("No such recording.".into());
    }
    let d = dir(app).ok_or("No such recording.")?.join(id);
    fs::remove_dir_all(d).map_err(|e| e.to_string())
}

/// A saved picture, for the `cyexhls` scheme.
pub fn asset(app: &AppHandle, id: &str, file: &str) -> Option<(Vec<u8>, &'static str)> {
    let file_ok = !file.is_empty()
        && file
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
        && !file.contains("..");
    if !valid_id(id) || !file_ok {
        return None;
    }
    let bytes = fs::read(dir(app)?.join(id).join(file)).ok()?;
    let kind = match sniff(&bytes) {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "jpg" => "image/jpeg",
        _ => "application/octet-stream",
    };
    Some((bytes, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_names() {
        assert!(valid_id("xqc-20261010-120000"));
        assert!(!valid_id("../x"));
        assert_eq!(
            emote_asset(Provider::Ffz, "1", true).1,
            "https://cdn.frankerfacez.com/emote/1/animated/2"
        );
        assert_eq!(sniff(b"GIF89a"), "gif");
    }
}
