//! V105-11 Uploads pasted or dropped images to the user's own host (Chatterino's image uploader). Headers and the upload URL are never logged or shown.

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use crate::busy::Busy;
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::{Uploader, random_id};
use cyex_twitch::uploader::{error_text, fill, headers, mime, multipart};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::auth::Auth;
use crate::settings::Settings;

const MAX_BYTES: usize = 20 * 1024 * 1024;
const MAX_STAGED: usize = 10;
const MAX_LOG: usize = 500;
/// V1051-05 Dropped files remembered for previews, and a preview's longest side.
const MAX_DROPPED: usize = 20;
const PREVIEW_SIDE: u32 = 96;
const BAD_TYPE: &str = "Only PNG, JPG, GIF and WebP images.";
const TOO_BIG: &str = "That image is over 20 MB.";

#[derive(Default)]
pub struct Uploads {
    /// Pasted image data waiting for its upload, oldest first (id, bytes, extension).
    staged: Mutex<Vec<(String, Vec<u8>, String)>>,
    /// V1051-05 Image files dropped on the window, newest last: the only paths a preview may read.
    dropped: Mutex<Vec<String>>,
    busy: AtomicBool,
    /// Links (and delete links) of this session's uploads: the only ones the chat line may open.
    links: Mutex<Vec<String>>,
}

pub fn init(app: &AppHandle) {
    app.manage(Uploads::default());
}

#[derive(Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum UploadEvent {
    Started { index: u32, total: u32 },
    Done { link: String },
    Failed { message: String },
    Finished,
}

#[derive(Deserialize, TS)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
#[ts(export)]
pub enum UploadItem {
    File(String),
    Staged(String),
}

/// Keeps pasted image data until `run` picks it up. Returns its id.
pub fn stage(state: &Uploads, bytes: Vec<u8>, ext: &str) -> Result<String, String> {
    let ext = ext.to_ascii_lowercase();
    mime(&ext).ok_or(BAD_TYPE)?;
    if bytes.len() > MAX_BYTES {
        return Err(TOO_BIG.into());
    }
    let id = random_id();
    let mut staged = state.staged.lock().unwrap_or_else(|e| e.into_inner());
    staged.push((id.clone(), bytes, ext));
    if staged.len() > MAX_STAGED {
        staged.remove(0);
    }
    Ok(id)
}

/// V1051-05 The window saw these files dropped on it. Only image types are kept, and only
/// the latest few.
pub fn dropped(state: &Uploads, paths: &[std::path::PathBuf]) {
    let mut kept = state.dropped.lock().unwrap_or_else(|e| e.into_inner());
    for p in paths {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if mime(ext).is_none() {
            continue;
        }
        let Some(text) = p.to_str() else { continue };
        kept.retain(|k| k != text);
        kept.push(text.to_owned());
    }
    let extra = kept.len().saturating_sub(MAX_DROPPED);
    kept.drain(..extra);
}

/// V1051-05 A small PNG of a dropped file, for the upload question. Only a file this window
/// saw dropped, within the upload size cap.
pub fn preview(state: &Uploads, path: &str) -> Result<Vec<u8>, String> {
    let known = state.dropped.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|k| k == path);
    if !known {
        return Err("Not a file you dropped.".into());
    }
    let len = fs::metadata(path).map_err(|_| "Couldn’t open that file.")?.len();
    if len > MAX_BYTES as u64 {
        return Err(TOO_BIG.into());
    }
    let bytes = fs::read(path).map_err(|_| "Couldn’t open that file.")?;
    crate::sys::thumb::thumbnail(bytes, PREVIEW_SIDE).ok_or_else(|| "Couldn’t read that image.".into())
}

/// An image ready to send: bytes, the file name to announce, the mime type and where it came from.
struct Image {
    bytes: Vec<u8>,
    name: String,
    mime: &'static str,
    file: String,
}

