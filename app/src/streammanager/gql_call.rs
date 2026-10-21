//! V107-R41 The manager's GQL sign-in for every dashboard call, in one place: Twitch with
//! the account's own token, or the local mock with a fake one (SEC-01: no real token goes
//! to a mock). A mock only in mocks builds, only on a test data folder, only at
//! `http://127.0.0.1:<port>` (`CYEX_GQL_DASHBOARD_MOCK`; the older `CYEX_GQL_CONTENT_MOCK`
//! and `CYEX_GQL_REWARDS_MOCK` still work).

use cyex_twitch::Http;
use cyex_twitch::gql::{self, dashboard::Channel};
use tauri::AppHandle;

const GQL_ONLY: &str = "This needs the sign-in with a code (Settings → Accounts).";
/// The fake sign-in the mock gets.
const MOCK_TOKEN: &str = "mock-token";
#[cfg(feature = "mocks")]
const MOCK_VARS: [&str; 3] = [
    "CYEX_GQL_DASHBOARD_MOCK",
    "CYEX_GQL_CONTENT_MOCK",
    "CYEX_GQL_REWARDS_MOCK",
];

/// Who calls and where.
pub struct GqlCall {
    pub http: Http,
    token: String,
    client_id: String,
    base: Option<String>,
    /// The manager's own channel (its user id).
    pub channel_id: String,
    pub login: String,
}

impl GqlCall {
    pub fn auth(&self) -> gql::Auth<'_> {
        crate::auth::gql_auth(&self.token, &self.client_id)
    }

    /// The dashboard of the manager's channel.
    pub fn channel(&self) -> Channel<'_> {
        Channel {
            http: &self.http,
            auth: self.auth(),
            base: self.base.as_deref(),
            id: &self.channel_id,
        }
    }
}

/// The manager's GQL call, or the line to show.
pub fn gql_call(app: &AppHandle) -> Result<GqlCall, String> {
    let m = super::manager(app)
        .ok_or("The stream manager needs a Twitch sign-in for your channel (Settings → Accounts).")?;
    let base = mock_base(app)?;
    let (token, client_id) = sign_in(base.as_deref(), || crate::auth::gql_token(app, &m.account_id))?;
    Ok(GqlCall {
        http: super::http(app),
        token,
        client_id,
        base,
        channel_id: m.account_id,
        login: m.login,
    })
}

/// The token for `base`: a mock never sees the real one (it isn't even read).
fn sign_in(
    base: Option<&str>,
    real: impl FnOnce() -> Option<(String, String)>,
) -> Result<(String, String), String> {
    match base {
        Some(_) => Ok((MOCK_TOKEN.to_owned(), cyex_twitch::clients::WEB.to_owned())),
        None => real().ok_or_else(|| GQL_ONLY.into()),
    }
}

#[cfg(feature = "mocks")]
fn mock_base(app: &AppHandle) -> Result<Option<String>, String> {
    let raw = MOCK_VARS.iter().find_map(|v| std::env::var(v).ok());
    pick(raw.as_deref(), crate::paths::webview(app).is_some())
}

#[cfg(not(feature = "mocks"))]
fn mock_base(_: &AppHandle) -> Result<Option<String>, String> {
    Ok(None)
}

/// The mock to use: none when unset; set, it needs a test data folder and a loopback
/// address, or nothing is sent at all (never Twitch with a real token instead).
#[cfg(feature = "mocks")]
fn pick(raw: Option<&str>, test_data: bool) -> Result<Option<String>, String> {
    let Some(raw) = raw.filter(|r| !r.trim().is_empty()) else {
        return Ok(None);
    };
    if !test_data {
        return Err("The GQL mock runs only on a test data folder.".into());
    }
    crate::mock_url::loopback_base(raw)
        .map(Some)
        .ok_or_else(|| "The GQL mock must be http://127.0.0.1:<port>.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "mocks")]
    #[test]
    fn v107_r41_a_mock_needs_test_data_and_loopback() {
        assert_eq!(pick(None, false), Ok(None));
        assert_eq!(pick(Some(" "), true), Ok(None));
        assert_eq!(
            pick(Some("http://127.0.0.1:5231"), true),
            Ok(Some("http://127.0.0.1:5231".into()))
        );
        assert!(
            pick(Some("http://127.0.0.1:5231"), false).is_err(),
            "real data: refused, not sent to Twitch"
        );
        assert!(pick(Some("http://evil.example:5231"), true).is_err());
        assert!(pick(Some("http://127.0.0.1:1@evil.example"), true).is_err());
    }

    #[test]
    fn v107_r41_the_mock_gets_a_fake_token() {
        let real = || -> Option<(String, String)> { panic!("the real token was read for the mock") };
        let (t, _) = sign_in(Some("http://127.0.0.1:1"), real).unwrap();
        assert_eq!(t, MOCK_TOKEN);
        let (t, c) = sign_in(None, || Some(("real".into(), "client".into()))).unwrap();
        assert_eq!((t.as_str(), c.as_str()), ("real", "client"));
        assert_eq!(sign_in(None, || None).unwrap_err(), GQL_ONLY);
    }
}
