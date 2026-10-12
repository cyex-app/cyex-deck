//! Pulls a token out of whatever the user pasted: a bare token, `oauth:...`, an
//! `Authorization` header, Chatterino's `oauth_token=...;client_id=...` line, or a
//! redirect URL with `#access_token=...`. Then checks it against the account it's added
//! to (V102-19).

use cyex_twitch::clients;
use cyex_twitch::id::Validation;

use crate::AuthError;

/// V102-19 A GQL token pasted for an account signed in with Twitch: it has to be made for
/// the GQL client ID and be that account's. The name and user ID come from the account.
pub fn check_gql(v: &Validation, user_id: &str) -> Result<(), AuthError> {
    if v.client_id != clients::GQL {
        return Err(AuthError::NotGql);
    }
    same_user(v, user_id)
}

/// V102-19 A token added to an existing account (pasted, or from a code or Twitch's
/// sign-in) must be that account's: one account then holds both sign-ins.
pub fn same_user(v: &Validation, user_id: &str) -> Result<(), AuthError> {
    if v.user_id == user_id {
        Ok(())
    } else {
        Err(AuthError::OtherUser(v.login.clone()))
    }
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

    fn validation(client_id: &str, user_id: &str) -> Validation {
        Validation {
            client_id: client_id.into(),
            login: "someone".into(),
            user_id: user_id.into(),
            scopes: None,
            expires_in: 0,
        }
    }

    #[test]
    fn a_gql_token_is_for_the_gql_client_and_the_same_user() {
        assert!(check_gql(&validation(clients::GQL, "42"), "42").is_ok());
        // Made for another app, even Twitch's website.
        assert!(matches!(check_gql(&validation(clients::WEB, "42"), "42"), Err(AuthError::NotGql)));
        assert!(matches!(check_gql(&validation("someappclientid000000000000", "42"), "42"), Err(AuthError::NotGql)));
        // Someone else's token.
        assert!(matches!(check_gql(&validation(clients::GQL, "7"), "42"), Err(AuthError::OtherUser(l)) if l == "someone"));
    }

    #[test]
    fn a_code_approved_by_someone_else_is_refused() {
        assert!(same_user(&validation("app", "42"), "42").is_ok());
        assert!(matches!(same_user(&validation("app", "7"), "42"), Err(AuthError::OtherUser(_))));
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
