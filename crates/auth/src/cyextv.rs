//! V112-14 Sign-in to cyex.tv (platform ADR 0021): authorization code with PKCE S256 and a
//! loopback redirect on `http://127.0.0.1:<free port>/callback`, then form-encoded token calls.
//! The access token lasts an hour and lives in memory only; the refresh token (60 days, rotated
//! on every use) is kept per Twitch account in the token store (`sessions`). Tokens are never
//! logged, printed or put in an error.

pub mod callback;
pub mod sessions;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cyex_twitch::Http;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::browser::encode;

/// The deck's public client (no secret).
pub const CLIENT_ID: &str = "cyex-deck";
/// cyex.tv's API; a mocks build may point at a loopback mock instead.
pub const API: &str = "https://api.cyex.tv/v1";
/// Longest token kept (the vault holds about 1.200 characters an entry).
const MAX_TOKEN: usize = 1024;
/// Largest token answer read.
const MAX_ANSWER: usize = 64 * 1024;

/// Why a cyex.tv sign-in or call didn't work. Worded for the UI; never carries a token.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CyextvError {
    /// No session, or cyex.tv ended it (an old refresh token, a revoke, 60 days gone).
    #[error("Sign in to cyex.tv again.")]
    SignedOut,
    #[error("Sign-in was declined on cyex.tv.")]
    Denied,
    #[error("Cancelled.")]
    Cancelled,
    #[error("The sign-in took too long. Start again.")]
    TimedOut,
    #[error("Couldn’t reach cyex.tv. Try again soon.")]
    Down,
    #[error("Too many tries. Wait a minute.")]
    RateLimited,
    #[error("Couldn’t save the sign-in: {0}")]
    Store(String),
    #[error("cyex.tv said: {0}")]
    Said(String),
}

/// A PKCE pair (RFC 7636): the verifier stays here, the challenge goes to the authorize page.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl std::fmt::Debug for Pkce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pkce(..)")
    }
}

impl Pkce {
    /// 32 random bytes as the verifier (43 characters).
    pub fn new() -> Result<Self, CyextvError> {
        Ok(Self::from_verifier(&random(32)?))
    }

    /// S256: `BASE64URL(SHA256(verifier))`.
    pub fn from_verifier(verifier: &str) -> Self {
        Self {
            verifier: verifier.to_owned(),
            challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
        }
    }
}

/// `n` bytes from the OS, base64url.
fn random(n: usize) -> Result<String, CyextvError> {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).map_err(|e| CyextvError::Said(e.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

/// A random `state` (128 bits).
pub fn new_state() -> Result<String, CyextvError> {
    random(16)
}

pub fn redirect_uri(port: u16) -> String {
    format!("http://127.0.0.1:{port}/callback")
}

/// cyex.tv's authorize page (it shows the consent page, then sends the browser to `redirect`).
pub fn authorize_url(base: &str, redirect: &str, challenge: &str, state: &str) -> String {
    format!(
        "{}/auth/app/authorize?response_type=code&client_id={CLIENT_ID}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        base.trim_end_matches('/'),
        encode(redirect),
        encode(challenge),
        encode(state),
    )
}

/// Whether `url` is the authorize page of `base`, the only page the sign-in opens.
pub fn is_authorize_url(base: &str, url: &str) -> bool {
    let page = format!("{}/auth/app/authorize?", base.trim_end_matches('/'));
    url.starts_with(&page) && !url.contains(['#', ' ', '\n'])
}

/// What a token call gave. A secret: `Debug` never shows the tokens.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenAnswer {
    pub access: String,
    pub refresh: String,
    /// Seconds the access token lasts.
    pub expires_in: i64,
    pub scope: String,
}

impl std::fmt::Debug for TokenAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TokenAnswer {{ expires_in: {}, scope: {:?}, .. }}", self.expires_in, self.scope)
    }
}

/// A token is short printable text without spaces.
pub(crate) fn token_ok(t: &str) -> bool {
    !t.is_empty() && t.len() <= MAX_TOKEN && t.bytes().all(|b| b.is_ascii_graphic())
}

#[derive(Deserialize)]
struct RawAnswer {
    access_token: String,
    #[serde(default)]
    token_type: String,
    #[serde(default)]
    expires_in: i64,
    refresh_token: String,
    #[serde(default)]
    scope: String,
}

