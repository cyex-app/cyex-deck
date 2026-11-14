//! V111-14 The last good answer of each emote-set call on disk, as Chatterino keeps its
//! provider caches (`<id>.<provider>` in its Cache folder): a join draws the kept sets at once
//! and asks the provider again in the background; the fresh answer takes their place, and
//! when the call fails (7TV slow or down) the kept sets stay. Pictures need nothing here:
//! the providers' CDNs send `immutable, max-age=31536000`, so the WebView's own disk cache
//! draws a seen emote without asking again.
//!
//! Files: `%LOCALAPPDATA%\cyex-deck\emote-cache\<provider>-<what>[-<id>].json`, each
//! `{ "v": 1, "at": <unix s>, "body": <the answer> }`. Older than [`MAX_AGE`] is not used;
//! past [`MAX_TOTAL`] the least recently written go; a file that doesn't read is removed.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cyex_chat::model::Provider;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use super::providers::Which;

/// Shape of a kept file; another number is a file from another version, not used.
const VERSION: u32 = 1;
/// A kept answer older than this is not drawn (a month: Chatterino keeps its 14 days by
/// file date but rewrites on every good answer, so a channel seen weekly never expires).
pub const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// One answer bigger than this isn't kept (7TV's biggest channel sets are well under 1 MB).
pub const MAX_FILE: u64 = 4 * 1024 * 1024;
/// The whole folder: past this the least recently written files go.
pub const MAX_TOTAL: u64 = 64 * 1024 * 1024;
/// The first clean-up waits this long after start, then runs once a day.
const TRIM_AFTER: Duration = Duration::from_secs(60);
const TRIM_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
const DIR: &str = "emote-cache";

/// What a kept answer is of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// A provider's global sets.
    Global(Provider),
    /// A channel's sets (7TV account and set, BTTV user, FFZ room), by Twitch id.
    Channel(Provider, String),
    /// C9 A 7TV set by id (personal and event sets).
    Set(String),
}

impl Key {
    /// The file's name; `None` for an id that isn't plain letters, digits, `-` or `_`.
    pub fn file(&self) -> Option<String> {
        let plain = |id: &str| !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        match self {
            Key::Global(p) => Some(format!("{}-global.json", tag(*p)?)),
            Key::Channel(p, id) => plain(id).then(|| Some(format!("{}-channel-{id}.json", tag(*p)?))).flatten(),
            Key::Set(id) => plain(id).then(|| format!("7tv-set-{id}.json")),
        }
    }
}

fn tag(p: Provider) -> Option<&'static str> {
    match p {
        Provider::Seventv => Some("7tv"),
        Provider::Bttv => Some("bttv"),
        Provider::Ffz => Some("ffz"),
        Provider::Twitch => None,
    }
}

#[derive(Serialize, Deserialize)]
struct Entry {
    v: u32,
    at: u64,
    body: Value,
}

/// What to keep after a call: the answer, or `None` to drop the kept one (the channel has
/// no account there any more).
pub type Save = (Key, Option<Value>);

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// A kept answer written at `at` is drawn at `now`: not older than [`MAX_AGE`], and not
/// more than a day ahead (a clock set back would keep it forever).
pub fn usable(at: u64, now: u64) -> bool {
    at <= now.saturating_add(24 * 60 * 60) && now.saturating_sub(at) <= MAX_AGE.as_secs()
}

pub fn dir(app: &AppHandle) -> Option<PathBuf> {
    crate::paths::local(app).ok().map(|d| d.join(DIR))
}

