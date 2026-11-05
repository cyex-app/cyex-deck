//! V109-B02 cyex sessions (api.cyex.app), one per account, made from the account's Twitch
//! token. Kept in the same store as the Twitch tokens (`<twitch id>.cyex`), never in settings,
//! never printed.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::secrets::SecretStore;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CyexSession {
    pub token: String,
    /// As the server sent it (RFC 3339); only shown, never trusted for timing.
    #[serde(default)]
    pub expires: String,
}

impl std::fmt::Debug for CyexSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CyexSession(..)")
    }
}

impl CyexSession {
    /// A session token is short printable text; anything else is refused.
    pub fn new(token: &str, expires: &str) -> Option<Self> {
        let ok = !token.is_empty() && token.len() <= 512 && token.bytes().all(|b| b.is_ascii_graphic());
        ok.then(|| Self {
            token: token.to_owned(),
            expires: expires.chars().take(40).collect(),
        })
    }
}

pub struct CyexSessions {
    store: Arc<dyn SecretStore>,
}

impl CyexSessions {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self { store }
    }

    fn key(twitch_id: &str) -> String {
        format!("{twitch_id}.cyex")
    }

    pub fn get(&self, twitch_id: &str) -> Option<CyexSession> {
        let raw = self.store.load(&Self::key(twitch_id)).ok()??;
        serde_json::from_str(&raw).ok()
    }

    pub fn put(&self, twitch_id: &str, s: &CyexSession) -> Result<(), String> {
        let raw = serde_json::to_string(s).map_err(|e| e.to_string())?;
        self.store.save(&Self::key(twitch_id), &raw)
    }

    pub fn forget(&self, twitch_id: &str) -> Result<(), String> {
        self.store.delete(&Self::key(twitch_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;

    #[test]
    fn put_get_forget_use_the_account_key() {
        let store = Arc::new(MemoryStore::default());
        let s = CyexSessions::new(store.clone());
        let one = CyexSession::new("abc.def", "2026-12-05T00:00:00Z").unwrap();
        assert!(s.get("1").is_none());
        s.put("1", &one).unwrap();
        assert!(store.load("1.cyex").unwrap().is_some());
        assert_eq!(s.get("1"), Some(one));
        s.forget("1").unwrap();
        assert!(s.get("1").is_none());
    }

    #[test]
    fn junk_is_refused_and_debug_hides_the_token() {
        assert!(CyexSession::new("", "").is_none());
        assert!(CyexSession::new("a b", "").is_none());
        assert!(CyexSession::new(&"x".repeat(600), "").is_none());
        let s = CyexSession::new("secret-token", "").unwrap();
        assert!(!format!("{s:?}").contains("secret"));
        let store = Arc::new(MemoryStore::default());
        store.save("2.cyex", "not json").unwrap();
        assert!(CyexSessions::new(store).get("2").is_none());
    }
}
