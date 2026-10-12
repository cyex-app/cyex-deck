//! Device Code sign-in (1.5, D5, D6): show a code, the user approves it on
//! twitch.tv/activate, we poll until Twitch hands over the token.

use std::time::{Duration, Instant};

use cyex_twitch::id::{self, Poll};
use cyex_twitch::{Http, clients};
use tokio::sync::watch;

pub use cyex_twitch::id::DeviceCode;

use crate::secrets::Secret;
use crate::{AuthError, TokenKind, scopes};

/// Helix sign-in uses this app's client (V103-01); GQL sign-in always `clients::GQL`
/// (V102-19).
pub fn client_for(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Helix => clients::SIGN_IN,
        TokenKind::Gql => clients::GQL,
    }
}

fn scopes_for(kind: TokenKind) -> &'static [&'static str] {
    match kind {
        TokenKind::Helix => scopes::HELIX,
        TokenKind::Gql => scopes::GQL,
    }
}

pub async fn start(http: &Http, client_id: &str, kind: TokenKind) -> Result<DeviceCode, AuthError> {
    id::device_code(http, client_id, scopes_for(kind))
        .await
        .map_err(|e| match e.api_message() {
            Some(m) if m.contains("invalid client") => AuthError::BadClientId,
            _ => e.into(),
        })
}

/// Polls until the user approves, the code expires, or `cancel` turns true.
pub async fn wait(
    http: &Http,
    client_id: &str,
    kind: TokenKind,
    code: &DeviceCode,
    mut cancel: watch::Receiver<bool>,
) -> Result<Secret, AuthError> {
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = Duration::from_secs(code.interval.clamp(1, 30));
    loop {
        tokio::select! {
            () = tokio::time::sleep(interval) => {}
            _ = cancel.wait_for(|c| *c) => return Err(AuthError::Cancelled),
        }
        if Instant::now() >= deadline {
            return Err(AuthError::Expired);
        }
        match id::device_poll(http, client_id, scopes_for(kind), &code.device_code).await {
            Ok(Poll::Pending) => {}
            Ok(Poll::SlowDown) => interval += Duration::from_secs(5),
            Ok(Poll::Done(grant)) => {
                return Ok(Secret {
                    access: grant.access_token,
                    refresh: grant.refresh_token,
                    client_id: client_id.to_owned(),
                });
            }
            // A dropped connection shouldn't end the sign-in; try again next round.
            Err(e) if e.is_network() || e.is_server_error() => {}
            Err(e) => return Err(poll_error(e)),
        }
    }
}

fn poll_error(e: cyex_twitch::Error) -> AuthError {
    let message = e.api_message().unwrap_or_default().to_ascii_lowercase();
    if message.contains("invalid device code") || message.contains("expired") {
        AuthError::Expired
    } else if message.contains("denied") || message.contains("declined") {
        AuthError::Denied
    } else {
        e.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_twitch::Error;

    fn api(message: &str) -> Error {
        Error::Api {
            status: 400,
            message: message.into(),
        }
    }

    #[test]
    fn poll_errors_read_well() {
        assert!(matches!(
            poll_error(api("invalid device code")),
            AuthError::Expired
        ));
        assert!(matches!(poll_error(api("access_denied")), AuthError::Denied));
        assert!(matches!(poll_error(api("something new")), AuthError::Twitch(_)));
    }

    #[test]
    fn each_sign_in_has_its_client() {
        assert_eq!(client_for(TokenKind::Helix), clients::SIGN_IN);
        assert_eq!(client_for(TokenKind::Gql), clients::GQL);
    }
}
