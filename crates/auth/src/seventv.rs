//! V1051-02 7TV sessions per Twitch account, kept like account tokens (Credential Manager),
//! never in settings, exports or backups. Not a `TokenKind`: Twitch never validates these.

use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::secrets::SecretStore;

/// K19 Stored in place of the token: 7TV rejected it. Never parses as a token.
const REJECTED: &str = "rejected";

/// Windows keeps 1.280 UTF-16 characters per vault value; stay under it.
const MAX_LEN: usize = 1200;

/// A 7TV session token (a JWT). A secret: `Debug` never shows it.
#[derive(Clone)]
pub struct SevenTvToken(String);

impl SevenTvToken {
    /// Refuses anything that isn't three base64url parts or is over 1.200 chars (vault limit).
    pub fn parse(jwt: &str) -> Option<Self> {
        let jwt = jwt.trim();
        let parts: Vec<&str> = jwt.split('.').collect();
        let url_safe = |s: &str| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        };
        if jwt.len() > MAX_LEN || parts.len() != 3 || !parts.iter().all(|p| url_safe(p)) {
            return None;
        }
        Some(Self(jwt.to_owned()))
    }

    pub fn secret(&self) -> &str {
        &self.0
    }

    /// The 7TV user id (payload `sub`).
    pub fn user_id(&self) -> Option<String> {
        self.claims()?.get("sub")?.as_str().map(str::to_owned)
    }

    /// Payload `exp`, unix seconds.
    pub fn expires(&self) -> Option<i64> {
        self.claims()?.get("exp")?.as_i64()
    }

    /// Counts as expired a minute early. A token with no `exp` is taken as live.
    pub fn expired(&self, now: i64) -> bool {
        self.expires().is_some_and(|exp| exp <= now + 60)
    }

    /// The payload is read, never verified: it's signed with 7TV's own secret.
    fn claims(&self) -> Option<serde_json::Value> {
        let payload = self.0.split('.').nth(1)?;
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
    }
}

impl fmt::Debug for SevenTvToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SevenTvToken(..)")
    }
}

/// `yyyy/mm/dd` (UTC) for unix seconds.
pub fn date(unix: i64) -> String {
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = unix.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}/{m:02}/{d:02}")
}

pub struct SevenTvTokens {
    store: Arc<dyn SecretStore>,
    /// Accounts whose session 7TV rejected this run, also when saving the K19 marker failed.
    rejected: Mutex<HashSet<String>>,
}