#[derive(Deserialize)]
struct RawError {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
    #[serde(default)]
    message: String,
}

/// A token answer (RFC 6749 5.1/5.2). `invalid_grant` means the session is over.
pub fn parse_token(status: u16, body: &[u8]) -> Result<TokenAnswer, CyextvError> {
    if status == 200 {
        let raw: RawAnswer = serde_json::from_slice(body).map_err(|_| CyextvError::Down)?;
        let bearer = raw.token_type.is_empty() || raw.token_type.eq_ignore_ascii_case("bearer");
        if !bearer || !token_ok(&raw.access_token) || !token_ok(&raw.refresh_token) {
            return Err(CyextvError::Down);
        }
        let expires_in = if raw.expires_in <= 0 { 3600 } else { raw.expires_in.min(86_400) };
        return Ok(TokenAnswer {
            access: raw.access_token,
            refresh: raw.refresh_token,
            expires_in,
            scope: raw.scope.chars().take(200).collect(),
        });
    }
    let raw: Option<RawError> = serde_json::from_slice(body).ok();
    let code = raw.as_ref().map_or("", |r| r.error.as_str());
    match (status, code) {
        (_, "invalid_grant") => Err(CyextvError::SignedOut),
        (429, _) | (_, "rate_limited") => Err(CyextvError::RateLimited),
        (500.., _) | (_, "server_error") => Err(CyextvError::Down),
        _ => {
            let said = raw
                .map(|r| if r.error_description.is_empty() { r.message } else { r.error_description })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("error {status}"));
            Err(CyextvError::Said(said.chars().take(200).collect()))
        }
    }
}

/// `application/x-www-form-urlencoded`.
pub fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn code_form(code: &str, verifier: &str, redirect: &str) -> String {
    form(&[
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("code_verifier", verifier),
        ("redirect_uri", redirect),
    ])
}

pub fn refresh_form(refresh: &str) -> String {
    form(&[("grant_type", "refresh_token"), ("client_id", CLIENT_ID), ("refresh_token", refresh)])
}

pub fn revoke_form(token: &str) -> String {
    form(&[("token", token), ("token_type_hint", "refresh_token"), ("client_id", CLIENT_ID)])
}

/// One form POST. The client's own user agent (`cyex-deck/<version>`) goes with it.
async fn post_form(http: &Http, url: &str, body: String) -> Result<(u16, Vec<u8>), CyextvError> {
    let body = Some(("application/x-www-form-urlencoded".to_owned(), body.into_bytes()));
    http.send("POST", url, None, body, MAX_ANSWER)
        .await
        .map_err(|_| CyextvError::Down)
}

/// The code from the callback for tokens.
pub async fn exchange(http: &Http, base: &str, code: &str, verifier: &str, redirect: &str) -> Result<TokenAnswer, CyextvError> {
    let url = format!("{}/auth/app/token", base.trim_end_matches('/'));
    let (status, body) = post_form(http, &url, code_form(code, verifier, redirect)).await?;
    parse_token(status, &body)
}

/// New tokens for a refresh token, which is used up by this call.
pub async fn refresh(http: &Http, base: &str, refresh: &str) -> Result<TokenAnswer, CyextvError> {
    let url = format!("{}/auth/app/token", base.trim_end_matches('/'));
    let (status, body) = post_form(http, &url, refresh_form(refresh)).await?;
    parse_token(status, &body)
}

