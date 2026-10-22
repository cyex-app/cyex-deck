//! V107-Bn One idempotency id per cheer the user sends. A try whose outcome is unknown
//! (network, `UNKNOWN`, throttled) leaves its id, so sending the same cheer again soon
//! reuses it and Twitch can't take the Bits twice. A cheer that went out is remembered a
//! short while: the same one again asks first (a held Ctrl+Enter, Up and Enter, two windows).

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How long an unsure try's id is reused, from its last try.
const KEEP: Duration = Duration::from_secs(600);
/// The same cheer again within this asks first.
const AGAIN: Duration = Duration::from_secs(30);

/// (account, channel id, bits, text).
pub type Key = (String, String, u32, String);

/// How a try ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    Sent,
    /// Twitch said no: nothing was spent.
    Refused,
    /// Maybe spent: keep the id for a retry.
    Unsure,
}

enum State {
    Busy,
    Unsure,
    Sent,
}

struct Open {
    id: String,
    at: Instant,
    state: State,
}

#[derive(Default)]
pub struct Attempts {
    open: HashMap<Key, Open>,
}

impl Attempts {
    fn prune(&mut self, now: Instant) {
        self.open.retain(|_, o| match o.state {
            State::Busy => true,
            State::Unsure => now.duration_since(o.at) < KEEP,
            State::Sent => now.duration_since(o.at) < AGAIN,
        });
    }

    /// The same cheer went out moments ago.
    pub fn again(&mut self, key: &Key, now: Instant) -> bool {
        self.prune(now);
        self.open.get(key).is_some_and(|o| matches!(o.state, State::Sent))
    }

    /// The id to send with. `None` while the same cheer is already on its way.
    pub fn begin(&mut self, key: Key, now: Instant, fresh: impl FnOnce() -> String) -> Option<String> {
        self.prune(now);
        match self.open.get_mut(&key) {
            Some(o) if matches!(o.state, State::Busy) => None,
            Some(o) if matches!(o.state, State::Unsure) => {
                o.state = State::Busy;
                o.at = now;
                Some(o.id.clone())
            }
            // New, or sent before and confirmed again: a new cheer.
            _ => {
                let id = fresh();
                let open = Open {
                    id: id.clone(),
                    at: now,
                    state: State::Busy,
                };
                self.open.insert(key, open);
                Some(id)
            }
        }
    }

    pub fn end(&mut self, key: &Key, now: Instant, how: Ended) {
        let Some(o) = self.open.get_mut(key) else { return };
        o.at = now;
        o.state = match how {
            Ended::Sent => State::Sent,
            Ended::Unsure => State::Unsure,
            Ended::Refused => {
                self.open.remove(key);
                return;
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> Key {
        ("me".into(), "1".into(), 100, text.into())
    }

    fn counter() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("id{n}")
        }
    }

    #[test]
    fn v107_bn_a_retry_reuses_the_id() {
        let mut a = Attempts::default();
        let t = Instant::now();
        let mut fresh = counter();
        let k = key("Cheer100");
        let first = a.begin(k.clone(), t, &mut fresh).unwrap();
        assert_eq!(a.begin(k.clone(), t, &mut fresh), None, "on its way: no second send");
        a.end(&k, t, Ended::Unsure);
        assert!(!a.again(&k, t), "unsure isn't sent: no extra ask");
        assert_eq!(a.begin(k.clone(), t + Duration::from_secs(5), &mut fresh), Some(first.clone()));
        a.end(&k, t, Ended::Refused);
        let next = a.begin(k.clone(), t + Duration::from_secs(6), &mut fresh).unwrap();
        assert_ne!(next, first, "refused: the same text again is a new try");
        let other = a.begin(key("Cheer100 gg"), t, &mut fresh).unwrap();
        assert_ne!(other, next);
    }

    #[test]
    fn v107_bn_the_same_cheer_again_asks_then_is_new() {
        let mut a = Attempts::default();
        let t = Instant::now();
        let mut fresh = counter();
        let k = key("Cheer100");
        let first = a.begin(k.clone(), t, &mut fresh).unwrap();
        a.end(&k, t, Ended::Sent);
        assert!(a.again(&k, t + Duration::from_secs(29)));
        assert!(!a.again(&key("Cheer100 gg"), t));
        let second = a.begin(k.clone(), t + Duration::from_secs(1), &mut fresh).unwrap();
        assert_ne!(first, second, "confirmed again: a new cheer with a new id");
        a.end(&k, t + Duration::from_secs(1), Ended::Sent);
        assert!(!a.again(&k, t + Duration::from_secs(32)));
    }

    #[test]
    fn v107_bn_an_unsure_id_lives_from_its_last_try() {
        let mut a = Attempts::default();
        let t = Instant::now();
        let k = key("Cheer1");
        let first = a.begin(k.clone(), t, || "a".into()).unwrap();
        let late = t + KEEP - Duration::from_secs(1);
        a.end(&k, late, Ended::Unsure);
        assert_eq!(a.begin(k.clone(), late + Duration::from_secs(5), || "b".into()), Some(first));
        a.end(&k, late + Duration::from_secs(5), Ended::Unsure);
        let gone = late + Duration::from_secs(5) + KEEP + Duration::from_secs(1);
        assert_eq!(a.begin(k, gone, || "c".into()).as_deref(), Some("c"));
    }
}
