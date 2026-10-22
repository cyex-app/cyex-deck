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

    /// Too many requests (429): wait, then try again.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Error::Api { status: 429, .. })
    }

    /// Twitch had a problem on its side (5xx): worth retrying later.
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
            .unwrap_or_else(|| plain_body(body));
        Error::Api { status, message }
    }

    /// The same error told as coming from `service` (BTTV, FFZ, 7TV, recent-messages,
    /// logs.cyex.app), which share this client. Display stays Twitch's own.
    pub fn describe(&self, service: &str) -> String {
        match self {
            Error::Network(_) => format!("couldn’t reach {service}"),
            Error::Api { status, message } => format!("{service} said: {message} ({status})"),
            Error::Decode(_) => format!("unexpected answer from {service}"),
            Error::Refused(code) => format!("{service} said no: {code}"),
        }
    }
}

/// A body that isn't JSON, short: an HTML error page gives its `<title>`, whitespace
/// collapses, at most 200 chars.
fn plain_body(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let lower = text.to_ascii_lowercase();
    let html = lower.contains("<html") || lower.contains("<!doctype") || lower.contains("<title");
    let text = if html {
        let title = lower.find("<title").and_then(|start| {
            let open = start + lower[start..].find('>')? + 1;
            let close = open + lower[open..].find("</title")?;
            Some(&text[open..close])
        });
        match title.map(collapse).filter(|t| !t.is_empty()) {
            Some(t) => t,
            None => return "HTML error page".into(),
        }
    } else {
        collapse(&text)
    };
    text.chars().take(200).collect()
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
        assert_eq!(
            Error::api(500, b"line one\n\n   line\ttwo").api_message(),
            Some("line one line two")
        );
    }

    #[test]
    fn html_pages_shrink_to_their_title() {
        let page = b"<html>\r\n<head><title>503 Service Temporarily Unavailable</title></head>\r\n<body>\r\n<center><h1>503 Service Temporarily Unavailable</h1></center>\r\n<hr><center>nginx</center>\r\n</body>\r\n</html>\r\n";
        assert_eq!(
            Error::api(503, page).api_message(),
            Some("503 Service Temporarily Unavailable")
        );
        let spaced = b"<!DOCTYPE html><HTML><TITLE>\n  Bad \n Gateway </TITLE></HTML>";
        assert_eq!(Error::api(502, spaced).api_message(), Some("Bad Gateway"));
        let bare = b"<!doctype html><html><body><h1>Oops</h1></body></html>";
        assert_eq!(Error::api(500, bare).api_message(), Some("HTML error page"));
        let empty = b"<html><title>  </title></html>";
        assert_eq!(Error::api(500, empty).api_message(), Some("HTML error page"));
    }

    #[test]
    fn describe_names_the_service() {
        let limited = Error::Api {
            status: 429,
            message: "You have exhausted your available requests.".into(),
        };
        assert_eq!(
            limited.describe("FFZ"),
            "FFZ said: You have exhausted your available requests. (429)"
        );
        assert_eq!(
            Error::Refused("FORBIDDEN".into()).describe("7TV"),
            "7TV said no: FORBIDDEN"
        );
        let decode = Error::Decode(serde_json::from_str::<u8>("x").unwrap_err());
        assert_eq!(decode.describe("BTTV"), "unexpected answer from BTTV");
        // Twitch's own wording is unchanged.
        assert_eq!(limited.to_string(), "Twitch said: You have exhausted your available requests. (429)");
    }
}
