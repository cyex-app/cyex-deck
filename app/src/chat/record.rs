//! X6 Chat recordings (D23): a chat's live events from the moment you press record, with
//! the emote, badge and GIF pictures they use saved next to them, so a replay works
//! offline. Folder per recording under `recordings\`: `meta.json`, `events.jsonl`
//! (`{"t": ms since start, "e": event}` per line), `assets.json` and the pictures.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_chat::model::{ChatMessage, Fragment, Provider, RoomEvent};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Semaphore;
use ts_rs::TS;

use super::{RoomKey, now_ms};

/// X6 Events sent to a replay at a time: it starts on the first part while the rest reads.
const PART: usize = 2_000;
/// Pictures bigger than this are skipped (moltorino's cap).
const MAX_ASSET: usize = 8 * 1024 * 1024;
const EVENT: &str = "chat:recording";
/// V107-R1 How long pictures may still come in after a recording stops.
const LATE_ASSETS: std::time::Duration = std::time::Duration::from_secs(120);
/// V110-F85 A recording stops by itself when its events file passes this.
const MAX_EVENTS_BYTES: u64 = 500 * 1024 * 1024;
/// V110-F85 Pictures downloaded at the same time, over all recordings.
static DOWNLOADS: Semaphore = Semaphore::const_new(4);
/// V110-F85 `assets.json` is written at most this often (the pictures in between wait).
const ASSETS_EVERY: Duration = Duration::from_secs(1);

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
    /// V110-F85 Bytes written to `events.jsonl`.
    bytes: u64,
}

#[derive(Default)]
pub struct Recorder {
    live: Mutex<HashMap<RoomKey, Rec>>,
    /// Asset key → file name, per recording id.
    assets: Mutex<HashMap<String, HashMap<String, String>>>,
    /// V110-F85 Recording ids whose `assets.json` write is already scheduled.
    flushing: Mutex<HashSet<String>>,
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
        bytes: 0,
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
    stop_for(app, key, None)
}

/// [`stop`], telling the UI why when it wasn't the user (`reason` rides on the same event).
fn stop_for(app: &AppHandle, key: &RoomKey, reason: Option<&str>) -> Option<RecordingMeta> {
    let mut rec = app
        .state::<Recorder>()
        .live
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(key)?;
    let _ = rec.out.flush();
    rec.meta.ended_ms = now_ms();
    let _ = write_meta(&rec.dir, &rec.meta);
    let _ = app.emit(EVENT, json!({ "channel": key.1, "recording": false, "reason": reason }));
    // V107-R1 Its asset list goes once the last pictures are in (it stayed for the run).
    let (app, id) = (app.clone(), rec.meta.id.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(LATE_ASSETS).await;
        let recorder = app.state::<Recorder>();
        let live = recorder.live.lock().unwrap_or_else(|p| p.into_inner());
        if !live.values().any(|r| r.meta.id == id) {
            // V110-F85 The last write, whatever the batching left waiting.
            if let Some(map) = recorder.assets.lock().unwrap_or_else(|p| p.into_inner()).remove(&id)
                && let Some(dir) = dir(&app)
            {
                write_assets(&dir.join(&id), &map);
            }
        }
    });
    Some(rec.meta)
}

fn write_meta(dir: &std::path::Path, meta: &RecordingMeta) -> Result<(), String> {
    let text = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    // Through a temp file: a crash halfway leaves the old meta, not a broken recording.
    let tmp = dir.join("meta.json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, dir.join("meta.json")).map_err(|e| e.to_string())
}

/// New events in a room: written down when it's being recorded.
pub fn events(app: &AppHandle, key: &RoomKey, events: &[RoomEvent]) {
    let mut wanted: Vec<(String, String, PathBuf, String)> = Vec::new();
    let mut full = false;
    {
        let recorder = app.state::<Recorder>();
        let mut live = recorder.live.lock().unwrap_or_else(|p| p.into_inner());
        let Some(rec) = live.get_mut(key) else { return };
        let t = now_ms() - rec.meta.started_ms;
        let before = rec.meta.messages;
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
            let line = json!({ "t": t, "e": e }).to_string();
            let _ = writeln!(rec.out, "{line}");
            rec.bytes += line.len() as u64 + 1;
            if over_cap(rec.bytes) {
                full = true;
                break;
            }
        }
        // Every 50 messages; a batch of only edits or deletes on a round count writes nothing.
        if rec.meta.messages / 50 != before / 50 {
            let _ = rec.out.flush();
            let _ = write_meta(&rec.dir, &rec.meta);
        }
    }
    if full {
        // V110-F85 The UI hears it on the same event as any stop, with the reason.
        eprintln!("chat recording of {} stopped: events.jsonl passed 500 MB", key.1);
        stop_for(app, key, Some("size"));
        return;
    }
    for (id, k, dir, url) in wanted {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { save_asset(&app, &id, &k, &dir, &url).await });
    }
}