fn load(app: &AppHandle, item: &UploadItem) -> Result<Image, String> {
    match item {
        UploadItem::Staged(id) => {
            let state = app.state::<Uploads>();
            let mut staged = state.staged.lock().unwrap_or_else(|e| e.into_inner());
            let at = staged
                .iter()
                .position(|(i, ..)| i == id)
                .ok_or("That image is gone. Paste it again.")?;
            let (_, bytes, ext) = staged.remove(at);
            let mime = mime(&ext).ok_or(BAD_TYPE)?;
            Ok(Image {
                bytes,
                name: format!("control_v.{ext}"),
                mime,
                file: String::new(),
            })
        }
        UploadItem::File(path) => {
            let p = Path::new(path);
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            let mime = mime(ext).ok_or(BAD_TYPE)?;
            let len = fs::metadata(p).map_err(|_| "Couldn’t open that file.")?.len();
            if len > MAX_BYTES as u64 {
                return Err(TOO_BIG.into());
            }
            let bytes = fs::read(p).map_err(|_| "Couldn’t open that file.")?;
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("image")
                .to_string();
            Ok(Image {
                bytes,
                name,
                mime,
                file: path.clone(),
            })
        }
    }
}

pub async fn run(app: AppHandle, channel: String, items: Vec<UploadItem>, on: Channel<UploadEvent>) {
    let uploads = app.state::<Uploads>();
    let Some(_busy) = Busy::take(&uploads.busy) else {
        let _ = on.send(UploadEvent::Failed {
            message: "Wait for the upload to finish.".into(),
        });
        return;
    };
    if let Err(message) = send_all(&app, &channel, &items, &on).await {
        let _ = on.send(UploadEvent::Failed { message });
    }
    let _ = on.send(UploadEvent::Finished);
}

