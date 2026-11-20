//! V112-14 cyex.tv sessions per Twitch account. The token store keeps the refresh token and who
//! signed in (`<twitch id>.cyextv`, never settings, exports or backups); the access token stays in
//! memory. A refresh rotates the refresh token, so refreshes run one at a time per account and
//! the new refresh token is saved before the new access token is used.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use super::{CyextvError, TokenAnswer};
use crate::secrets::SecretStore;

/// Kept in place of a session cyex.tv ended: "Sign in again". Never parses as a session.
const ENDED: &str = "ended";
/// An access token counts as gone a minute early.
const EARLY: i64 = 60;

/// Who signed in: the cyex.tv account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Who {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    refresh: String,
    #[serde(default)]
    who: Who,
}

type Gate = Arc<tokio::sync::Mutex<()>>;

pub struct CyextvSessions {
    store: Arc<dyn SecretStore>,
    /// Access token and its end (unix seconds) by Twitch id.
    access: Mutex<HashMap<String, (String, i64)>>,
    /// One refresh, sign-in save or sign-out at a time per account.
    gates: Mutex<HashMap<String, Gate>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl CyextvSessions {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self {
            store,
            access: Mutex::default(),
            gates: Mutex::default(),
        }
    }

    fn key(twitch_id: &str) -> String {
        format!("{twitch_id}.cyextv")
    }

    fn gate(&self, twitch_id: &str) -> Gate {
        lock(&self.gates).entry(twitch_id.to_owned()).or_default().clone()
    }

    fn stored(&self, twitch_id: &str) -> Option<Stored> {
        let raw = self.store.load(&Self::key(twitch_id)).ok()??;
        serde_json::from_str::<Stored>(&raw)
            .ok()
            .filter(|s| super::token_ok(&s.refresh))
    }

    fn save(&self, twitch_id: &str, refresh: &str, who: &Who) -> Result<(), CyextvError> {
        let raw = serde_json::to_string(&Stored {
            refresh: refresh.to_owned(),
            who: who.clone(),
        })
        .map_err(|e| CyextvError::Store(e.to_string()))?;
        self.store.save(&Self::key(twitch_id), &raw).map_err(CyextvError::Store)
    }

    /// The signed-in cyex.tv account, if a session is kept.
    pub fn who(&self, twitch_id: &str) -> Option<Who> {
        self.stored(twitch_id).map(|s| s.who)
    }

    /// cyex.tv ended the session: "Sign in again" until the next sign-in or sign-out.
    pub fn ended(&self, twitch_id: &str) -> bool {
        self.store.load(&Self::key(twitch_id)).ok().flatten().as_deref() == Some(ENDED)
    }

    /// A finished sign-in: the refresh token is saved first, then the access token is kept.
    pub async fn start(&self, twitch_id: &str, t: &TokenAnswer, who: &Who, now: i64) -> Result<(), CyextvError> {
        let gate = self.gate(twitch_id);
        let _turn = gate.lock().await;
        self.save(twitch_id, &t.refresh, who)?;
        lock(&self.access).insert(twitch_id.to_owned(), (t.access.clone(), now + t.expires_in));
        Ok(())
    }

    /// The account's name changed on cyex.tv (shown as "Signed in as …").
    pub async fn rename(&self, twitch_id: &str, who: &Who) {
        let gate = self.gate(twitch_id);
        let _turn = gate.lock().await;
        if let Some(s) = self.stored(twitch_id)
            && s.who != *who
            && let Err(e) = self.save(twitch_id, &s.refresh, who)
        {
            eprintln!("cyex.tv: couldn't save the account name: {e}");
        }
    }

    /// A live access token: the kept one, or a new one from `refresh` (given the stored refresh
    /// token). Single-flight: a second caller waits and then takes the first one's token.
    pub async fn bearer<F, Fut>(&self, twitch_id: &str, now: i64, refresh: F) -> Result<String, CyextvError>
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = Result<TokenAnswer, CyextvError>>,
    {
        let gate = self.gate(twitch_id);
        let _turn = gate.lock().await;
        let kept_access = lock(&self.access).get(twitch_id).cloned();
        if let Some((access, until)) = kept_access
            && until - EARLY > now
        {
            return Ok(access);
        }
        let Some(kept) = self.stored(twitch_id) else {
            return Err(CyextvError::SignedOut);
        };
        match refresh(kept.refresh).await {
            Ok(t) => {
                // The old refresh token is spent now: the new one is saved before anything else.
                self.save(twitch_id, &t.refresh, &kept.who)?;
                lock(&self.access).insert(twitch_id.to_owned(), (t.access.clone(), now + t.expires_in));
                Ok(t.access)
            }
            Err(CyextvError::SignedOut) => {
                self.end_locked(twitch_id);
                Err(CyextvError::SignedOut)
            }
            Err(e) => Err(e),
        }
    }

    /// cyex.tv refused `used` (401): it isn't handed out again; the next `bearer` refreshes.
    pub fn stale(&self, twitch_id: &str, used: &str) {
        let mut access = lock(&self.access);
        if access.get(twitch_id).is_some_and(|(a, _)| a == used) {
            access.remove(twitch_id);
        }
    }

