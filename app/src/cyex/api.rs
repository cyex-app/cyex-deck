//! V109-B02 The calls to api.cyex.app an account makes. Errors come as `{ error, message }`;
//! nothing here prints a token or a URL.

use cyex_twitch::{Call, Http, Reply, Verb};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::model::{CyexEquipped, CyexHistory, CyexMe};

const MAX: usize = 1 << 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    /// No answer, or the server had a problem (5xx).
    Offline,
    /// The server said no: its code (`signed_out`, `not_owned`, …) and message.
    Said { status: u16, code: String, message: String },
}

impl ApiError {
    pub fn code(&self) -> &str {
        match self {
            ApiError::Offline => "",
            ApiError::Said { code, .. } => code,
        }
    }

    /// The session token is gone or ran out.
    pub fn signed_out(&self) -> bool {
        matches!(self, ApiError::Said { status: 401, code, .. } if code == "signed_out" || code.is_empty())
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ErrorBody {
    error: String,
    message: String,
}

/// Turns a reply into the answer or the server's error.
pub(super) fn read<T: DeserializeOwned>(reply: &Reply) -> Result<T, ApiError> {
    if (200..300).contains(&reply.status) {
        let body: &[u8] = if reply.body.is_empty() { b"null" } else { &reply.body };
        return serde_json::from_slice(body).map_err(|_| ApiError::Offline);
    }
    if reply.status >= 500 {
        return Err(ApiError::Offline);
    }
    let e: ErrorBody = serde_json::from_slice(&reply.body).unwrap_or_default();
    Err(ApiError::Said {
        status: reply.status,
        code: e.error.chars().take(40).collect(),
        message: e.message.chars().filter(|c| !c.is_control()).take(200).collect(),
    })
}

async fn send<T: DeserializeOwned>(
    http: &Http,
    verb: Verb,
    url: String,
    bearer: Option<&str>,
    body: Option<&Value>,
) -> Result<T, ApiError> {
    let call = Call {
        bearer,
        json: body,
        max: MAX,
        ..Call::default()
    };
    let reply = http.call(verb, &url, call).await.map_err(|_| ApiError::Offline)?;
    read(&reply)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_token: String,
    #[serde(default)]
    pub expires_at: String,
}

pub async fn session(http: &Http, base: &str, access: &str) -> Result<Session, ApiError> {
    let body = json!({ "accessToken": access });
    send(http, Verb::Post, format!("{base}/app/v1/session"), None, Some(&body)).await
}

pub async fn end(http: &Http, base: &str, token: &str) {
    let _ = send::<Value>(http, Verb::Delete, format!("{base}/app/v1/session"), Some(token), None).await;
}

pub async fn me(http: &Http, base: &str, token: &str) -> Result<CyexMe, ApiError> {
    send(http, Verb::Get, format!("{base}/me"), Some(token), None).await
}

#[derive(Deserialize)]
struct EquipAnswer {
    equipped: CyexEquipped,
}

pub async fn equip(http: &Http, base: &str, token: &str, main: Option<&str>, custom: bool) -> Result<CyexEquipped, ApiError> {
    let body = json!({ "main": main, "custom": custom });
    let a: EquipAnswer = send(http, Verb::Put, format!("{base}/me/equipped"), Some(token), Some(&body)).await?;
    Ok(a.equipped)
}

#[derive(Deserialize)]
struct RedeemAnswer {
    #[serde(default)]
    reward: Value,
}

pub async fn redeem(http: &Http, base: &str, token: &str, code: &str) -> Result<Value, ApiError> {
    let body = json!({ "code": code });
    let a: RedeemAnswer = send(http, Verb::Post, format!("{base}/promo/redeem"), Some(token), Some(&body)).await?;
    Ok(a.reward)
}

/// V109-B06 The Supporter+ uploads (app session works there).
pub async fn history(http: &Http, base: &str, token: &str) -> Result<CyexHistory, ApiError> {
    send(http, Verb::Get, format!("{base}/plus/badges"), Some(token), None).await
}

/// V109-B07 Owner previews are small PNGs.
const PREVIEW_MAX: usize = 256 << 10;
const PNG_SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// V109-B07 An owner preview of the account's own upload, and nothing else:
/// `{base}/me/badge/<id>/<file>.png`.
pub fn preview_ok(base: &str, url: &str) -> bool {
    let Some(rest) = url.strip_prefix(base.trim_end_matches('/')).and_then(|r| r.strip_prefix("/me/badge/")) else {
        return false;
    };
    let mut parts = rest.split('/');
    let (Some(id), Some(file), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let id_ok = (1..=12).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit());
    let file_ok = file
        .strip_suffix(".png")
        .is_some_and(|f| (1..=8).contains(&f.len()) && f.bytes().all(|b| b.is_ascii_alphanumeric()));
    id_ok && file_ok
}

/// A PNG answer, or the server's error.
fn png(reply: &Reply) -> Result<Vec<u8>, ApiError> {
    if (200..300).contains(&reply.status) {
        return if reply.body.starts_with(&PNG_SIG) {
            Ok(reply.body.clone())
        } else {
            Err(ApiError::Offline)
        };
    }
    Err(read::<Value>(reply).err().unwrap_or(ApiError::Offline))
}

/// V109-B07 The picture behind an owner preview URL (checked with `preview_ok` first).
pub async fn preview(http: &Http, url: &str, token: &str) -> Result<Vec<u8>, ApiError> {
    let call = Call {
        bearer: Some(token),
        max: PREVIEW_MAX,
        ..Call::default()
    };
    let reply = http.call(Verb::Get, url, call).await.map_err(|_| ApiError::Offline)?;
    png(&reply)
}

/// V109-B06 Puts an earlier approved upload back live.
pub async fn restore(http: &Http, base: &str, token: &str, id: u64) -> Result<(), ApiError> {
    send::<Value>(http, Verb::Post, format!("{base}/plus/badges/{id}/restore"), Some(token), None)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(status: u16, body: &str) -> Reply {
        Reply {
            status,
            etag: None,
            body: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn errors_carry_the_servers_code() {
        let e = read::<Value>(&reply(409, r#"{"error":"not_owned","message":"You don't own that."}"#)).unwrap_err();
        assert_eq!(e.code(), "not_owned");
        assert!(!e.signed_out());
        let out = read::<Value>(&reply(401, r#"{"error":"signed_out","message":"x"}"#)).unwrap_err();
        assert!(out.signed_out());
        assert!(!read::<Value>(&reply(401, r#"{"error":"bad_token"}"#)).unwrap_err().signed_out());
        assert_eq!(read::<Value>(&reply(502, "")).unwrap_err(), ApiError::Offline);
        assert_eq!(read::<Value>(&reply(400, "<html>")).unwrap_err().code(), "");
        assert_eq!(read::<Value>(&reply(204, "")).unwrap(), Value::Null);
    }

    #[test]
    fn me_reads_with_missing_parts() {
        let me: CyexMe = read(&reply(
            200,
            r#"{"twitch":{"id":"1","login":"a","displayName":"A","avatar":null},
                "badges":[{"id":"founder","number":37},{"id":"supporter"},{"id":"founder"}],
                "plus":{"active":true,"badge":{"status":"pending"},"live":null,"new":1},
                "equipped":{"main":"founder","custom":true,"shown":["founder"]}}"#,
        ))
        .unwrap();
        assert_eq!(me.twitch.display_name, "A");
        assert!(me.owns("founder") && me.owns("supporter") && !me.owns("custom"));
        assert_eq!(me.number_of("founder"), Some(37));
        assert_eq!(me.plus.badge.unwrap().status, "pending");
        assert_eq!(me.equipped.shown, ["founder"]);
        let bare: CyexMe = read(&reply(200, r#"{"badges":[{"id":"founder"}]}"#)).unwrap();
        assert!(!bare.owns("founder"), "a founder without a number isn't owned");
        assert!(bare.equipped.main.is_none() && !bare.plus.active);
        let rejected: CyexMe = read(&reply(200, r#"{"plus":{"badge":{"id":4,"status":"rejected","reason":"Too blurry."}}}"#)).unwrap();
        assert_eq!(rejected.plus.badge.unwrap().reason.as_deref(), Some("Too blurry."));
    }

    #[test]
    fn previews_only_from_the_own_badge_path() {
        let base = "https://api.cyex.app";
        assert!(preview_ok(base, "https://api.cyex.app/me/badge/14/2x.png"));
        assert!(preview_ok("https://api.cyex.app/", "https://api.cyex.app/me/badge/14/4x.png"));
        for bad in [
            "https://api.cyex.app/me/badge/14/2x.png?x=1",
            "https://api.cyex.app/me/badge/14/../2x.png",
            "https://api.cyex.app/me/badge/abc/2x.png",
            "https://api.cyex.app/me/badge/14/2x.svg",
            "https://api.cyex.app/me/badge/14/2x.png/more",
            "https://api.cyex.app/me/equipped",
            "https://api.cyex.app.evil.test/me/badge/14/2x.png",
            "https://evil.test/me/badge/14/2x.png",
            "http://api.cyex.app/me/badge/14/2x.png",
            "https://api.cyex.app/me/badge//2x.png",
            "https://api.cyex.app/me/badge/1234567890123/2x.png",
        ] {
            assert!(!preview_ok(base, bad), "{bad}");
        }
    }

    #[test]
    fn previews_must_be_png() {
        let ok = Reply {
            status: 200,
            etag: None,
            body: [&PNG_SIG[..], b"rest"].concat(),
        };
        assert_eq!(png(&ok).unwrap().len(), 12);
        assert_eq!(png(&reply(200, "<html>")).unwrap_err(), ApiError::Offline);
        assert_eq!(png(&reply(404, r#"{"error":"not_found","message":"x"}"#)).unwrap_err().code(), "not_found");
        assert!(png(&reply(401, r#"{"error":"signed_out"}"#)).unwrap_err().signed_out());
    }

    #[test]
    fn history_reads_and_only_approved_come_back() {
        let h: CyexHistory = read(&reply(
            200,
            r#"{"items":[
                {"id":9,"images":{"1x":"https://a/1.png","2x":"https://a/2.png","4x":"https://a/4.png"},"restorable":false,"status":"approved","uploadedAt":"2026-11-01T10:00:00Z","reviewedAt":"2026-11-01T12:00:00Z"},
                {"id":8,"images":{"1x":"https://a/me/badge/8/1x.png"},"reason":"Too blurry.","restorable":true,"status":"rejected","uploadedAt":"2026-10-20T10:00:00Z"},
                {"id":7,"images":null,"restorable":true,"status":"approved","uploadedAt":"2026-10-01T10:00:00Z","extra":1}
            ],"live":9}"#,
        ))
        .unwrap();
        let h = h.tidy();
        assert_eq!(h.live, Some(9));
        assert_eq!(h.items.len(), 3);
        assert_eq!(h.items[0].images.as_ref().unwrap().x2, "https://a/2.png");
        assert_eq!(h.items[1].reason.as_deref(), Some("Too blurry."));
        assert!(h.items[1].images.is_some() && !h.items[1].restorable, "not approved: its preview, no restore");
        assert!(h.items[2].restorable && h.items[2].images.is_none());
        let empty: CyexHistory = read(&reply(200, r#"{"items":[],"live":null}"#)).unwrap();
        assert!(empty.items.is_empty() && empty.live.is_none());
    }
}
