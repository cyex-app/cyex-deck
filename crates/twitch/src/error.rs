use serde::Deserialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Offline, DNS, timeout: worth retrying later.
    #[error("couldn't reach Twitch")]
    Network(#[source] reqwest::Error),
    /// Twitch answered with an error.
    #[error("Twitch said: {message} ({status})")]
    Api { status: u16, message: String },
    #[error("unexpected answer from Twitch")]
    Decode(#[source] serde_json::Error),
}

impl Error {
    pub fn is_network(&self) -> bool {
        matches!(self, Error::Network(_))
    }

    /// The token is invalid or revoked.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Error::Api { status: 401, .. })
    }

    /// Twitch had a problem on its side (5xx): worth retrying later.
    pub fn is_server_error(&self) -> bool {
        matches!(self, Error::Api { status, .. } if *status >= 500)
    }

    /// The `message` Twitch sent, if this is an API error.
    pub fn api_message(&self) -> Option<&str> {
        match self {
            Error::Api { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Builds an API error from a status and body like `{"status":400,"message":"..."}`.
    pub(crate) fn api(status: u16, body: &[u8]) -> Self {
        #[derive(Deserialize)]
        struct Body {
            message: Option<String>,
            error: Option<String>,
        }
        let parsed = serde_json::from_slice::<Body>(body).ok();
        let message = parsed
            .and_then(|b| b.message.filter(|m| !m.is_empty()).or(b.error))
            .unwrap_or_else(|| String::from_utf8_lossy(body).trim().chars().take(200).collect());
        Error::Api { status, message }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn api_errors_use_the_message_field() {
        let e = Error::api(400, br#"{"status":400,"message":"authorization_pending"}"#);
        assert_eq!(e.api_message(), Some("authorization_pending"));
        let e = Error::api(
            401,
            br#"{"error":"Unauthorized","status":401,"message":"Invalid OAuth token"}"#,
        );
        assert!(e.is_unauthorized());
        assert_eq!(e.api_message(), Some("Invalid OAuth token"));
    }

    #[test]
    fn api_errors_fall_back_to_the_body() {
        assert_eq!(
            Error::api(502, b" Bad Gateway ").api_message(),
            Some("Bad Gateway")
        );
        assert_eq!(
            Error::api(500, br#"{"error":"Internal"}"#).api_message(),
            Some("Internal")
        );
    }
}
