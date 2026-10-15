//! V105-11 Uploads pasted or dropped images to the user's own host (Chatterino's image uploader). Headers and the upload URL are never logged or shown.

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
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
const BAD_TYPE: &str = "Only PNG, JPG, GIF and WebP images.";
const TOO_BIG: &str = "That image is over 20 MB.";

#[derive(Default)]
pub struct Uploads {
    /// Pasted image data waiting for its upload, oldest first (id, bytes, extension).
    staged: Mutex<Vec<(String, Vec<u8>, String)>>,
    busy: AtomicBool,
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

struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
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
            let len = fs::metadata(p).map_err(|_| "Can't read that file.")?.len();
            if len > MAX_BYTES as u64 {
                return Err(TOO_BIG.into());
            }
            let bytes = fs::read(p).map_err(|_| "Can't read that file.")?;
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
    if uploads.busy.swap(true, Ordering::SeqCst) {
        let _ = on.send(UploadEvent::Failed {
            message: "Wait for the upload to finish.".into(),
        });
        return;
    }
    let _busy = Busy(&uploads.busy);
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
    let extra = headers(&cfg.headers);
    let field = if cfg.form_field.is_empty() {
        "file"
    } else {
        cfg.form_field.as_str()
    };
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
            .map_err(|_| "Upload failed: can't reach the host.".to_string())?;
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
