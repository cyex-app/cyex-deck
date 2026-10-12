//! Token storage. Real tokens go to the Windows Credential Manager, one entry per account
//! and token kind. Tests use the in-memory store. Entries saved under the app's old name
//! are copied to the new one the first time they're read (V102-02).

use std::collections::HashMap;
use std::sync::Mutex;

use cyex_core::app::{OLD_SLUG, SLUG};
use serde::{Deserialize, Serialize};

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

/// The Credential Manager under the app's name. Reads fall back to the old name once
/// (V102-02).
pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn load(&self, key: &str) -> Result<Option<String>, String> {
        load_or_copy(&Service(SLUG), &Service(OLD_SLUG), key)
    }

    fn save(&self, key: &str, value: &str) -> Result<(), String> {
        Service(SLUG).save(key, value)
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        delete_both(&Service(SLUG), &Service(OLD_SLUG), key)
    }
}

/// One Credential Manager service name. The entries' target names hold it.
struct Service(&'static str);

impl SecretStore for Service {
    fn load(&self, key: &str) -> Result<Option<String>, String> {
        match keyring::Entry::new(self.0, key).and_then(|e| e.get_password()) {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn save(&self, key: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(self.0, key)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        match keyring::Entry::new(self.0, key).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// V102-02 The entry under the new name, else the old name's, copied over the first time
/// it's needed. The old entry stays.
fn load_or_copy(new: &dyn SecretStore, old: &dyn SecretStore, key: &str) -> Result<Option<String>, String> {
    if let Some(value) = new.load(key)? {
        return Ok(Some(value));
    }
    let Some(value) = old.load(key)? else {
        return Ok(None);
    };
    new.save(key, &value)?;
    Ok(Some(value))
}

/// A token (or the proxy) removed in the app goes under the old name too, so the next
/// read doesn't copy it back.
fn delete_both(new: &dyn SecretStore, old: &dyn SecretStore, key: &str) -> Result<(), String> {
    new.delete(key)?;
    old.delete(key)
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
    fn old_entries_are_copied_once_and_go_when_deleted() {
        let (new, old) = (MemoryStore::default(), MemoryStore::default());
        old.save("1.gql", "from before").unwrap();
        assert_eq!(load_or_copy(&new, &old, "1.gql").unwrap().as_deref(), Some("from before"));
        // Copied, and the old entry stays.
        assert_eq!(new.load("1.gql").unwrap().as_deref(), Some("from before"));
        assert_eq!(old.load("1.gql").unwrap().as_deref(), Some("from before"));
        // Once there's a new entry, it wins.
        new.save("1.gql", "refreshed").unwrap();
        assert_eq!(load_or_copy(&new, &old, "1.gql").unwrap().as_deref(), Some("refreshed"));
        // A removed token doesn't come back from the old name.
        delete_both(&new, &old, "1.gql").unwrap();
        assert_eq!(load_or_copy(&new, &old, "1.gql").unwrap(), None);
        assert_eq!(load_or_copy(&new, &old, "2.helix").unwrap(), None);
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
