//! Tokens in memory, backed by the credential store, plus each token's live state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_settings::schema::Accounts;
use serde::Serialize;
use ts_rs::TS;

use crate::TokenKind;
use crate::secrets::{Secret, SecretStore};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TokenState {
    /// Loaded, not checked with Twitch yet.
    Checking,
    Valid,
    /// Twitch couldn't be reached. Checked again soon.
    Offline,
    /// Expired or revoked: needs a new sign-in.
    Invalid,
    /// Not in the credential store, e.g. after restoring settings on another PC.
    Missing,
}

impl TokenState {
    /// Worth using until Twitch says otherwise.
    pub fn usable(self) -> bool {
        matches!(
            self,
            TokenState::Checking | TokenState::Valid | TokenState::Offline
        )
    }
}

struct Slot {
    secret: Secret,
    state: TokenState,
    checked: Option<Instant>,
    /// V107-A1 In memory only: the Credential Manager write failed. `save_pending` retries.
    unsaved: bool,
}

type Key = (String, TokenKind);

pub struct Vault {
    store: Arc<dyn SecretStore>,
    slots: Mutex<HashMap<Key, Slot>>,
}

impl Vault {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self {
            store,
            slots: Mutex::default(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<Key, Slot>> {
        self.slots.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Reads every token the accounts list mentions. Tokens that didn't change keep their
    /// state. Returns read errors, if any.
    pub fn load(&self, accounts: &Accounts) -> Vec<String> {
        let mut errors = Vec::new();
        let mut old = std::mem::take(&mut *self.lock());
        let mut slots = HashMap::new();
        for account in &accounts.list {
            for kind in TokenKind::ALL.into_iter().filter(|k| k.meta(account).is_some()) {
                let key = (account.user_id.clone(), kind);
                // V107-A1 A refreshed token that couldn't be saved is newer than the stored
                // one (whose refresh token is spent), or than none at all.
                if let Some(prev) = old.remove(&key) {
                    if prev.unsaved {
                        slots.insert(key, prev);
                        continue;
                    }
                    old.insert(key.clone(), prev);
                }
                let secret = match self.store.load(&store_key(&account.user_id, kind)) {
                    Ok(Some(json)) => match serde_json::from_str::<Secret>(&json) {
                        Ok(secret) => secret,
                        // S1 Position only: serde's text can echo a stored value.
                        Err(e) => {
                            errors.push(format!(
                                "{}: stored token unreadable (line {} col {})",
                                store_key(&account.user_id, kind),
                                e.line(),
                                e.column()
                            ));
                            continue;
                        }
                    },
                    Ok(None) => continue,
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };
                let slot = match old.remove(&key) {
                    Some(prev) if prev.secret == secret => prev,
                    _ => Slot {
                        secret,
                        state: TokenState::Checking,
                        checked: None,
                        unsaved: false,
                    },
                };
                slots.insert(key, slot);
            }
        }
        *self.lock() = slots;
        errors
    }

    /// Saves a token that was just checked with Twitch.
    pub fn put(&self, user_id: &str, kind: TokenKind, secret: Secret) -> Result<(), String> {
        let json = serde_json::to_string(&secret).map_err(|e| e.to_string())?;
        self.store.save(&store_key(user_id, kind), &json)?;
        let slot = Slot {
            secret,
            state: TokenState::Valid,
            checked: Some(Instant::now()),
            unsaved: false,
        };
        self.lock().insert((user_id.to_owned(), kind), slot);
        Ok(())
    }

    /// V107-A1 Keeps a token Twitch just handed out, with `state`. Unlike [`Vault::put`] the
    /// memory slot always takes it, since the old refresh token is spent; a failed
    /// Credential Manager write is returned and retried by [`Vault::save_pending`].
    pub fn keep(&self, user_id: &str, kind: TokenKind, secret: Secret, state: TokenState) -> Result<(), String> {
        let saved = serde_json::to_string(&secret)
            .map_err(|e| e.to_string())
            .and_then(|json| self.store.save(&store_key(user_id, kind), &json));
        let slot = Slot {
            secret,
            state,
            checked: Some(Instant::now()),
            unsaved: saved.is_err(),
        };
        self.lock().insert((user_id.to_owned(), kind), slot);
        saved
    }

    /// V107-A1 Tries again to save tokens that only live in memory. Returns the errors, by
    /// key (never the token).
    pub fn save_pending(&self) -> Vec<String> {
        let pending: Vec<(Key, Secret)> = self
            .lock()
            .iter()
            .filter(|(_, s)| s.unsaved)
            .map(|(k, s)| (k.clone(), s.secret.clone()))
            .collect();
        let mut errors = Vec::new();
        for ((user_id, kind), secret) in pending {
            let key = store_key(&user_id, kind);
            let saved = serde_json::to_string(&secret)
                .map_err(|e| e.to_string())
                .and_then(|json| self.store.save(&key, &json));
            match saved {
                Ok(()) => {
                    // Only if the slot still holds what was saved (a sign-in may have replaced it).
                    if let Some(slot) = self.lock().get_mut(&(user_id, kind))
                        && slot.secret == secret
                    {
                        slot.unsaved = false;
                    }
                }
                Err(e) => errors.push(format!("{key}: {e}")),
            }
        }
        errors
    }

    /// Deletes the token. Returns it so the caller can revoke it.
    pub fn forget(&self, user_id: &str, kind: TokenKind) -> Result<Option<Secret>, String> {
        self.store.delete(&store_key(user_id, kind))?;
        Ok(self.lock().remove(&(user_id.to_owned(), kind)).map(|s| s.secret))
    }

    pub fn secret(&self, user_id: &str, kind: TokenKind) -> Option<Secret> {
        self.lock()
            .get(&(user_id.to_owned(), kind))
            .map(|s| s.secret.clone())
    }

    pub fn state(&self, user_id: &str, kind: TokenKind) -> TokenState {
        self.lock()
            .get(&(user_id.to_owned(), kind))
            .map_or(TokenState::Missing, |s| s.state)
    }

    pub fn since_check(&self, user_id: &str, kind: TokenKind) -> Option<Duration> {
        self.lock()
            .get(&(user_id.to_owned(), kind))
            .and_then(|s| s.checked)
            .map(|t| t.elapsed())
    }

    /// Records the result of a check. Returns true if the state changed.
    pub fn mark(&self, user_id: &str, kind: TokenKind, state: TokenState) -> bool {
        let mut slots = self.lock();
        let Some(slot) = slots.get_mut(&(user_id.to_owned(), kind)) else {
            return false;
        };
        slot.checked = Some(Instant::now());
        std::mem::replace(&mut slot.state, state) != state
    }
}

fn store_key(user_id: &str, kind: TokenKind) -> String {
    format!("{user_id}.{}", kind.key())
}

#[cfg(test)]
mod tests {
    use cyex_settings::schema::{Account, TokenMeta};

    use super::*;
    use crate::secrets::MemoryStore;

    fn secret(access: &str) -> Secret {
        Secret {
            access: access.into(),
            refresh: Some("r".into()),
            client_id: "c".into(),
        }
    }

    fn accounts() -> Accounts {
        Accounts {
            list: vec![Account {
                user_id: "1".into(),
                helix: Some(TokenMeta::default()),
                gql: Some(TokenMeta::default()),
                ..Default::default()
            }],
            default_id: Some("1".into()),
        }
    }

    #[test]
    fn tokens_survive_a_restart() {
        let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
        let vault = Vault::new(store.clone());
        vault.put("1", TokenKind::Helix, secret("a")).unwrap();
        assert_eq!(vault.state("1", TokenKind::Helix), TokenState::Valid);

        let restarted = Vault::new(store);
        assert!(restarted.load(&accounts()).is_empty());
        assert_eq!(restarted.secret("1", TokenKind::Helix), Some(secret("a")));
        assert_eq!(restarted.state("1", TokenKind::Helix), TokenState::Checking);
        assert_eq!(restarted.state("1", TokenKind::Gql), TokenState::Missing);
    }

    #[test]
    fn reload_keeps_state_of_unchanged_tokens() {
        let vault = Vault::new(Arc::new(MemoryStore::default()));
        vault.put("1", TokenKind::Helix, secret("a")).unwrap();
        vault.load(&accounts());
        assert_eq!(vault.state("1", TokenKind::Helix), TokenState::Valid);
        // Tokens the list no longer mentions are dropped from memory.
        vault.load(&Accounts::default());
        assert_eq!(vault.state("1", TokenKind::Helix), TokenState::Missing);
    }

    #[test]
    fn forget_deletes_from_the_store() {
        let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
        let vault = Vault::new(store.clone());
        vault.put("1", TokenKind::Gql, secret("g")).unwrap();
        assert_eq!(vault.forget("1", TokenKind::Gql).unwrap(), Some(secret("g")));
        assert_eq!(store.load("1.gql").unwrap(), None);
    }

    /// S1 A corrupt entry is reported by key and position, never by what it holds.
    #[test]
    fn unreadable_tokens_do_not_echo_their_value() {
        let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
        store.save("1.helix", r#"{"access": 5}"#).unwrap();
        let errors = Vault::new(store).load(&accounts());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("1.helix: stored token unreadable (line 1 col "));
        assert!(!errors[0].contains('5'), "{}", errors[0]);
    }

    /// Fails every save until `heal` is set.
    #[derive(Default)]
    struct FlakyStore {
        inner: MemoryStore,
        healed: std::sync::atomic::AtomicBool,
    }

    impl FlakyStore {
        fn heal(&self) {
            self.healed.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl SecretStore for FlakyStore {
        fn load(&self, key: &str) -> Result<Option<String>, String> {
            self.inner.load(key)
        }
        fn save(&self, key: &str, value: &str) -> Result<(), String> {
            if self.healed.load(std::sync::atomic::Ordering::SeqCst) {
                self.inner.save(key, value)
            } else {
                Err("store locked".into())
            }
        }
        fn delete(&self, key: &str) -> Result<(), String> {
            self.inner.delete(key)
        }
    }

    /// V107-A1 A refreshed token stays in memory when the Credential Manager write fails.
    #[test]
    fn a_kept_token_survives_a_failed_save() {
        let vault = Vault::new(Arc::new(FlakyStore::default()));
        assert!(
            vault
                .keep("1", TokenKind::Helix, secret("new"), TokenState::Offline)
                .is_err()
        );
        assert_eq!(vault.secret("1", TokenKind::Helix), Some(secret("new")));
        assert_eq!(vault.state("1", TokenKind::Helix), TokenState::Offline);
        // A reload from the store (still holding nothing) doesn't bring the old one back.
        vault.load(&accounts());
        assert_eq!(vault.secret("1", TokenKind::Helix), Some(secret("new")));
    }

    #[test]
    fn a_failed_save_is_retried() {
        let store = Arc::new(FlakyStore::default());
        let vault = Vault::new(store.clone());
        let _ = vault.keep("1", TokenKind::Helix, secret("new"), TokenState::Offline);
        let errors = vault.save_pending();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("1.helix: "));
        assert!(!errors[0].contains("new"), "{}", errors[0]);
        store.heal();
        assert!(vault.save_pending().is_empty());
        assert_eq!(store.load("1.helix").unwrap(), Some(serde_json::to_string(&secret("new")).unwrap()));
        // Saved now: a reload takes the stored copy as unchanged.
        assert!(vault.save_pending().is_empty());
    }

    #[test]
    fn put_keeps_memory_when_the_save_fails() {
        // Sign-in treats an error as "nothing stored".
        let vault = Vault::new(Arc::new(FlakyStore::default()));
        assert!(vault.put("1", TokenKind::Helix, secret("a")).is_err());
        assert_eq!(vault.secret("1", TokenKind::Helix), None);
    }

    #[test]
    fn mark_reports_changes() {
        let vault = Vault::new(Arc::new(MemoryStore::default()));
        vault.put("1", TokenKind::Helix, secret("a")).unwrap();
        assert!(!vault.mark("1", TokenKind::Helix, TokenState::Valid));
        assert!(vault.mark("1", TokenKind::Helix, TokenState::Invalid));
        assert!(!vault.mark("9", TokenKind::Helix, TokenState::Invalid));
    }
}