/// Ends the session the token belongs to (RFC 7009); 200 whether or not it was known.
pub async fn revoke(http: &Http, base: &str, token: &str) -> Result<(), CyextvError> {
    let url = format!("{}/auth/app/revoke", base.trim_end_matches('/'));
    match post_form(http, &url, revoke_form(token)).await? {
        (200..=299, _) => Ok(()),
        (status, body) => parse_token(status, &body).map(|_| ()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S256 checked against another SHA-256 (.NET's) for the same verifier.
    #[test]
    fn pkce_s256_is_base64url_sha256() {
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mJ92K9a7BebthXZIXnbmZYNchSt2Kw");
        assert_eq!(p.challenge, "zEpsi2LMVqzhG7LxtGMPxJsFsDrOZX-bb2U40hx4LXk");
        assert_eq!(format!("{p:?}"), "Pkce(..)");
    }

    #[test]
    fn random_verifiers_and_states_differ_and_fit_the_rfc() {
        let (a, b) = (Pkce::new().unwrap(), Pkce::new().unwrap());
        assert_eq!(a.verifier.len(), 43);
        assert_ne!(a.verifier, b.verifier);
        assert!(a.verifier.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
        assert_eq!(Pkce::from_verifier(&a.verifier).challenge, a.challenge);
        let (s, t) = (new_state().unwrap(), new_state().unwrap());
        assert_eq!(s.len(), 22);
        assert_ne!(s, t);
    }

    #[test]
    fn the_authorize_url_has_every_part() {
        let r = redirect_uri(51234);
        assert_eq!(r, "http://127.0.0.1:51234/callback");
        let u = authorize_url(API, &r, "ch_al-lenge", "st+ate");
        assert_eq!(
            u,
            "https://api.cyex.tv/v1/auth/app/authorize?response_type=code&client_id=cyex-deck\
             &redirect_uri=http%3A%2F%2F127.0.0.1%3A51234%2Fcallback&code_challenge=ch_al-lenge\
             &code_challenge_method=S256&state=st%2Bate"
        );
        assert!(is_authorize_url(API, &u));
        assert!(is_authorize_url("http://127.0.0.1:5260/v1/", &authorize_url("http://127.0.0.1:5260/v1/", &r, "c", "s")));
        assert!(!is_authorize_url(API, "https://api.cyex.tv/v1/auth/app/authorizeX?a"));
        assert!(!is_authorize_url(API, "https://evil.example/?https://api.cyex.tv/v1/auth/app/authorize?"));
        assert!(!is_authorize_url(API, "https://api.cyex.tv.evil.example/v1/auth/app/authorize?a"));
    }

    #[test]
    fn forms_are_url_encoded() {
        assert_eq!(
            code_form("c/1", "v", "http://127.0.0.1:1/callback"),
            "grant_type=authorization_code&client_id=cyex-deck&code=c%2F1&code_verifier=v\
             &redirect_uri=http%3A%2F%2F127.0.0.1%3A1%2Fcallback"
        );
        assert_eq!(refresh_form("r=1"), "grant_type=refresh_token&client_id=cyex-deck&refresh_token=r%3D1");
        assert_eq!(revoke_form("t"), "token=t&token_type_hint=refresh_token&client_id=cyex-deck");
    }

    #[test]
    fn token_answers_parse() {
        let ok = br#"{"access_token":"a.b","token_type":"Bearer","expires_in":3600,"refresh_token":"r1","scope":"set_emotes cosmetics"}"#;
        let t = parse_token(200, ok).unwrap();
        assert_eq!((t.access.as_str(), t.refresh.as_str(), t.expires_in), ("a.b", "r1", 3600));
        assert_eq!(t.scope, "set_emotes cosmetics");
        let shown = format!("{t:?}");
        assert!(!shown.contains("a.b") && !shown.contains("r1"), "{shown}");
        // No expiry or a silly one: an hour, capped at a day.
        let t = parse_token(200, br#"{"access_token":"a","refresh_token":"r"}"#).unwrap();
        assert_eq!(t.expires_in, 3600);
        let t = parse_token(200, br#"{"access_token":"a","refresh_token":"r","expires_in":9999999}"#).unwrap();
        assert_eq!(t.expires_in, 86_400);
        // Junk is never kept.
        for bad in [
            &br#"{"access_token":"a b","refresh_token":"r"}"#[..],
            br#"{"access_token":"a","refresh_token":""}"#,
            br#"{"access_token":"a","refresh_token":"r","token_type":"mac"}"#,
            br#"not json"#,
        ] {
            assert_eq!(parse_token(200, bad), Err(CyextvError::Down));
        }
    }

    #[test]
    fn token_errors_map() {
        let err = |e: &str| format!(r#"{{"error":"{e}","error_description":"Nope.","message":"Nope."}}"#).into_bytes();
        assert_eq!(parse_token(400, &err("invalid_grant")), Err(CyextvError::SignedOut));
        assert_eq!(parse_token(429, b"{}"), Err(CyextvError::RateLimited));
        assert_eq!(parse_token(500, &err("server_error")), Err(CyextvError::Down));
        assert_eq!(parse_token(401, &err("invalid_client")), Err(CyextvError::Said("Nope.".into())));
        assert_eq!(parse_token(400, b""), Err(CyextvError::Said("error 400".into())));
    }
}