/// The kept answer for `key`, if there is a usable one. A file that doesn't read (cut short,
/// another shape) is removed.
pub fn read(dir: &Path, key: &Key, now: u64) -> Option<Value> {
    let path = dir.join(key.file()?);
    let meta = fs::metadata(&path).ok()?;
    if meta.len() > MAX_FILE {
        let _ = fs::remove_file(&path);
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    match serde_json::from_slice::<Entry>(&bytes) {
        Ok(e) if e.v == VERSION => usable(e.at, now).then_some(e.body),
        _ => {
            let _ = fs::remove_file(&path);
            None
        }
    }
}

/// Keeps `body` for `key` (through a temp file, so a crash never leaves half a file), or
/// drops the kept one for `None`. Errors are only logged: the cache is a nicety.
pub fn write(dir: &Path, key: &Key, body: Option<&Value>, now: u64) {
    let Some(name) = key.file() else { return };
    let path = dir.join(&name);
    let Some(body) = body else {
        let _ = fs::remove_file(&path);
        return;
    };
    let Ok(bytes) = serde_json::to_vec(&Entry { v: VERSION, at: now, body: body.clone() }) else {
        return;
    };
    if bytes.len() as u64 > MAX_FILE {
        return;
    }
    let tmp = dir.join(format!("{name}.tmp"));
    let done = fs::create_dir_all(dir)
        .and_then(|()| fs::write(&tmp, &bytes))
        .and_then(|()| fs::rename(&tmp, &path));
    if let Err(e) = done {
        let _ = fs::remove_file(&tmp);
        eprintln!("emotes: couldn't keep {name}: {e}");
    }
}

/// Drops files past [`MAX_AGE`] (by file date), leftover temp files, and, past `cap`
/// bytes, the least recently written. What it removed, in bytes.
pub fn trim(dir: &Path, now: SystemTime, cap: u64) -> u64 {
    let Ok(list) = fs::read_dir(dir) else { return 0 };
    let mut kept: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    let mut freed = 0;
    for e in list.flatten() {
        let path = e.path();
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let written = meta.modified().unwrap_or(UNIX_EPOCH);
        let old = now.duration_since(written).is_ok_and(|d| d > MAX_AGE);
        let tmp = path.extension().is_some_and(|x| x == "tmp");
        if old || tmp || path.extension().is_none_or(|x| x != "json") {
            if fs::remove_file(&path).is_ok() {
                freed += meta.len();
            }
            continue;
        }
        kept.push((written, meta.len(), path));
    }
    kept.sort_by_key(|k| std::cmp::Reverse(k.0));
    let mut total = 0u64;
    for (_, len, path) in kept {
        total += len;
        if total > cap && fs::remove_file(&path).is_ok() {
            freed += len;
        }
    }
    freed
}

/// Reads kept answers off the async threads.
pub async fn read_many(app: &AppHandle, keys: Vec<Key>) -> Vec<(Key, Value)> {
    let Some(dir) = dir(app) else { return Vec::new() };
    tauri::async_runtime::spawn_blocking(move || {
        let now = now();
        keys.into_iter().filter_map(|k| read(&dir, &k, now).map(|v| (k, v))).collect()
    })
    .await
    .unwrap_or_default()
}

/// Writes (or drops) kept answers in the background.
pub fn save(app: &AppHandle, saves: Vec<Save>) {
    if saves.is_empty() {
        return;
    }
    let Some(dir) = dir(app) else { return };
    tauri::async_runtime::spawn_blocking(move || {
        let now = now();
        for (k, v) in &saves {
            write(&dir, k, v.as_ref(), now);
        }
    });
}

/// The clean-up: a minute after start, then once a day.
pub fn init(app: &AppHandle) {
    let Some(dir) = dir(app) else { return };
    tauri::async_runtime::spawn(async move {
        let mut wait = TRIM_AFTER;
        loop {
            tokio::time::sleep(wait).await;
            let d = dir.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || trim(&d, SystemTime::now(), MAX_TOTAL)).await;
            wait = TRIM_EVERY;
        }
    });
}

