//! Usher, Twitch's playlist service (1.1): master playlists for live streams and VODs,
//! signed by a playback token, plus plain fetches of what they point to. Twitch's servers
//! refuse the app's own origin, so playlists (and VOD segments) go through Rust.

use serde::Deserialize;
use url::Url;

use crate::gql::AccessToken;
use crate::random::hex;
use crate::{Error, Http, Result};

const LIVE: &str = "https://usher.ttvnw.net/api/v2/channel/hls/";
const VOD: &str = "https://usher.ttvnw.net/vod/v2/";

/// `codecs` is usher's `supported_codecs`, e.g. `h264` or `av1,h265,h264`.
pub fn live_url(login: &str, token: &AccessToken, codecs: &str) -> String {
    let mut url = Url::parse(&format!("{LIVE}{login}.m3u8")).expect("valid base");
    url.query_pairs_mut()
        .append_pair("allow_source", "true")
        .append_pair("allow_audio_only", "true")
        .append_pair("fast_bread", "true")
        .append_pair("p", &random_p())
        .append_pair("platform", "web")
        .append_pair("playlist_include_framerate", "true")
        .append_pair("supported_codecs", codecs)
        .append_pair("sig", &token.signature)
        .append_pair("token", &token.value);
    url.into()
}

pub fn vod_url(id: &str, token: &AccessToken, codecs: &str) -> String {
    let mut url = Url::parse(&format!("{VOD}{id}.m3u8")).expect("valid base");
    url.query_pairs_mut()
        .append_pair("allow_source", "true")
        .append_pair("allow_audio_only", "true")
        .append_pair("player_backend", "mediaplayer")
        .append_pair("playlist_include_framerate", "true")
        .append_pair("supported_codecs", codecs)
        .append_pair("platform", "web")
        .append_pair("p", &random_p())
        .append_pair("nauthsig", &token.signature)
        .append_pair("nauth", &token.value);
    url.into()
}

fn random_p() -> String {
    (u32::from_str_radix(&hex(4), 16).unwrap_or(0) % 10_000_000).to_string()
}

/// A playlist. Usher's refusals come back as errors carrying its `error_code`, e.g.
/// `content_geoblocked`, or its text, e.g. `Can not find channel` (offline).
pub async fn text(http: &Http, url: &str) -> Result<String> {
    let body = get(http, url).await?;
    String::from_utf8(body).map_err(|_| Error::Api {
        status: 200,
        message: "not text".into(),
    })
}

/// A VOD segment.
pub async fn bytes(http: &Http, url: &str) -> Result<Vec<u8>> {
    get(http, url).await
}

async fn get(http: &Http, url: &str) -> Result<Vec<u8>> {
    let res = http.0.get(url).send().await.map_err(Error::Network)?;
    let status = res.status();
    let body = res.bytes().await.map_err(Error::Network)?;
    if status.is_success() {
        return Ok(body.to_vec());
    }
    Err(usher_error(status.as_u16(), &body))
}

/// Usher sends `[{"error": "...", "error_code": "..."}]`.
fn usher_error(status: u16, body: &[u8]) -> Error {
    #[derive(Deserialize)]
    struct Item {
        error: Option<String>,
        error_code: Option<String>,
    }
    match serde_json::from_slice::<Vec<Item>>(body).ok().and_then(|v| v.into_iter().next()) {
        Some(item) => Error::Api {
            status,
            message: item.error_code.or(item.error).unwrap_or_default(),
        },
        None => Error::api(status, body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> AccessToken {
        AccessToken {
            value: r#"{"channel":"xqc","expires":1}"#.into(),
            signature: "ab12".into(),
        }
    }

    #[test]
    fn urls_carry_the_signed_token() {
        let url = Url::parse(&live_url("xqc", &token(), "h264")).unwrap();
        assert_eq!(url.path(), "/api/v2/channel/hls/xqc.m3u8");
        let q: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(q.contains(&("token".into(), r#"{"channel":"xqc","expires":1}"#.into())));
        assert!(q.contains(&("sig".into(), "ab12".into())));
        assert!(q.contains(&("supported_codecs".into(), "h264".into())));
        let vod = Url::parse(&vod_url("123", &token(), "h264")).unwrap();
        assert_eq!(vod.path(), "/vod/v2/123.m3u8");
        assert!(vod.query_pairs().any(|(k, v)| k == "nauthsig" && v == "ab12"));
    }

    #[test]
    fn usher_errors() {
        let e = usher_error(404, br#"[{"error":"Can not find channel"}]"#);
        assert_eq!(e.api_message(), Some("Can not find channel"));
        let e = usher_error(
            403,
            br#"[{"type":"error","error":"Content Restricted In Region","error_code":"content_geoblocked"}]"#,
        );
        assert_eq!(e.api_message(), Some("content_geoblocked"));
        assert_eq!(usher_error(502, b"Bad Gateway").api_message(), Some("Bad Gateway"));
    }
}
