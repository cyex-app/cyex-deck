//! C1 Recent chat history from recent-messages.robotty.de, the service Chatterino uses.
//! Lines come back as raw IRC, oldest first, tagged `historical=1`.

use serde::Deserialize;

use crate::pace::{self, Lane, Service};
use crate::{Error, Http, Result};

const URL: &str = "https://recent-messages.robotty.de/api/v2/recent-messages";

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    messages: Vec<String>,
    error_code: Option<String>,
    error: Option<String>,
}

/// At most `limit` lines (V101-03); the service keeps up to 800. Paced and retried (V107-P2).
pub async fn recent_messages(http: &Http, login: &str, limit: u32, lane: Lane) -> Result<Vec<String>> {
    let req = || http.0.get(format!("{URL}/{login}")).query(&[("limit", limit)]);
    unwrap(pace::json(Service::Recent, lane, req).await?)
}

/// V101-32 Why there's no history, as a line for the chat.
pub fn unavailable(e: &Error) -> String {
    let ignored = e.api_message().is_some_and(|m| m.contains("ignored") || m.contains("excluded"));
    match e.status() {
        _ if ignored => "Chat history is off for this channel".into(),
        Some(status) if status != 200 => format!("Chat history unavailable (error {status})"),
        _ if e.is_network() => "Chat history unavailable: couldn’t reach the service".into(),
        _ => "Chat history unavailable right now".into(),
    }
}

/// C24 Lines sent between two moments (unix ms), at most `limit`, and whether the service
/// was still catching up on the channel (then it may lack some of them).
pub async fn recent_between(
    http: &Http,
    login: &str,
    after_ms: i64,
    before_ms: i64,
    limit: u32,
) -> Result<(Vec<String>, bool)> {
    let req = || {
        http.0
            .get(format!("{URL}/{login}"))
            .query(&[("limit", i64::from(limit)), ("after", after_ms), ("before", before_ms)])
    };
    let r: Response = pace::json(Service::Recent, Lane::Back, req).await?;
    let recovering = recovering(&r);
    Ok((unwrap(r)?, recovering))
}

fn recovering(r: &Response) -> bool {
    r.error_code.as_deref() == Some("channel_not_joined")
}

fn unwrap(r: Response) -> Result<Vec<String>> {
    match r.error_code.as_deref() {
        // Still recovering: it sends what it has.
        None | Some("channel_not_joined") => Ok(r.messages),
        Some(code) => Err(Error::Api {
            status: 200,
            message: format!("{code}: {}", r.error.unwrap_or_default()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_and_gaps() {
        let ok: Response = serde_json::from_str(r#"{"messages":["a"],"error":null,"error_code":null}"#).unwrap();
        assert_eq!(unwrap(ok).unwrap(), ["a"]);
        let gap: Response =
            serde_json::from_str(r#"{"messages":["b"],"error":"recovering","error_code":"channel_not_joined"}"#).unwrap();
        assert!(recovering(&gap));
        assert_eq!(unwrap(gap).unwrap(), ["b"]);
        let bad: Response =
            serde_json::from_str(r#"{"messages":[],"error":"ignored","error_code":"channel_ignored"}"#).unwrap();
        assert!(unwrap(bad).is_err());
    }

    #[test]
    fn reasons_read_short() {
        let ignored = unwrap(serde_json::from_str(r#"{"messages":[],"error":"x","error_code":"channel_ignored"}"#).unwrap());
        assert_eq!(unavailable(&ignored.unwrap_err()), "Chat history is off for this channel");
        let forbidden = Error::api(403, br#"{"status":403,"error":"Forbidden"}"#);
        assert_eq!(unavailable(&forbidden), "Chat history unavailable (error 403)");
        let odd = unwrap(serde_json::from_str(r#"{"messages":[],"error":"x","error_code":"other"}"#).unwrap());
        assert_eq!(unavailable(&odd.unwrap_err()), "Chat history unavailable right now");
    }
}