async fn send_all(
    app: &AppHandle,
    channel: &str,
    items: &[UploadItem],
    on: &Channel<UploadEvent>,
) -> Result<(), String> {
    let cfg = app.state::<Settings>().read::<Uploader>(Partition::Uploader);
    if !cfg.enabled || cfg.url.is_empty() {
        return Err("Set up the uploader in Settings → Image uploads.".into());
    }
    let http = app.state::<Auth>().http.clone();
    let extra = headers(&app.state::<Settings>().uploader_headers());
    let field = if cfg.form_field.is_empty() {
        "file"
    } else {
        cfg.form_field.as_str()
    };
    let uploads = app.state::<Uploads>();
    let total = items.len() as u32;
    for (n, item) in items.iter().enumerate() {
        let _ = on.send(UploadEvent::Started {
            index: n as u32 + 1,
            total,
        });
        let img = load(app, item)?;
        let (content_type, body) = multipart(field, &img.name, img.mime, &img.bytes);
        let (status, answer) = http
            .post_body(&cfg.url, &extra, &content_type, body, 64 * 1024)
            .await
            .map_err(|_| "Upload failed: can’t reach the host.".to_string())?;
        if !(200..300).contains(&status) {
            return Err(format!("Upload failed: {}", error_text(status, &answer)));
        }
        let link = fill(&cfg.link, &answer);
        if link.is_empty() {
            return Err("Upload failed: no link in the answer.".into());
        }
        let deletion = if cfg.deletion_link.is_empty() {
            String::new()
        } else {
            fill(&cfg.deletion_link, &answer)
        };
        {
            let mut links = uploads.links.lock().unwrap_or_else(|e| e.into_inner());
            links.push(link.clone());
            if !deletion.is_empty() {
                links.push(deletion.clone());
            }
        }
        let _ = on.send(UploadEvent::Done { link: link.clone() });
        let tail = if deletion.is_empty() {
            String::new()
        } else {
            format!(" · delete: {deletion}")
        };
        crate::chat::system_line(app, channel, &format!("Uploaded {link}{tail}"));
        log(
            app,
            json!({ "channel": channel, "link": link, "deletion": deletion, "file": img.file, "time": now_s() }),
        );
        if n + 1 < items.len() {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    Ok(())
}

/// True when `url` is a web address that equals a link or delete link we recorded:
/// this session's list or `uploads.json`. Nothing else may be opened from a chat line.
pub fn may_open(url: &str, session: &[String], logged: &[Value]) -> bool {
    let web = url::Url::parse(url).is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some());
    web && (session.iter().any(|l| l == url)
        || logged
            .iter()
            .any(|e| ["link", "deletion"].iter().any(|k| e.get(k).and_then(Value::as_str) == Some(url))))
}

/// V105-11 Opens an upload link or delete link in the browser, if we recorded it.
pub fn open_link(app: &AppHandle, url: &str) -> Result<(), String> {
    let session = app.state::<Uploads>().links.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let logged = crate::paths::local(app)
        .ok()
        .and_then(|d| fs::read(d.join("uploads.json")).ok())
        .and_then(|b| serde_json::from_slice::<Vec<Value>>(&b).ok())
        .unwrap_or_default();
    if !may_open(url, &session, &logged) {
        return Err("That link can’t be opened.".into());
    }
    tauri_plugin_opener::OpenerExt::opener(app)
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Appends to `<local>\uploads.json` (newest last, at most 500). Never headers or the URL.
fn log(app: &AppHandle, entry: Value) {
    let Ok(dir) = crate::paths::local(app) else { return };
    let path = dir.join("uploads.json");
    let mut list: Vec<Value> = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    list.push(entry);
    if list.len() > MAX_LOG {
        list.drain(..list.len() - MAX_LOG);
    }
    let Ok(text) = serde_json::to_vec_pretty(&list) else {
        return;
    };
    let tmp = dir.join("uploads.json.tmp");
    if fs::create_dir_all(&dir).is_ok() && fs::write(&tmp, text).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("cyex-upload-preview-test");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn a_preview_only_reads_dropped_images() {
        let state = Uploads::default();
        let img = temp("seen.png");
        let other = temp("unseen.png");
        let text = temp("note.txt");
        for p in [&img, &other, &text] {
            fs::write(p, b"x").unwrap();
        }
        dropped(&state, &[img.clone(), text.clone()]);
        // Never dropped: refused before the file is touched.
        assert_eq!(preview(&state, other.to_str().unwrap()).unwrap_err(), "Not a file you dropped.");
        // Dropped but not an image type: not remembered.
        assert_eq!(preview(&state, text.to_str().unwrap()).unwrap_err(), "Not a file you dropped.");
        assert_eq!(preview(&state, r"C:\Windows\win.ini").unwrap_err(), "Not a file you dropped.");
        // A remembered one is read (here it is not a real image).
        assert_eq!(preview(&state, img.to_str().unwrap()).unwrap_err(), "Couldn’t read that image.");
    }

    #[test]
    fn a_preview_stops_at_the_size_cap() {
        let state = Uploads::default();
        let big = temp("big.png");
        fs::File::create(&big).unwrap().set_len(MAX_BYTES as u64 + 1).unwrap();
        dropped(&state, std::slice::from_ref(&big));
        assert_eq!(preview(&state, big.to_str().unwrap()).unwrap_err(), TOO_BIG);
        let _ = fs::remove_file(big);
    }

    #[test]
    fn only_the_latest_drops_are_kept() {
        let state = Uploads::default();
        let all: Vec<_> = (0..MAX_DROPPED + 5).map(|i| std::path::PathBuf::from(format!("a{i}.png"))).collect();
        dropped(&state, &all);
        let kept = state.dropped.lock().unwrap();
        assert_eq!(kept.len(), MAX_DROPPED);
        assert_eq!(kept[0], "a5.png");
    }

    #[test]
    fn only_recorded_web_links_open() {
        let session = vec!["https://i.example/a.png".to_string()];
        let logged = vec![json!({ "link": "http://127.0.0.1:1/b.png", "deletion": "https://i.example/del/9" })];
        assert!(may_open("https://i.example/a.png", &session, &logged));
        assert!(may_open("http://127.0.0.1:1/b.png", &session, &logged));
        assert!(may_open("https://i.example/del/9", &session, &logged));
        assert!(!may_open("https://i.example/other", &session, &logged));
        assert!(!may_open("https://i.example/a.png?x=1", &session, &logged));
        assert!(!may_open("", &session, &logged));
    }

    #[test]
    fn recorded_non_web_links_still_refused() {
        let logged = vec![json!({ "link": "file:///C:/Windows/system32/calc.exe", "deletion": "javascript:alert(1)" })];
        assert!(!may_open("file:///C:/Windows/system32/calc.exe", &[], &logged));
        assert!(!may_open("javascript:alert(1)", &["javascript:alert(1)".to_string()], &logged));
    }
}
