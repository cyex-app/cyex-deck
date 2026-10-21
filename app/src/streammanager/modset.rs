//! V107-R38 Your channel's moderation settings (Twitch's Settings › Moderation) for the
//! stream manager's panel. GQL only, as Twitch's dashboard: Helix has AutoMod and chat
//! settings but not verification, rules, unban requests or permitted terms.

use cyex_twitch::gql::dashboard::modset::{self as ms, Change, ModSettings};
use cyex_twitch::helix::moderation::BlockedTerm;
use tauri::AppHandle;

use super::{Call, words};

const GQL_ONLY: &str = "Moderation settings need the sign-in with a code (Settings → Accounts).";

fn call(app: &AppHandle) -> Result<Call, String> {
    let c = Call::new(app)?;
    if c.has_gql() { Ok(c) } else { Err(GQL_ONLY.into()) }
}

pub async fn read(app: &AppHandle) -> Result<ModSettings, String> {
    let c = call(app)?;
    ms::read(&c.http, c.gql(), &c.channel_id)
        .await
        .map_err(|e| words::gql(&e))
}

pub async fn set(app: &AppHandle, change: Change) -> Result<ModSettings, String> {
    let c = call(app)?;
    ms::set(&c.http, c.gql(), &c.channel_id, &change)
        .await
        .map_err(|e| words::gql(&e))
}

/// Why a term can't go on a list, before asking Twitch: 2 to 500 characters, not on it yet.
pub fn check(text: &str, list: &[BlockedTerm], permitted: bool) -> Result<String, String> {
    let t = crate::moderation::terms::check(text, &[])?;
    if list.iter().any(|b| b.text.eq_ignore_ascii_case(&t)) {
        return Err(if permitted {
            "That term is allowed already."
        } else {
            "That term is blocked already."
        }
        .into());
    }
    Ok(t)
}

/// Adds a blocked or permitted term and reads everything back.
pub async fn add_term(app: &AppHandle, permitted: bool, text: &str) -> Result<ModSettings, String> {
    let c = call(app)?;
    let now = ms::read(&c.http, c.gql(), &c.channel_id)
        .await
        .map_err(|e| words::gql(&e))?;
    let t = check(
        text,
        if permitted { &now.permitted } else { &now.blocked },
        permitted,
    )?;
    ms::add_term(&c.http, c.gql(), &c.channel_id, permitted, &t)
        .await
        .map_err(|e| words::gql(&e))?;
    read(app).await
}

pub async fn remove_term(app: &AppHandle, permitted: bool, id: &str) -> Result<ModSettings, String> {
    let c = call(app)?;
    ms::remove_term(&c.http, c.gql(), &c.channel_id, permitted, id)
        .await
        .map_err(|e| words::gql(&e))?;
    read(app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(text: &str) -> BlockedTerm {
        BlockedTerm {
            id: "1".into(),
            text: text.into(),
            created_at: String::new(),
            expires_at: None,
        }
    }

    #[test]
    fn checks_each_list() {
        assert_eq!(check(" gg ", &[], true), Ok("gg".into()));
        assert!(check("a", &[], true).unwrap_err().contains("at least 2"));
        assert!(check("GG", &[term("gg")], true).unwrap_err().contains("allowed"));
        assert!(check("GG", &[term("gg")], false).unwrap_err().contains("blocked"));
    }
}
