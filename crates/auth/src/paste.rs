//! Pulls a token out of whatever the user pasted (1.4): a bare token, `oauth:...`, an
//! `Authorization` header, Chatterino's `oauth_token=...;client_id=...` line, or a
//! redirect URL with `#access_token=...`.

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
    use super::token_from;

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
