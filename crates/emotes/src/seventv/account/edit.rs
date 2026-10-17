//! V106-31/32 The three set mutations and what 7TV's errors mean for them (research-phase-5 §2).

use std::fmt;

use serde_json::{json, Value};

use super::error::{wait, Error};
use super::set_query::{ADD, REMOVE, RENAME};
use super::Client;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    SignInAgain,
    NotAllowed,
    RateLimited { seconds: u64 },
    Down,
    Full,
    NameTaken,
    Private,
    NotFound,
    BadName,
    Bad,
}

impl From<Error> for EditError {
    fn from(e: Error) -> Self {
        match e {
            Error::SignInAgain => EditError::SignInAgain,
            Error::NotAllowed => EditError::NotAllowed,
            Error::RateLimited { seconds } => EditError::RateLimited { seconds },
            Error::Down => EditError::Down,
            Error::Bad => EditError::Bad,
        }
    }
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EditError::SignInAgain => "sign in again",
            EditError::NotAllowed => "not allowed",
            EditError::RateLimited { .. } => "rate limited",
            EditError::Down => "7TV is down",
            EditError::Full => "set is full",
            EditError::NameTaken => "name taken",
            EditError::Private => "emote is private",
            EditError::NotFound => "not found",
            EditError::BadName => "bad name",
            EditError::Bad => "unexpected answer",
        })
    }
}

impl std::error::Error for EditError {}

const RESET: &str = "x-ratelimit-emote_set_change-reset";

/// HTTP first (like `error::classify`), then the first GQL error, else `data`.
pub fn classify_edit(status: u16, reset: Option<u64>, body: &[u8]) -> Result<Value, EditError> {
    match status {
        401 => return Err(EditError::SignInAgain),
        403 => return Err(EditError::NotAllowed),
        429 => return Err(EditError::RateLimited { seconds: wait(reset) }),
        500.. => return Err(EditError::Down),
        200..=299 => {}
        _ => return Err(EditError::Bad),
    }
    let mut v: Value = serde_json::from_slice(body).map_err(|_| EditError::Bad)?;
    if let Some(first) = v.pointer("/errors/0") {
        return Err(field_error(first, reset));
    }
    match v.get_mut("data") {
        Some(d) if !d.is_null() => Ok(d.take()),
        _ => Err(EditError::Bad),
    }
}

fn field_error(e: &Value, reset: Option<u64>) -> EditError {
    let ext = e.get("extensions");
    let code = ext
        .and_then(|x| x.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let status = ext
        .and_then(|x| x.get("status"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let text = ext
        .and_then(|x| x.get("message"))
        .or_else(|| e.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    match code {
        "LOGIN_REQUIRED" => EditError::SignInAgain,
        "LACKING_PRIVILEGES" => EditError::NotAllowed,
        "RATE_LIMIT_EXCEEDED" => EditError::RateLimited {
            seconds: wait(header_reset(ext).or(reset)),
        },
        "LOAD_ERROR" if text.contains("capacity") => EditError::Full,
        "LOAD_ERROR" if status == 404 => EditError::NotFound,
        "LOAD_ERROR" => EditError::NotAllowed,
        "BAD_REQUEST" if status == 409 || text.contains("conflict") => EditError::NameTaken,
        "BAD_REQUEST" if text.contains("private") => EditError::Private,
        "BAD_REQUEST" if text.contains("personal") => EditError::NotAllowed,
        "BAD_REQUEST" if status == 404 => EditError::NotFound,
        "TRANSACTION_ERROR" | "INTERNAL_SERVER_ERROR" => EditError::Down,
        "" if text.contains("invalid emote alias") => EditError::BadName,
        _ => EditError::Bad,
    }
}

/// `extensions.headers` is an object of strings.
fn header_reset(ext: Option<&Value>) -> Option<u64> {
    ext?.get("headers")?.get(RESET)?.as_str()?.trim().parse().ok()
}

impl Client {
    pub async fn add_emote(&self, token: &str, set: &str, emote: &str, alias: &str) -> Result<(), EditError> {
        let vars = json!({ "setId": set, "emote": { "emoteId": emote, "alias": alias } });
        self.edit(token, ADD, vars, "/emoteSets/emoteSet/addEmote").await
    }

    /// With the alias given, only that entry goes (V105-04).
    pub async fn remove_emote(
        &self,
        token: &str,
        set: &str,
        emote: &str,
        alias: &str,
    ) -> Result<(), EditError> {
        let vars = json!({ "setId": set, "emote": { "emoteId": emote, "alias": alias } });
        self.edit(token, REMOVE, vars, "/emoteSets/emoteSet/removeEmote")
            .await
    }

    pub async fn rename_emote(
        &self,
        token: &str,
        set: &str,
        emote: &str,
        from: &str,
        to: &str,
    ) -> Result<(), EditError> {
        let vars = json!({ "setId": set, "emote": { "emoteId": emote, "alias": from }, "alias": to });
        self.edit(token, RENAME, vars, "/emoteSets/emoteSet/updateEmoteAlias")
            .await
    }

    async fn edit(&self, token: &str, query: &str, vars: Value, path: &str) -> Result<(), EditError> {
        let (status, reset, body) = self.post(Some(token), query, vars).await?;
        let data = classify_edit(status, reset, &body)?;
        data.pointer(path)
            .filter(|v| !v.is_null())
            .map(|_| ())
            .ok_or(EditError::Bad)
    }
}
