//! V109-B02 The calls to api.cyex.app an account makes. Errors come as `{ error, message }`;
//! nothing here prints a token or a URL.

use cyex_twitch::{Call, Http, Reply, Verb};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::model::{CyexEquipped, CyexMe};

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
        assert_eq!(me.founder_number(), Some(37));
        assert_eq!(me.plus.badge.unwrap().status, "pending");
        assert_eq!(me.equipped.shown, ["founder"]);
        let bare: CyexMe = read(&reply(200, r#"{"badges":[{"id":"founder"}]}"#)).unwrap();
        assert!(!bare.owns("founder"), "a founder without a number isn't owned");
        assert!(bare.equipped.main.is_none() && !bare.plus.active);
    }
}
