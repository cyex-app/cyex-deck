//! V107-R38 Your channel's moderation settings (Twitch's Settings › Moderation) for the
//! stream manager's panel. GQL only, as Twitch's dashboard: Helix has AutoMod and chat
//! settings but not verification, rules, unban requests or permitted terms. V107-R41 no
//! read before a write (PERF-03), calls through `gql_call`.

use cyex_twitch::Error;
use cyex_twitch::gql::dashboard::modset::{self as ms, Change, ModSettings, TERM_MAX, TERM_MIN};
use tauri::AppHandle;

use super::gql_call::gql_call;
use super::words;

/// Twitch's answer as a line; a term on the list already first.
fn say(e: &Error, permitted: bool) -> String {
    match e {
        Error::Refused(c) if c.contains("DUPLICATE") || c.contains("ALREADY") => already(permitted),
        e => words::gql(e),
    }
}

fn already(permitted: bool) -> String {
    if permitted { "That term is allowed already." } else { "That term is blocked already." }.into()
}

pub async fn read(app: &AppHandle) -> Result<ModSettings, String> {
    let c = gql_call(app)?;
    ms::read(&c.channel()).await.map_err(|e| words::gql(&e))
}

pub async fn set(app: &AppHandle, change: Change) -> Result<ModSettings, String> {
    let c = gql_call(app)?;
    ms::set(&c.channel(), &change).await.map_err(|e| words::gql(&e))
}

/// Why a term can't go on a list, before asking Twitch: its length, and the list the panel
/// shows (`have`, when it sent one). Twitch's own duplicate code covers the rest.
pub fn check(text: &str, have: &[String], permitted: bool) -> Result<String, String> {
    let t = text.trim();
    let n = t.chars().count();
    if n < TERM_MIN {
        return Err(format!("A term needs at least {TERM_MIN} characters."));
    }
    if n > TERM_MAX {
        return Err(format!("A term can have up to {TERM_MAX} characters."));
    }
    if have.iter().any(|h| h.trim().eq_ignore_ascii_case(t)) {
        return Err(already(permitted));
    }
    Ok(t.to_owned())
}

/// Adds a blocked or permitted term and reads everything back.
pub async fn add_term(app: &AppHandle, permitted: bool, text: &str, have: &[String]) -> Result<ModSettings, String> {
    let t = check(text, have, permitted)?;
    let c = gql_call(app)?;
    ms::add_term(&c.channel(), permitted, &t).await.map_err(|e| say(&e, permitted))?;
    read(app).await
}

pub async fn remove_term(app: &AppHandle, permitted: bool, id: &str) -> Result<ModSettings, String> {
    let c = gql_call(app)?;
    ms::remove_term(&c.channel(), permitted, id).await.map_err(|e| words::gql(&e))?;
    read(app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_each_list() {
        assert_eq!(check(" gg ", &[], true), Ok("gg".into()));
        assert!(check("a", &[], true).unwrap_err().contains("at least 2"));
        assert!(check(&"x".repeat(501), &[], true).unwrap_err().contains("500"));
        assert!(check("GG", &["gg".into()], true).unwrap_err().contains("allowed"));
        assert!(check("GG", &["gg".into()], false).unwrap_err().contains("blocked"));
    }

    #[test]
    fn v107_r41_twitchs_duplicate_code_reads_as_a_line() {
        assert_eq!(say(&Error::Refused("DUPLICATE_TERM".into()), false), "That term is blocked already.");
        assert_eq!(say(&Error::Refused("TERM_ALREADY_EXISTS".into()), true), "That term is allowed already.");
        assert_eq!(say(&Error::Refused("FORBIDDEN".into()), true), "Twitch didn’t allow that.");
    }
}
