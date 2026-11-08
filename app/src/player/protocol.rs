//! `cyexhls://`: what hls.js loads. Twitch's playlist and VOD servers refuse the app's own
//! origin, so they're fetched in Rust and served from here. Paths:
//! `live/<id>/master.m3u8`, `live/<id>/audio.m3u8`, `live/<id>/<variant>.m3u8`,
//! `live/<id>/replay.m3u8`, `live/<id>/replay-audio.m3u8` (V108-N24),
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
    let origin = req.headers().get("Origin").and_then(|v| v.to_str().ok()).and_then(allowed_origin).map(str::to_owned);
    tauri::async_runtime::spawn(async move {
        let mut res = match route(&app, &path).await {
            Ok(r) => r,
            Err(e) => failure(&e),
        };
        // V110-F60 hls.js asks from the app's page, which is another origin than this scheme's:
        // only that page may read the reply.
        if let Some(o) = origin.and_then(|o| o.parse().ok()) {
            res.headers_mut().insert("Access-Control-Allow-Origin", o);
            res.headers_mut().insert("Vary", tauri::http::HeaderValue::from_static("Origin"));
        }
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
        #[cfg(feature = "mocks")]
        (Kind::Live(live), "live", ["seg", rest @ ..]) => {
            let kind = if rest.last().is_some_and(|f| f.ends_with(".mp4")) { "video/mp4" } else { "video/mp2t" };
            Ok(response(200, kind, live.mock_segment(&rest.join("/")).await?))
        }
        (Kind::Live(live), "live", ["replay.m3u8"]) => Ok(playlist(live.replay(false).await?)),
        (Kind::Live(live), "live", ["replay-audio.m3u8"]) => Ok(playlist(live.replay(true).await?)),
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

/// V110-F60 The origins the app's own page has: the built page on Windows (`tauri.localhost`,
/// http or https), elsewhere `tauri://localhost`, and the dev server.
fn allowed_origin(origin: &str) -> Option<&str> {
    const OWN: [&str; 4] = ["http://tauri.localhost", "https://tauri.localhost", "tauri://localhost", "http://localhost:5173"];
    OWN.contains(&origin).then_some(origin)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v110_f60_only_the_apps_own_page_may_read_replies() {
        assert_eq!(allowed_origin("http://tauri.localhost"), Some("http://tauri.localhost"));
        assert_eq!(allowed_origin("https://example.com"), None);
        assert_eq!(allowed_origin("http://tauri.localhost.evil.com"), None);
        assert_eq!(allowed_origin("null"), None);
    }
}