impl SevenTvTokens {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self {
            store,
            rejected: Mutex::default(),
        }
    }

    fn marks(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.rejected.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn key(twitch_id: &str) -> String {
        format!("{twitch_id}.7tv")
    }

    pub fn get(&self, twitch_id: &str) -> Option<SevenTvToken> {
        let raw = self.store.load(&Self::key(twitch_id)).ok()??;
        SevenTvToken::parse(&raw)
    }

    pub fn put(&self, twitch_id: &str, t: &SevenTvToken) -> Result<(), String> {
        self.store.save(&Self::key(twitch_id), t.secret())?;
        self.marks().remove(twitch_id);
        Ok(())
    }

    /// Signed out or withdrawn: no session and no "Sign in again".
    pub fn forget(&self, twitch_id: &str) -> Result<(), String> {
        self.marks().remove(twitch_id);
        self.store.delete(&Self::key(twitch_id))
    }

    /// 7TV rejected the session: the token is replaced by the K19 marker, and the account says
    /// "Sign in again" (marked even if the save fails).
    pub fn reject(&self, twitch_id: &str) -> Result<(), String> {
        self.marks().insert(twitch_id.to_owned());
        self.store.save(&Self::key(twitch_id), REJECTED)
    }

    /// "Sign in again" until the next sign-in (`put` overwrites the marker) or sign-out
    /// (`forget` deletes it). K19: kept across restarts as a marker in Credential Manager.
    pub fn rejected(&self, twitch_id: &str) -> bool {
        self.marks().contains(twitch_id)
            || self.store.load(&Self::key(twitch_id)).ok().flatten().as_deref() == Some(REJECTED)
    }

    /// The session ran out (rejected, or kept past its `exp`): "Sign in again".
    pub fn ended(&self, twitch_id: &str, now: i64) -> bool {
        self.rejected(twitch_id) || self.get(twitch_id).is_some_and(|t| t.expired(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;

    fn jwt(payload: &str) -> String {
        let b = |s: &str| URL_SAFE_NO_PAD.encode(s);
        format!("{}.{}.{}", b(r#"{"alg":"HS256"}"#), b(payload), "c2ln")
    }

    fn fake() -> String {
        jwt(r#"{"sub":"01X","exp":2000000000}"#)
    }

    #[test]
    fn parses_and_reads_claims() {
        let t = SevenTvToken::parse(&fake()).unwrap();
        assert_eq!(t.user_id().as_deref(), Some("01X"));
        assert_eq!(t.expires(), Some(2_000_000_000));
        assert!(!t.expired(1_999_999_000));
        assert!(t.expired(1_999_999_950));
        assert!(t.expired(2_000_000_001));
    }

    #[test]
    fn refuses_bad_shapes() {
        assert!(SevenTvToken::parse("a.b").is_none());
        assert!(SevenTvToken::parse("a.b.c.d").is_none());
        assert!(SevenTvToken::parse("a.b.c d").is_none());
        let long = format!("a.{}.c", "b".repeat(1200));
        assert!(SevenTvToken::parse(&long).is_none());
    }

    #[test]
    fn debug_hides_the_token() {
        let raw = fake();
        let t = SevenTvToken::parse(&raw).unwrap();
        let shown = format!("{t:?}");
        assert_eq!(shown, "SevenTvToken(..)");
        assert!(!shown.contains(&raw));
    }

    #[test]
    fn dates_are_yyyy_mm_dd() {
        assert_eq!(date(0), "1970/01/01");
        assert_eq!(date(2_000_000_000), "2033/05/18");
    }

    #[test]
    fn put_get_forget_use_the_account_key() {
        let store = Arc::new(MemoryStore::default());
        let tokens = SevenTvTokens::new(store.clone());
        let t = SevenTvToken::parse(&fake()).unwrap();
        assert!(tokens.get("123").is_none());
        tokens.put("123", &t).unwrap();
        assert!(store.load("123.7tv").unwrap().is_some());
        assert_eq!(tokens.get("123").unwrap().secret(), t.secret());
        tokens.forget("123").unwrap();
        assert!(tokens.get("123").is_none());
    }

    #[test]
    fn a_rejected_session_says_sign_in_again_until_the_next_sign_in_or_out() {
        let tokens = SevenTvTokens::new(Arc::new(MemoryStore::default()));
        let t = SevenTvToken::parse(&fake()).unwrap();
        tokens.put("1", &t).unwrap();
        assert!(!tokens.ended("1", 0));
        tokens.reject("1").unwrap();
        assert!(tokens.get("1").is_none());
        assert!(tokens.rejected("1") && tokens.ended("1", 0));
        tokens.put("1", &t).unwrap();
        assert!(!tokens.rejected("1"));
        tokens.reject("1").unwrap();
        tokens.forget("1").unwrap();
        assert!(!tokens.ended("1", 0));
        // Kept past its `exp`: ended, not rejected.
        tokens.put("1", &t).unwrap();
        assert!(tokens.ended("1", 2_000_000_000) && !tokens.rejected("1"));
    }

    /// K19 The mark lives in the store, so a restart (a new `SevenTvTokens`) keeps it.
    #[test]
    fn the_rejected_mark_survives_a_restart() {
        let store = Arc::new(MemoryStore::default());
        let t = SevenTvToken::parse(&fake()).unwrap();
        SevenTvTokens::new(store.clone()).reject("1").unwrap();
        let tokens = SevenTvTokens::new(store.clone());
        assert!(tokens.rejected("1"));
        assert!(tokens.get("1").is_none());
        assert!(tokens.ended("1", 0));
        tokens.put("1", &t).unwrap();
        assert!(!tokens.rejected("1"));
        tokens.reject("1").unwrap();
        tokens.forget("1").unwrap();
        assert!(!tokens.rejected("1"));
        assert!(store.load("1.7tv").unwrap().is_none());
        assert!(!SevenTvTokens::new(store).rejected("1"));
    }
}
