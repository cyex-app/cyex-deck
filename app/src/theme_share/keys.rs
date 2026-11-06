//! V109-T01 Edit keys of the themes you shared, one Credential Manager entry per link
//! (`theme-edit.<id>`). Never in settings, exports, backups or codes. A test data folder
//! keeps them in memory.

use std::sync::Arc;

use cyex_auth::secrets::SecretStore;

/// The server's keys are 43 characters; anything far longer isn't one.
const MAX_KEY: usize = 200;

pub struct EditKeys(Arc<dyn SecretStore>);

impl EditKeys {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self(store)
    }

    fn entry(id: &str) -> String {
        format!("theme-edit.{id}")
    }

    pub fn get(&self, id: &str) -> Option<String> {
        if id.is_empty() {
            return None;
        }
        self.0.load(&Self::entry(id)).ok().flatten().filter(|k| !k.is_empty())
    }

    pub fn put(&self, id: &str, key: &str) -> Result<(), String> {
        let key = key.trim();
        if key.is_empty() || key.len() > MAX_KEY || !key.bytes().all(|b| b.is_ascii_graphic()) {
            return Err("cyex.app sent an odd edit key.".into());
        }
        self.0.save(&Self::entry(id), key)
    }

    pub fn forget(&self, id: &str) -> Result<(), String> {
        self.0.delete(&Self::entry(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_auth::secrets::MemoryStore;

    #[test]
    fn keys_per_link() {
        let keys = EditKeys::new(Arc::new(MemoryStore::default()));
        assert_eq!(keys.get("k3f9x2ab"), None);
        keys.put("k3f9x2ab", "abc_DEF-123").unwrap();
        assert_eq!(keys.get("k3f9x2ab").as_deref(), Some("abc_DEF-123"));
        assert_eq!(keys.get("m4n5p6q7"), None);
        assert!(keys.put("m4n5p6q7", "has space").is_err());
        assert!(keys.put("m4n5p6q7", "").is_err());
        keys.forget("k3f9x2ab").unwrap();
        assert_eq!(keys.get("k3f9x2ab"), None);
        assert_eq!(keys.get(""), None);
    }
}
