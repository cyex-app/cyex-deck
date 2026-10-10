//! C1 Recent chat history from recent-messages.robotty.de, the service Chatterino uses.
//! Lines come back as raw IRC, oldest first, tagged `historical=1`.

use serde::Deserialize;

use crate::http::json;
use crate::{Error, Http, Result};

const URL: &str = "https://recent-messages.robotty.de/api/v2/recent-messages";
/// Lines to ask for. The service keeps up to 800.
pub const LIMIT: u32 = 200;

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    messages: Vec<String>,
    error_code: Option<String>,
    error: Option<String>,
}

pub async fn recent_messages(http: &Http, login: &str) -> Result<Vec<String>> {
    let req = http.0.get(format!("{URL}/{login}")).query(&[("limit", LIMIT)]);
    unwrap(json(req).await?)
}

/// C24 Lines sent between two moments (unix ms), at most `limit`.
pub async fn recent_between(http: &Http, login: &str, after_ms: i64, before_ms: i64, limit: u32) -> Result<Vec<String>> {
    let req = http
        .0
        .get(format!("{URL}/{login}"))
        .query(&[("limit", i64::from(limit)), ("after", after_ms), ("before", before_ms)]);
    unwrap(json(req).await?)
}

fn unwrap(r: Response) -> Result<Vec<String>> {
    match r.error_code.as_deref() {
        // Still recovering: it sends what it has.
        None | Some("channel_not_joined") => Ok(r.messages),
        Some(code) => Err(Error::Api {
            status: 200,
            message: r.error.unwrap_or_else(|| code.to_owned()),
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
        assert_eq!(unwrap(gap).unwrap(), ["b"]);
        let bad: Response =
            serde_json::from_str(r#"{"messages":[],"error":"ignored","error_code":"channel_ignored"}"#).unwrap();
        assert!(unwrap(bad).is_err());
    }
}
