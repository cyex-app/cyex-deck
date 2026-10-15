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

/// Turns a status, the reset hint and a body into the GQL `data`, or the matching error.
pub fn classify(status: u16, reset: Option<u64>, body: &[u8]) -> Result<Value, Error> {
    match status {
        401 => return Err(Error::SignInAgain),
        403 => return Err(Error::NotAllowed),
        429 => {
            return Err(Error::RateLimited {
                seconds: reset.unwrap_or(DEFAULT_WAIT),
            });
        }
        500.. => return Err(Error::Down),
        200..=299 => {}
        _ => return Err(Error::Bad),
    }
    let mut v: Value = serde_json::from_slice(body).map_err(|_| Error::Bad)?;
    if let Some(first) = v.pointer("/errors/0") {
        let code = first
            .pointer("/extensions/code")
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(match code {
            "LOGIN_REQUIRED" => Error::SignInAgain,
            "LACKING_PRIVILEGES" | "LOAD_ERROR" => Error::NotAllowed,
            "RATE_LIMIT_EXCEEDED" => Error::RateLimited {
                seconds: reset.unwrap_or(DEFAULT_WAIT),
            },
            _ => Error::Bad,
        });
    }
    match v.get_mut("data") {
        Some(d) if !d.is_null() => Ok(d.take()),
        _ => Err(Error::Bad),
    }
}

/// A transport error from the shared client: nothing worth showing beyond "down".
pub fn transport(_: cyex_twitch::Error) -> Error {
    Error::Down
}