    fn end_locked(&self, twitch_id: &str) {
        lock(&self.access).remove(twitch_id);
        if let Err(e) = self.store.save(&Self::key(twitch_id), ENDED) {
            eprintln!("cyex.tv: couldn't mark the session ended: {e}");
        }
    }

    /// Signed out or the account removed: everything goes. Returns the refresh token to revoke.
    pub async fn forget(&self, twitch_id: &str) -> Result<Option<String>, CyextvError> {
        let gate = self.gate(twitch_id);
        let _turn = gate.lock().await;
        let kept = self.stored(twitch_id).map(|s| s.refresh);
        lock(&self.access).remove(twitch_id);
        self.store.delete(&Self::key(twitch_id)).map_err(CyextvError::Store)?;
        Ok(kept)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::secrets::MemoryStore;

    fn answer(access: &str, refresh: &str) -> TokenAnswer {
        TokenAnswer {
            access: access.into(),
            refresh: refresh.into(),
            expires_in: 3600,
            scope: String::new(),
        }
    }

    fn who() -> Who {
        Who {
            id: "acc1".into(),
            name: "Ada".into(),
        }
    }

    #[tokio::test]
    async fn sign_in_keeps_the_refresh_token_and_who_but_not_the_access_token() {
        let store = Arc::new(MemoryStore::default());
        let s = CyextvSessions::new(store.clone());
        assert!(s.who("1").is_none());
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        let raw = store.load("1.cyextv").unwrap().unwrap();
        assert!(raw.contains("r1") && !raw.contains("a1"), "{raw}");
        assert_eq!(s.who("1"), Some(who()));
        let a = s.bearer("1", 10, |_| async { panic!("no refresh while live") }).await.unwrap();
        assert_eq!(a, "a1");
    }

    #[tokio::test]
    async fn refresh_rotates_and_saves_the_new_token_first() {
        let store = Arc::new(MemoryStore::default());
        let s = CyextvSessions::new(store.clone());
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        // An hour later (minus the early minute) the access token is gone.
        let got = s
            .bearer("1", 3600 - 30, |old| async move {
                assert_eq!(old, "r1");
                Ok(answer("a2", "r2"))
            })
            .await
            .unwrap();
        assert_eq!(got, "a2");
        assert!(store.load("1.cyextv").unwrap().unwrap().contains("r2"));
        assert_eq!(s.who("1"), Some(who()), "who survives a refresh");
        // The next refresh sends r2, never r1 again.
        s.stale("1", "a2");
        s.bearer("1", 4000, |old| async move {
            assert_eq!(old, "r2");
            Ok(answer("a3", "r3"))
        })
        .await
        .unwrap();
        // `stale` with an older token keeps the newer one.
        s.stale("1", "a2");
        assert_eq!(s.bearer("1", 4000, |_| async { panic!() }).await.unwrap(), "a3");
    }

    #[tokio::test]
    async fn two_callers_refresh_once() {
        let s = CyextvSessions::new(Arc::new(MemoryStore::default()));
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        s.stale("1", "a1");
        let calls = AtomicUsize::new(0);
        let refresh = |old: String| {
            let n = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                tokio::task::yield_now().await;
                assert_eq!(old, "r1");
                Ok(answer(&format!("a{}", n + 2), "r2"))
            }
        };
        let (x, y) = tokio::join!(s.bearer("1", 100, refresh), s.bearer("1", 100, refresh));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!((x.unwrap(), y.unwrap()), ("a2".to_owned(), "a2".to_owned()));
    }

    #[tokio::test]
    async fn a_refused_refresh_ends_the_session_until_the_next_sign_in() {
        let s = CyextvSessions::new(Arc::new(MemoryStore::default()));
        assert_eq!(s.bearer("1", 0, |_| async { panic!() }).await, Err(CyextvError::SignedOut));
        assert!(!s.ended("1"));
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        s.stale("1", "a1");
        let r = s.bearer("1", 0, |_| async { Err(CyextvError::SignedOut) }).await;
        assert_eq!(r, Err(CyextvError::SignedOut));
        assert!(s.ended("1") && s.who("1").is_none());
        // A network failure keeps the session.
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        assert!(!s.ended("1"));
        s.stale("1", "a1");
        assert_eq!(s.bearer("1", 0, |_| async { Err(CyextvError::Down) }).await, Err(CyextvError::Down));
        assert!(s.who("1").is_some());
    }

    #[tokio::test]
    async fn forget_returns_the_refresh_token_and_clears_everything() {
        let store = Arc::new(MemoryStore::default());
        let s = CyextvSessions::new(store.clone());
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        assert_eq!(s.forget("1").await.unwrap().as_deref(), Some("r1"));
        assert!(store.load("1.cyextv").unwrap().is_none());
        assert_eq!(s.bearer("1", 0, |_| async { panic!() }).await, Err(CyextvError::SignedOut));
        assert_eq!(s.forget("1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn rename_keeps_the_refresh_token() {
        let s = CyextvSessions::new(Arc::new(MemoryStore::default()));
        s.start("1", &answer("a1", "r1"), &who(), 0).await.unwrap();
        let new = Who { id: "acc1".into(), name: "Ada L".into() };
        s.rename("1", &new).await;
        assert_eq!(s.who("1"), Some(new));
        assert_eq!(s.forget("1").await.unwrap().as_deref(), Some("r1"));
    }
}