/// V110-F85 The events file is as big as a recording may get.
fn over_cap(bytes: u64) -> bool {
    bytes >= MAX_EVENTS_BYTES
}

/// V110-F85 True for the first caller while `id` has no write scheduled.
fn claim_flush(pending: &mut HashSet<String>, id: &str) -> bool {
    pending.insert(id.to_owned())
}

fn write_assets(dir: &std::path::Path, map: &HashMap<String, String>) {
    if let Ok(text) = serde_json::to_string(map) {
        let _ = fs::write(dir.join("assets.json"), text);
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
    // V110-F85 A busy chat has many new emotes at once: four downloads at a time.
    let Ok(_slot) = DOWNLOADS.acquire().await else { return };
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
    let first = {
        let recorder = app.state::<Recorder>();
        let mut assets = recorder.assets.lock().unwrap_or_else(|p| p.into_inner());
        // Past `LATE_ASSETS` the list is gone: writing a new one would drop the others.
        let Some(map) = assets.get_mut(id) else { return };
        map.insert(key.to_owned(), file);
        claim_flush(&mut recorder.flushing.lock().unwrap_or_else(|p| p.into_inner()), id)
    };
    // V110-F85 One write per second for everything that arrived meanwhile, not one per picture.
    if first {
        let (app, id, dir) = (app.clone(), id.to_owned(), dir.to_owned());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(ASSETS_EVERY).await;
            let recorder = app.state::<Recorder>();
            recorder.flushing.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
            let map = recorder.assets.lock().unwrap_or_else(|p| p.into_inner()).get(&id).cloned();
            if let Some(map) = map {
                write_assets(&dir, &map);
            }
        });
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
    /// Asset key → file name, served at `<cyexhls>/rec/<id>/<file>`.
    pub assets: HashMap<String, String>,
}

/// One part of a recording's events: (ms since start, event).
pub type Part = Vec<(i64, Value)>;

/// A recording's meta and pictures, and its events file to `stream`. Nothing is left out:
/// the file is read in parts instead of whole, so a long one neither waits nor doubles in memory.
pub fn open(app: &AppHandle, id: &str) -> Result<(Recording, Option<File>), String> {
    if !valid_id(id) {
        return Err("No such recording.".into());
    }
    let d = dir(app).ok_or("No such recording.")?.join(id);
    let meta: RecordingMeta =
        serde_json::from_str(&fs::read_to_string(d.join("meta.json")).map_err(|_| "No such recording.")?)
            .map_err(|e| e.to_string())?;
    let assets = fs::read_to_string(d.join("assets.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    Ok((Recording { meta, assets }, File::open(d.join("events.jsonl")).ok()))
}

/// Reads the events in order, `PART` at a time, until `send` says the replay is gone.
pub fn stream(file: File, mut send: impl FnMut(Part) -> bool) {
    let mut part = Vec::with_capacity(PART);
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { break };
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let (Some(t), Some(e)) = (v["t"].as_i64(), v.get("e")) else { continue };
        part.push((t, e.clone()));
        if part.len() == PART && !send(std::mem::replace(&mut part, Vec::with_capacity(PART))) {
            return;
        }
    }
    // The last part, even an empty one: the replay knows it has everything.
    send(part);
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

    #[test]
    fn v110_f85_the_events_file_has_a_cap_and_asset_writes_are_batched() {
        assert!(!over_cap(MAX_EVENTS_BYTES - 1));
        assert!(over_cap(MAX_EVENTS_BYTES));
        let mut pending = HashSet::new();
        assert!(claim_flush(&mut pending, "a"));
        assert!(!claim_flush(&mut pending, "a"));
        assert!(claim_flush(&mut pending, "b"));
        pending.remove("a");
        assert!(claim_flush(&mut pending, "a"));
    }

    #[test]
    fn every_event_comes_in_order_in_parts_and_a_closed_replay_stops_the_read() {
        let path = std::env::temp_dir().join(format!("cyex-rec-stream-{}.jsonl", std::process::id()));
        let mut text = String::new();
        for t in 0..4_500 {
            text.push_str(&format!("{{\"t\":{t},\"e\":{{\"type\":\"wipe\"}}}}
"));
            if t == 10 {
                text.push_str("not json
");
            }
        }
        fs::write(&path, text).unwrap();
        let mut got: Vec<usize> = Vec::new();
        let mut ts: Vec<i64> = Vec::new();
        stream(File::open(&path).unwrap(), |p| {
            got.push(p.len());
            ts.extend(p.iter().map(|(t, _)| *t));
            true
        });
        assert_eq!(got, vec![PART, PART, 500]);
        assert_eq!(ts, (0..4_500).collect::<Vec<i64>>());
        let mut parts = 0;
        stream(File::open(&path).unwrap(), |_| {
            parts += 1;
            false
        });
        assert_eq!(parts, 1);
        let _ = fs::remove_file(&path);
    }
}
