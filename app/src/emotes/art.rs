//! V108-C10 Same badge art under different URLs. Drawing a person whose badges meet on one line
//! asks for prints of those pictures (`Cosmetics::look`); this task fetches them a few at a time
//! as a browser, lets Windows scale each to 8 x 8, and hands `Cosmetics` the URL → same-art map.
//! Prints are kept by URL in the local data folder, so a picture is fetched once. A failed fetch
//! is asked again after the next badge-set refresh. Nothing runs per message.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use cyex_emotes::badges::art::{Print, Prints, SIDE};
use cyex_twitch::BROWSER_AGENT;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use super::Emotes;

/// In the local data folder.
const FILE: &str = "badge-art.json";
/// Fetches at the same time.
const AT_ONCE: usize = 3;
const TIMEOUT: Duration = Duration::from_secs(10);
/// Moving badges run to a few hundred KB; past this a picture isn't compared.
const MAX_BYTES: usize = 1 << 20;
/// Asks gathered before a round, so a busy chat makes one round, not many.
const SETTLE: Duration = Duration::from_secs(2);

#[derive(Default)]
struct Art {
    prints: Mutex<Prints>,
    failed: Mutex<HashSet<String>>,
    wake: Notify,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn init(app: &AppHandle) {
    app.manage(Art::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        load(&app).await;
        loop {
            app.state::<Art>().wake.notified().await;
            tokio::time::sleep(SETTLE).await;
            round(&app).await;
        }
    });
}

/// After drawing: wakes the task when a picture waits for a print.
pub fn nudge(app: &AppHandle, waiting: bool) {
    if waiting && let Some(art) = app.try_state::<Art>() {
        art.wake.notify_one();
    }
}

/// A badge set was fetched again: pictures that failed may be asked for once more.
pub fn retry(app: &AppHandle) {
    let Some(art) = app.try_state::<Art>() else {
        return;
    };
    let failed = std::mem::take(&mut *lock(&art.failed));
    if !failed.is_empty() {
        app.state::<Emotes>().lock().cosmetics.art_forget(failed.iter().map(String::as_str));
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    crate::paths::local(app).ok().map(|d| d.join(FILE))
}

async fn load(app: &AppHandle) {
    let Some(file) = path(app) else {
        return;
    };
    let read = tauri::async_runtime::spawn_blocking(move || std::fs::read_to_string(file).ok()).await;
    let Ok(Some(text)) = read else {
        return;
    };
    let prints = Prints::from_json(&text);
    let map = prints.map();
    {
        let state = app.state::<Emotes>();
        let mut inner = state.lock();
        inner.cosmetics.art_known(prints.urls());
        inner.cosmetics.set_art(map);
    }
    *lock(&app.state::<Art>().prints) = prints;
}

/// Fetches and prints what waits, then hands over the new map.
async fn round(app: &AppHandle) {
    let art = app.state::<Art>();
    let asks: Vec<String> = {
        let asks = app.state::<Emotes>().lock().cosmetics.take_art_asks();
        let prints = lock(&art.prints);
        asks.into_iter().filter(|u| !prints.has(u)).collect()
    };
    if asks.is_empty() {
        return;
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let mut todo = asks.iter().cloned();
    let mut running = tokio::task::JoinSet::new();
    let mut got: HashMap<String, Option<Vec<u8>>> = HashMap::new();
    let mut failed = Vec::new();
    loop {
        while running.len() < AT_ONCE {
            let Some(url) = todo.next() else { break };
            let http = http.clone();
            running.spawn(async move {
                let r = http.get_small_as(&url, MAX_BYTES, BROWSER_AGENT, TIMEOUT).await;
                (url, r)
            });
        }
        let Some(done) = running.join_next().await else {
            break;
        };
        match done {
            Ok((url, Ok(bytes))) => {
                got.insert(url, bytes);
            }
            Ok((url, Err(_))) => failed.push(url),
            Err(_) => {}
        }
    }
    // Too big: no print, kept so it isn't fetched again.
    let order: Vec<String> = asks.into_iter().filter(|u| got.contains_key(u)).collect();
    let files: Vec<Vec<u8>> = order.iter().map(|u| got.remove(u).flatten().unwrap_or_default()).collect();
    let squares = tauri::async_runtime::spawn_blocking(move || crate::sys::thumb::squares(files, SIDE))
        .await
        .unwrap_or_default();
    let (map, text) = {
        let mut prints = lock(&art.prints);
        for (url, px) in order.iter().zip(squares) {
            prints.add(url, px.as_deref().and_then(Print::from_bgra));
        }
        (prints.map(), (!order.is_empty()).then(|| prints.to_json()))
    };
    if !failed.is_empty() {
        eprintln!("emotes: {} badge pictures didn't load for comparing", failed.len());
        lock(&art.failed).extend(failed);
    }
    if let (Some(text), Some(file)) = (text, path(app)) {
        let _ = tauri::async_runtime::spawn_blocking(move || save(&file, &text)).await;
    }
    let changed = app.state::<Emotes>().lock().cosmetics.set_art(map);
    if changed {
        let _ = app.emit(super::extra::EVENT, ());
    }
}

/// Written next to the file first, then swapped in, so a crash never leaves half a file.
fn save(file: &std::path::Path, text: &str) {
    let tmp = file.with_extension("json.tmp");
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, file).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}
