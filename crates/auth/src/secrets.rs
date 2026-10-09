//! Token storage. Real tokens go to the Windows Credential Manager, one entry per account
//! and token kind. Tests use the in-memory store.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const SERVICE: &str = "cyex-viewer";

/// What's kept in the credential store for one token.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Secret {
    pub access: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<String>,
    /// The client the token was issued to. Helix and GQL calls must send it.
    pub client_id: String,
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret { .. }")
    }
}

pub trait SecretStore: Send + Sync {
    fn load(&self, key: &str) -> Result<Option<String>, String>;
    fn save(&self, key: &str, value: &str) -> Result<(), String>;
    fn delete(&self, key: &str) -> Result<(), String>;
}

pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn load(&self, key: &str) -> Result<Option<String>, String> {
        match keyring::Entry::new(SERVICE, key).and_then(|e| e.get_password()) {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn save(&self, key: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, key)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        match keyring::Entry::new(SERVICE, key).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl MemoryStore {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl SecretStore for MemoryStore {
    fn load(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.map().get(key).cloned())
    }

    fn save(&self, key: &str, value: &str) -> Result<(), String> {
        self.map().insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        self.map().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_never_show_in_debug_output() {
        let s = Secret {
            access: "topsecret".into(),
            refresh: None,
            client_id: "c".into(),
        };
        assert!(!format!("{s:?}").contains("topsecret"));
    }

    /// Touches the real Credential Manager: `cargo test -p cyex-auth -- --ignored`.
    #[test]
    #[ignore]
    fn keyring_round_trip() {
        let store = KeyringStore;
        let key = "selftest.helix";
        store.save(key, "value").unwrap();
        assert_eq!(store.load(key).unwrap().as_deref(), Some("value"));
        store.delete(key).unwrap();
        assert_eq!(store.load(key).unwrap(), None);
        store.delete(key).unwrap();
    }

    #[test]
    fn stored_form_is_small_enough_for_the_credential_manager() {
        // The Windows limit is 2560 bytes of UTF-16, so 1280 characters.
        let s = Secret {
            access: "a".repeat(30),
            refresh: Some("r".repeat(50)),
            client_id: "c".repeat(30),
        };
        assert!(serde_json::to_string(&s).unwrap().len() < 400);
    }
}