/// V111-14 Providers whose call failed while their kept sets are drawn: a quiet row each in
/// About › Connection health, nothing in chat.
pub fn note_fallbacks(kept: Which, failed: Which) {
    let both = kept.minus(failed.not());
    for (on, label) in [(both.seventv, "7TV emotes"), (both.bttv, "BTTV emotes"), (both.ffz, "FFZ emotes")] {
        if on {
            cyex_core::health::note(label, "slow or down, saved emotes in use");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cyex-v111-14-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    const DAY: u64 = 24 * 60 * 60;

    #[test]
    fn v111_14_keys_make_plain_file_names() {
        assert_eq!(Key::Global(Provider::Seventv).file().as_deref(), Some("7tv-global.json"));
        assert_eq!(Key::Channel(Provider::Ffz, "71092938".into()).file().as_deref(), Some("ffz-channel-71092938.json"));
        assert_eq!(Key::Set("01HKQT8".into()).file().as_deref(), Some("7tv-set-01HKQT8.json"));
        // Nothing that could leave the folder or isn't a provider.
        assert_eq!(Key::Channel(Provider::Bttv, "../x".into()).file(), None);
        assert_eq!(Key::Set(String::new()).file(), None);
        assert_eq!(Key::Global(Provider::Twitch).file(), None);
    }

    #[test]
    fn v111_14_write_then_read_and_drop() {
        let d = temp("rw");
        let k = Key::Channel(Provider::Seventv, "1".into());
        let body = json!({ "data": { "users": { "userByConnection": null } } });
        write(&d, &k, Some(&body), 1_000);
        assert_eq!(read(&d, &k, 1_000 + DAY), Some(body));
        assert!(!d.join("7tv-channel-1.json.tmp").exists());
        // `None` drops it (no account any more).
        write(&d, &k, None, 2_000);
        assert_eq!(read(&d, &k, 2_000), None);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn v111_14_old_answers_are_not_drawn() {
        let d = temp("age");
        let k = Key::Global(Provider::Bttv);
        write(&d, &k, Some(&json!([])), 1_000);
        let month = MAX_AGE.as_secs();
        assert!(read(&d, &k, 1_000 + month).is_some());
        assert!(read(&d, &k, 1_000 + month + 1).is_none());
        // A file from the future (clock set back) isn't trusted past a day.
        assert!(usable(1_000 + DAY, 1_000));
        assert!(!usable(1_000 + DAY + 1, 1_000));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn v111_14_a_broken_file_is_ignored_and_removed() {
        let d = temp("bad");
        let k = Key::Global(Provider::Ffz);
        let path = d.join(k.file().unwrap());
        fs::write(&path, b"{\"v\":1,\"at\":5,\"bo").unwrap();
        assert_eq!(read(&d, &k, 5), None);
        assert!(!path.exists());
        // Another version's shape isn't used either.
        fs::write(&path, br#"{"v":99,"at":5,"body":[]}"#).unwrap();
        assert_eq!(read(&d, &k, 5), None);
        // Nor one too big to be an answer.
        let f = fs::File::create(&path).unwrap();
        f.set_len(MAX_FILE + 1).unwrap();
        drop(f);
        assert_eq!(read(&d, &k, 5), None);
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn v111_14_trim_keeps_the_newest_under_the_cap() {
        let d = temp("trim");
        let body = json!({ "x": "a".repeat(1_000) });
        for i in 0..5 {
            write(&d, &Key::Set(format!("s{i}")), Some(&body), 1);
            let t = SystemTime::now() - Duration::from_secs(100 - i * 10);
            fs::File::options().write(true).open(d.join(format!("7tv-set-s{i}.json"))).unwrap().set_modified(t).unwrap();
        }
        fs::write(d.join("x.json.tmp"), b"half").unwrap();
        let one = fs::metadata(d.join("7tv-set-s0.json")).unwrap().len();
        // Room for two: the two written last stay.
        trim(&d, SystemTime::now(), one * 2);
        let left: Vec<bool> = (0..5).map(|i| d.join(format!("7tv-set-s{i}.json")).exists()).collect();
        assert_eq!(left, [false, false, false, true, true]);
        assert!(!d.join("x.json.tmp").exists());
        // Past the age limit by file date: gone whatever the cap.
        let later = SystemTime::now() + MAX_AGE + Duration::from_secs(1);
        trim(&d, later, u64::MAX);
        assert!(fs::read_dir(&d).unwrap().next().is_none());
        let _ = fs::remove_dir_all(&d);
    }
}
