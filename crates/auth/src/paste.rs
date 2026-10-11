//! Pulls a token out of whatever the user pasted (1.4): a bare token, `oauth:...`, an
//! `Authorization` header, Chatterino's `oauth_token=...;client_id=...` line, or a
//! redirect URL with `#access_token=...`. Also checks the manual screen's other fields
//! against what Twitch says about the token (V101-08).

use cyex_twitch::id::Validation;
use serde::Deserialize;
use ts_rs::TS;

use crate::AuthError;

/// V101-08 The manual screen, the same fields Chatterino asks for. Empty ones are taken
/// from Twitch. No `Debug`: it holds the token.
#[derive(Clone, Default, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct ManualToken {
    pub login: String,
    pub user_id: String,
    pub client_id: String,
    pub token: String,
}

/// The filled-in fields must be the token's own: a token only works with the client ID it
/// was made for, and the account is the token's user.
pub fn check(m: &ManualToken, v: &Validation) -> Result<(), AuthError> {
    let login = m.login.trim().trim_start_matches('@');
    if !login.is_empty() && !login.eq_ignore_ascii_case(&v.login) {
        return Err(AuthError::OtherUser(v.login.clone()));
    }
    let user_id = m.user_id.trim();
    if !user_id.is_empty() && user_id != v.user_id {
        return Err(AuthError::OtherUser(v.login.clone()));
    }
    let client_id = m.client_id.trim();
    if !client_id.is_empty() && client_id != v.client_id {
        return Err(AuthError::OtherClient(v.client_id.clone()));
    }
    Ok(())
}

const KEYS: [&str; 3] = ["oauth_token", "access_token", "token"];
const PREFIXES: [&str; 4] = ["authorization:", "oauth:", "oauth ", "bearer "];

pub fn token_from(input: &str) -> Option<String> {
    let text = input.trim();
    for part in text.split([';', '&', '#', '?', '\n']) {
        if let Some((key, value)) = part.split_once('=')
            && KEYS.contains(&key.trim().to_ascii_lowercase().as_str())
        {
            return clean(value);
        }
    }
    let mut rest = text;
    loop {
        let lower = rest.to_ascii_lowercase();
        match PREFIXES.iter().find(|p| lower.starts_with(*p)) {
            Some(p) => rest = rest[p.len()..].trim_start(),
            None => break,
        }
    }
    clean(rest)
}

/// Twitch tokens are 30 lowercase letters and digits. Allow some slack.
fn clean(value: &str) -> Option<String> {
    let v = value.trim().trim_matches(['"', '\'']);
    let ok = (20..=64).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_alphanumeric());
    ok.then(|| v.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validation() -> Validation {
        Validation {
            client_id: "clientid123".into(),
            login: "someone".into(),
            user_id: "42".into(),
            scopes: None,
            expires_in: 0,
        }
    }

    fn manual(login: &str, user_id: &str, client_id: &str) -> ManualToken {
        ManualToken {
            login: login.into(),
            user_id: user_id.into(),
            client_id: client_id.into(),
            token: String::new(),
        }
    }

    #[test]
    fn manual_fields_match_the_token_or_are_empty() {
        let v = validation();
        assert!(check(&manual("", "", ""), &v).is_ok());
        assert!(check(&manual(" @SomeOne ", "42", "clientid123"), &v).is_ok());
        assert!(matches!(check(&manual("other", "", ""), &v), Err(AuthError::OtherUser(l)) if l == "someone"));
        assert!(matches!(check(&manual("", "7", ""), &v), Err(AuthError::OtherUser(_))));
        assert!(matches!(check(&manual("", "", "nope"), &v), Err(AuthError::OtherClient(c)) if c == "clientid123"));
    }

    const T: &str = "abcdefghijklmnopqrstuvwxyz0123";

    #[test]
    fn bare_and_prefixed_tokens() {
        assert_eq!(token_from(T).as_deref(), Some(T));
        assert_eq!(token_from(&format!("  oauth:{T}\n")).as_deref(), Some(T));
        assert_eq!(token_from(&format!("OAuth {T}")).as_deref(), Some(T));
        assert_eq!(
            token_from(&format!("Authorization: Bearer {T}")).as_deref(),
            Some(T)
        );
    }

    #[test]
    fn chatterino_line() {
        let line = format!("username=someone;user_id=123;client_id=xyz;oauth_token={T};");
        assert_eq!(token_from(&line).as_deref(), Some(T));
    }

    #[test]
    fn redirect_url() {
        let url = format!("http://localhost/#access_token={T}&scope=chat%3Aread&token_type=bearer");
        assert_eq!(token_from(&url).as_deref(), Some(T));
    }

    #[test]
    fn rejects_junk() {
        assert_eq!(token_from(""), None);
        assert_eq!(token_from("hello"), None);
        assert_eq!(token_from("not a token at all, just a sentence"), None);
        assert_eq!(token_from("oauth_token=short"), None);
    }
}
