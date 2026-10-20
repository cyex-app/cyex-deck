//! V107-R31 The mod view's blocked terms (1.48): list, add, remove. Helix only (needs
//! `moderator:read:blocked_terms` / `moderator:manage:blocked_terms`); there's no GQL
//! operation for them in our research yet. Twitch has no expiry when adding one.

use cyex_twitch::helix::moderation::{self as hm, BlockedTerm};
use tauri::AppHandle;

use super::{codes, http};

const READ: &str = "moderator:read:blocked_terms";
const MANAGE: &str = "moderator:manage:blocked_terms";

fn err(e: cyex_twitch::Error) -> String {
    match &e {
        cyex_twitch::Error::Api { status, message } => codes::helix(*status, message),
        _ => e.to_string(),
    }
}

/// Why a term can't be added, before asking Twitch: 2 to 500 characters, not on the list.
pub fn check(text: &str, list: &[BlockedTerm]) -> Result<String, String> {
    let t = text.trim();
    let n = t.chars().count();
    if n < 2 {
        return Err("A term needs at least 2 characters.".into());
    }
    if n > 500 {
        return Err("A term can have up to 500 characters.".into());
    }
    if list.iter().any(|b| b.text.eq_ignore_ascii_case(t)) {
        return Err("That term is blocked already.".into());
    }
    Ok(t.to_owned())
}

pub async fn list(app: &AppHandle, account: &str, login: &str) -> Result<Vec<BlockedTerm>, String> {
    let actor = super::actor(app, account, login)?;
    let h = actor.helix_scoped(app, READ).ok_or("Blocked terms need Sign in with Twitch.")?;
    hm::blocked_terms(&http(app), h, actor.room()).await.map_err(err)
}

pub async fn add(app: &AppHandle, account: &str, login: &str, text: &str) -> Result<BlockedTerm, String> {
    let actor = super::actor(app, account, login)?;
    let h = actor.helix_scoped(app, MANAGE).ok_or("Blocked terms need Sign in with Twitch.")?;
    let now = hm::blocked_terms(&http(app), h, actor.room()).await.map_err(err)?;
    let text = check(text, &now)?;
    hm::add_blocked_term(&http(app), h, actor.room(), &text).await.map_err(err)
}

pub async fn remove(app: &AppHandle, account: &str, login: &str, id: &str) -> Result<(), String> {
    let actor = super::actor(app, account, login)?;
    let h = actor.helix_scoped(app, MANAGE).ok_or("Blocked terms need Sign in with Twitch.")?;
    hm::remove_blocked_term(&http(app), h, actor.room(), id).await.map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(text: &str) -> BlockedTerm {
        BlockedTerm { id: "1".into(), text: text.into(), created_at: String::new(), expires_at: None }
    }

    #[test]
    fn checks_length_and_duplicates() {
        assert_eq!(check("  spam  ", &[]), Ok("spam".into()));
        assert!(check("a", &[]).unwrap_err().contains("at least 2"));
        assert!(check(&"x".repeat(501), &[]).unwrap_err().contains("up to 500"));
        assert!(check(&"x".repeat(500), &[]).is_ok());
        assert!(check("SPAM", &[term("spam")]).unwrap_err().contains("already"));
    }
}
