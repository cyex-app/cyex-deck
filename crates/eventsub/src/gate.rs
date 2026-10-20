//! V107-R17 One gate for every EventSub socket of a user token: Twitch allows 3 per
//! client id + user id, so a 4th waits until one has closed, never opens next to them.
//! A socket holds its slot from before it connects until its task has ended (reconnects
//! included). Who waits is told to the pressure hook, so the mod feeds can give back
//! their second socket.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tokio::sync::Notify;

/// Twitch's EventSub sockets per user token.
pub const PER_TOKEN: usize = 3;

/// Account → (sockets holding a slot, waiting for one).
type Counts = Option<HashMap<String, (usize, usize)>>;

/// Told the account whenever one of its sockets starts waiting.
type Hook = Box<dyn Fn(&str) + Send + Sync>;

pub struct Gate {
    counts: Mutex<Counts>,
    freed: Notify,
    pressure: OnceLock<Hook>,
}

/// Held by one socket; dropping it frees the slot.
pub struct Slot<'a> {
    gate: &'a Gate,
    account: String,
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        self.gate.change(&self.account, |c| c.0 = c.0.saturating_sub(1));
        self.gate.freed.notify_waiters();
    }
}

impl Gate {
    pub const fn new() -> Self {
        Gate {
            counts: Mutex::new(None),
            freed: Notify::const_new(),
            pressure: OnceLock::new(),
        }
    }

    fn change<R>(&self, account: &str, f: impl FnOnce(&mut (usize, usize)) -> R) -> R {
        let mut c = self.counts.lock().unwrap_or_else(|p| p.into_inner());
        let live = c.get_or_insert_with(HashMap::new);
        let e = live.entry(account.to_owned()).or_default();
        let r = f(e);
        if *e == (0, 0) {
            live.remove(account);
        }
        r
    }

    /// Called with the account whenever a socket starts waiting (set once).
    pub fn on_pressure(&self, f: impl Fn(&str) + Send + Sync + 'static) {
        let _ = self.pressure.set(Box::new(f));
    }

    /// (sockets open, sockets waiting) for the account.
    pub fn count(&self, account: &str) -> (usize, usize) {
        let c = self.counts.lock().unwrap_or_else(|p| p.into_inner());
        c.as_ref().and_then(|l| l.get(account)).copied().unwrap_or_default()
    }

    /// A slot now, if one is free.
    pub fn try_acquire(&self, account: &str) -> Option<Slot<'_>> {
        let got = self.change(account, |c| {
            let ok = c.0 < PER_TOKEN;
            if ok {
                c.0 += 1;
            }
            ok
        });
        got.then(|| Slot { gate: self, account: account.to_owned() })
    }

    /// Waits for a free slot; while waiting, the pressure hook asks others to give one back.
    pub async fn acquire(&self, account: &str) -> Slot<'_> {
        let mut waiting: Option<Waiting> = None;
        loop {
            let freed = self.freed.notified();
            tokio::pin!(freed);
            freed.as_mut().enable();
            if let Some(slot) = self.try_acquire(account) {
                return slot;
            }
            if waiting.is_none() {
                self.change(account, |c| c.1 += 1);
                waiting = Some(Waiting { gate: self, account: account.to_owned() });
                if let Some(f) = self.pressure.get() {
                    f(account);
                }
            }
            freed.await;
        }
    }
}

/// Counts a waiter until it has its slot or gives up (its task dropped).
struct Waiting<'a> {
    gate: &'a Gate,
    account: String,
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.gate.change(&self.account, |c| c.1 = c.1.saturating_sub(1));
    }
}

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

/// The app's gate: every socket goes through it.
pub static GATE: Gate = Gate::new();

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn v107_r17_never_a_fourth_socket_per_token() {
        let gate: &'static Gate = Box::leak(Box::new(Gate::new()));
        let asked = Arc::new(AtomicUsize::new(0));
        let a2 = asked.clone();
        gate.on_pressure(move |acc| {
            assert_eq!(acc, "1");
            a2.fetch_add(1, Ordering::SeqCst);
        });
        let s1 = gate.acquire("1").await;
        let _s2 = gate.acquire("1").await;
        let _s3 = gate.acquire("1").await;
        assert!(gate.try_acquire("1").is_none(), "a 4th never opens");
        let _other = gate.try_acquire("2").expect("another token has its own 3");
        let fourth = tokio::spawn(async move { gate.acquire("1").await.account.clone() });
        tokio::task::yield_now().await;
        while gate.count("1").1 == 0 {
            tokio::task::yield_now().await;
        }
        assert_eq!(gate.count("1"), (3, 1));
        assert_eq!(asked.load(Ordering::SeqCst), 1, "the mod feeds are asked to give one back");
        drop(s1);
        assert_eq!(fourth.await.unwrap(), "1");
        // The 4th's slot went with its task: 2 open, nobody waiting.
        assert_eq!(gate.count("1"), (2, 0));
        // A waiter that gives up stops counting.
        let _s4 = gate.acquire("1").await;
        let gone = tokio::spawn(async move { drop(gate.acquire("1").await) });
        while gate.count("1").1 == 0 {
            tokio::task::yield_now().await;
        }
        gone.abort();
        let _ = gone.await;
        assert_eq!(gate.count("1"), (3, 0));
    }
}
