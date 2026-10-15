use serde::Deserialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Offline, DNS, timeout: worth retrying later.
    #[error("couldn’t reach Twitch")]
    Network(#[source] reqwest::Error),
    /// Twitch answered with an error.
    #[error("Twitch said: {message} ({status})")]
    Api { status: u16, message: String },
    #[error("unexpected answer from Twitch")]
    Decode(#[source] serde_json::Error),
    /// GQL ran the mutation and said no: the code from its payload's `error`.
    #[error("Twitch said no: {0}")]
    Refused(String),
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
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Error::Api { status: 429, .. })
    }

    pub fn is_server_error(&self) -> bool {
        matches!(self, Error::Api { status, .. } if *status >= 500)
    }

    /// The HTTP status Twitch answered with, if this is an API error.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// GQL turned the query itself down because a field, type or argument changed on
    /// Twitch's side. Nothing ran, so another API can try.
    pub fn is_schema_error(&self) -> bool {
        const SIGNS: [&str; 6] = [
            "Cannot query field",
            "Unknown type",
            "Unknown argument",
            "is not defined by type",
            "of required type",
            "PersistedQueryNotFound",
        ];
        matches!(self, Error::Api { status: 200, message } if SIGNS.iter().any(|s| message.contains(s)))
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
    fn schema_errors_are_told_apart() {
        let e = |m: &str| Error::Api { status: 200, message: m.into() };
        assert!(e(r#"Cannot query field "startAd" on type "Mutation"."#).is_schema_error());
        assert!(e(r#"Field "StartAdInput.channelID" of required type "ID!" was not provided."#).is_schema_error());
        assert!(!e("service timeout").is_schema_error());
        assert!(!Error::Refused("FORBIDDEN".into()).is_schema_error());
    }

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
