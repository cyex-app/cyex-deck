//! V1051-02 What can go wrong with a 7TV account call. Never holds a body or a URL.

use std::fmt;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// HTTP 401, LOGIN_REQUIRED.
    SignInAgain,
    /// 403, LACKING_PRIVILEGES, LOAD_ERROR on a mutation.
    NotAllowed,
    /// 429, RATE_LIMIT_EXCEEDED (30 s when 7TV doesn't say).
    RateLimited { seconds: u64 },
    /// Network, timeout, 5xx.
    Down,
    /// Anything else, parse errors.
    Bad,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::SignInAgain => "sign in again",
            Error::NotAllowed => "not allowed",
            Error::RateLimited { .. } => "rate limited",
            Error::Down => "7TV is down",
            Error::Bad => "unexpected answer",
        })
    }
}

impl std::error::Error for Error {}

const DEFAULT_WAIT: u64 = 30;
/// Longer than this is no wait: `x-ratelimit-reset` can be a unix time, not seconds.
const MAX_WAIT: u64 = 3600;

/// How long to wait, as the user is told.
pub(super) fn wait(reset: Option<u64>) -> u64 {
    match reset {
        Some(s) if s <= MAX_WAIT => s.max(1),
        _ => DEFAULT_WAIT,
    }
}

/// R12 The HTTP status alone: `Ok` for 2xx, else the matching error. Shared by reads and edits.
pub(crate) fn status_gate(status: u16, reset: Option<u64>) -> Result<(), Error> {
    match status {
        401 => Err(Error::SignInAgain),
        403 => Err(Error::NotAllowed),
        429 => Err(Error::RateLimited { seconds: wait(reset) }),
        500.. => Err(Error::Down),
        200..=299 => Ok(()),
        _ => Err(Error::Bad),
    }
}

/// R12 The GQL `data`, taken out of the answer; missing or null is `Bad`.
pub(crate) fn take_data(v: &mut Value) -> Result<Value, Error> {
    match v.get_mut("data") {
        Some(d) if !d.is_null() => Ok(d.take()),
        _ => Err(Error::Bad),
    }
}

/// Turns a status, the reset hint and a body into the GQL `data`, or the matching error.
pub fn classify(status: u16, reset: Option<u64>, body: &[u8]) -> Result<Value, Error> {
    status_gate(status, reset)?;
    let mut v: Value = serde_json::from_slice(body).map_err(|_| Error::Bad)?;
    if let Some(first) = v.pointer("/errors/0") {
        let code = first
            .pointer("/extensions/code")
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(match code {
            "LOGIN_REQUIRED" => Error::SignInAgain,
            "LACKING_PRIVILEGES" | "LOAD_ERROR" => Error::NotAllowed,
            "RATE_LIMIT_EXCEEDED" => Error::RateLimited { seconds: wait(reset) },
            _ => Error::Bad,
        });
    }
    take_data(&mut v)
}

/// A transport error from the shared client: nothing worth showing beyond "down".
pub fn transport(_: cyex_twitch::Error) -> Error {
    Error::Down
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reset_time_is_not_a_wait() {
        let limited = |reset| classify(429, reset, b"{}");
        assert_eq!(limited(Some(7)), Err(Error::RateLimited { seconds: 7 }));
        assert_eq!(limited(Some(0)), Err(Error::RateLimited { seconds: 1 }));
        assert_eq!(limited(Some(1_760_000_000)), Err(Error::RateLimited { seconds: 30 }));
        assert_eq!(limited(None), Err(Error::RateLimited { seconds: 30 }));
    }
}
