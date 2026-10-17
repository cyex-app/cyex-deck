//! `cyexhls://`: what hls.js loads. Twitch's playlist and VOD servers refuse the app's own
//! origin, so they're fetched in Rust and served from here. Paths:
//! `live/<id>/master.m3u8`, `live/<id>/audio.m3u8`, `live/<id>/<variant>.m3u8`,
//! `vod/<id>/master.m3u8`, `vod/<id>/audio.m3u8`, `vod/<id>/<variant>/index.m3u8`,
//! `vod/<id>/<variant>/<segment>`. Also the pictures of chat recordings (X6):
//! `rec/<id>/<file>`.

use cyex_twitch::Error;
use tauri::http::{Request, Response};
use tauri::{AppHandle, UriSchemeContext, UriSchemeResponder, Wry};

use super::Kind;

pub const SCHEME: &str = "cyexhls";

/// Where the webview finds the scheme: Windows maps custom schemes to `http://<scheme>.localhost`.
pub fn base() -> String {
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/")
    } else {
        format!("{SCHEME}://localhost/")
    }
}

pub fn handle(ctx: UriSchemeContext<'_, Wry>, req: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = ctx.app_handle().clone();
    let path = req.uri().path().to_owned();
    tauri::async_runtime::spawn(async move {
        let res = match route(&app, &path).await {
            Ok(r) => r,
            Err(e) => failure(&e),
        };
        responder.respond(res);
    });
}

async fn route(app: &AppHandle, path: &str) -> Result<Response<Vec<u8>>, Error> {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    if let ["rec", id, file] = parts.as_slice() {
        let (bytes, kind) = crate::chat::record::asset(app, id, file).ok_or_else(missing)?;
        return Ok(response(200, kind, bytes));
    }
    let (kind, id) = match parts.as_slice() {
        [kind, id, ..] => (*kind, *id),
        _ => return Err(missing()),
    };
    match (super::session(app, id).ok_or_else(missing)?, kind, &parts[2..]) {
        (Kind::Live(live), "live", ["master.m3u8"]) => Ok(playlist(live.master().await?)),
        (Kind::Live(live), "live", ["audio.m3u8"]) => Ok(playlist(live.audio_master().await?)),
        (Kind::Vod(vod), "vod", ["audio.m3u8"]) => Ok(playlist(vod.audio_master().await?)),
        (Kind::Live(live), "live", ["seg", rest @ ..]) => {
            let kind = if rest.last().is_some_and(|f| f.ends_with(".mp4")) { "video/mp4" } else { "video/mp2t" };
            Ok(response(200, kind, live.mock_segment(&rest.join("/")).await?))
        }
        (Kind::Live(live), "live", [file]) => {
            let index = file.strip_suffix(".m3u8").and_then(|n| n.parse().ok()).ok_or_else(missing)?;
            let (text, ad) = live.media(index).await?;
            super::set_ad(app, id, ad);
            Ok(playlist(text))
        }
        (Kind::Vod(vod), "vod", ["master.m3u8"]) => Ok(playlist(vod.master().await?)),
        (Kind::Vod(vod), "vod", [index, file]) => {
            let index: usize = index.parse().map_err(|_| missing())?;
            if *file == "index.m3u8" {
                Ok(playlist(vod.media(index).await?))
            } else {
                let kind = if file.ends_with(".mp4") { "video/mp4" } else { "video/mp2t" };
                Ok(response(200, kind, vod.segment(index, file).await?))
            }
        }
        _ => Err(missing()),
    }
}

fn playlist(text: String) -> Response<Vec<u8>> {
    response(200, "application/vnd.apple.mpegurl", text.into_bytes())
}

/// The player shows the reason: usher's code, e.g. `Can not find channel` when offline.
fn failure(e: &Error) -> Response<Vec<u8>> {
    let (status, text) = match e {
        Error::Api { status, message } => (*status, message.clone()),
        Error::Network(_) => (502, "network".to_owned()),
        Error::Decode(_) => (502, "unexpected answer".to_owned()),
        Error::Refused(code) => (403, code.clone()),
    };
    response(status, "text/plain", text.into_bytes())
}

fn response(status: u16, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_default()
}

fn missing() -> Error {
    Error::Api {
        status: 404,
        message: "gone".into(),
    }
}
