//! id.twitch.tv: Device Code flow, refresh, validate, revoke (1.5).

use serde::Deserialize;

use crate::http::{empty, json};
use crate::{Http, Result};

const BASE: &str = "https://id.twitch.tv/oauth2";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Clone, Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Access token plus the one-time refresh token that replaces it.
#[derive(Clone, Deserialize)]
pub struct Grant {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

impl std::fmt::Debug for Grant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Grant { .. }")
    }
}

/// One poll of a pending Device Code sign-in.
#[derive(Debug)]
pub enum Poll {
    /// The user hasn't approved yet.
    Pending,
    /// Polling too fast: wait longer between polls.
    SlowDown,
    Done(Grant),
}

/// What `oauth2/validate` reports about a token.
#[derive(Clone, Debug, Deserialize)]
pub struct Validation {
    pub client_id: String,
    /// Empty for app tokens.
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    /// Seconds left. 0 means the token doesn't expire.
    pub expires_in: u64,
}

pub async fn device_code(http: &Http, client_id: &str, scopes: &[&str]) -> Result<DeviceCode> {
    let form = [("client_id", client_id), ("scopes", &scopes.join(" "))];
    json(http.0.post(format!("{BASE}/device")).form(&form)).await
}

pub async fn device_poll(http: &Http, client_id: &str, scopes: &[&str], device_code: &str) -> Result<Poll> {
    let form = [
        ("client_id", client_id),
        ("scopes", &scopes.join(" ")),
        ("device_code", device_code),
        ("grant_type", DEVICE_GRANT),
    ];
    classify_poll(json(http.0.post(format!("{BASE}/token")).form(&form)).await)
}

fn classify_poll(result: Result<Grant>) -> Result<Poll> {
    match result {
        Ok(grant) => Ok(Poll::Done(grant)),
        Err(e) => match e.api_message() {
            Some("authorization_pending") => Ok(Poll::Pending),
            Some("slow_down") => Ok(Poll::SlowDown),
            _ => Err(e),
        },
    }
}

/// Public clients refresh without a secret. Refresh tokens are one-time use.
pub async fn refresh(http: &Http, client_id: &str, refresh_token: &str) -> Result<Grant> {
    let form = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    json(http.0.post(format!("{BASE}/token")).form(&form)).await
}

pub async fn validate(http: &Http, token: &str) -> Result<Validation> {
    json(
        http.0
            .get(format!("{BASE}/validate"))
            .header("Authorization", format!("OAuth {token}")),
    )
    .await
}

pub async fn revoke(http: &Http, client_id: &str, token: &str) -> Result<()> {
    let form = [("client_id", client_id), ("token", token)];
    empty(http.0.post(format!("{BASE}/revoke")).form(&form)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn poll_answers_are_classified() {
        let pending = classify_poll(Err(Error::api(
            400,
            br#"{"status":400,"message":"authorization_pending"}"#,
        )));
        assert!(matches!(pending, Ok(Poll::Pending)));
        let slow = classify_poll(Err(Error::api(400, br#"{"status":400,"message":"slow_down"}"#)));
        assert!(matches!(slow, Ok(Poll::SlowDown)));
        let expired = classify_poll(Err(Error::api(
            400,
            br#"{"status":400,"message":"invalid device code"}"#,
        )));
        assert!(expired.is_err());
    }

    #[test]
    fn validation_parses_user_and_non_expiring_tokens() {
        let v: Validation = serde_json::from_str(
            r#"{"client_id":"abc","login":"someone","scopes":["chat:read"],"user_id":"123","expires_in":14000}"#,
        )
        .unwrap();
        assert_eq!(v.login, "someone");
        assert_eq!(v.scopes.unwrap(), ["chat:read"]);
        let v: Validation = serde_json::from_str(
            r#"{"client_id":"abc","login":"x","scopes":null,"user_id":"1","expires_in":0}"#,
        )
        .unwrap();
        assert_eq!(v.expires_in, 0);
        assert!(v.scopes.is_none());
    }

    #[test]
    fn grants_parse_with_or_without_expiry() {
        let g: Grant = serde_json::from_str(
            r#"{"access_token":"a","refresh_token":"r","expires_in":14400,"scope":["x"],"token_type":"bearer"}"#,
        )
        .unwrap();
        assert_eq!(g.expires_in, Some(14400));
        let g: Grant =
            serde_json::from_str(r#"{"access_token":"a","refresh_token":"r","token_type":"bearer"}"#)
                .unwrap();
        assert!(g.expires_in.is_none());
        assert_eq!(format!("{g:?}"), "Grant { .. }");
    }
}
