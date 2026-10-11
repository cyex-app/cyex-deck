//! 1.35 The stream manager over GQL, the way Twitch's creator dashboard runs it (V101-28,
//! D30). Every shape was checked against the live schema on 2026/10/11; no write was run.
//! Answers reuse the Helix types, so the UI doesn't care which API ran.

pub mod ads;
pub mod feed;
pub mod info;
pub mod modes;
pub mod raids;
pub mod stats;
pub mod together;
pub mod votes;

use serde::Deserialize;
use serde_json::Value;

use super::{Auth, query};
use crate::{Error, Http, Result};

/// A payload's `error { code }`.
#[derive(Deserialize)]
pub(crate) struct Refusal {
    code: Option<String>,
}

/// `Err(Refused)` when a payload carries an error code.
pub(crate) fn check(error: Option<Refusal>) -> Result<()> {
    match error.and_then(|e| e.code) {
        Some(code) => Err(Error::Refused(code)),
        None => Ok(()),
    }
}

/// Runs a mutation whose payload is `{ error { code } }` under `field`.
pub(crate) async fn mutate(http: &Http, auth: Auth<'_>, text: &str, field: &str, input: Value) -> Result<()> {
    #[derive(Deserialize)]
    struct Payload {
        error: Option<Refusal>,
    }
    let data: Value = query(http, auth, text, serde_json::json!({ "input": input })).await?;
    let payload = data.get(field).cloned().unwrap_or(Value::Null);
    if payload.is_null() {
        return Ok(());
    }
    let p: Payload = serde_json::from_value(payload).map_err(Error::Decode)?;
    check(p.error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_a_refusal() {
        assert!(check(None).is_ok());
        assert!(check(Some(Refusal { code: None })).is_ok());
        let e = check(Some(Refusal {
            code: Some("FORBIDDEN".into()),
        }))
        .unwrap_err();
        assert!(matches!(e, Error::Refused(c) if c == "FORBIDDEN"));
    }
}
