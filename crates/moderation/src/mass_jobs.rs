//! V107-R21 Running mass actions: one per account at a time, each with its own Stop flag
//! and a job number the panels tell runs apart by.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub const BUSY: &str = "A mass action is already running.";

struct Entry {
    account: String,
    channel_id: String,
    flag: Arc<AtomicBool>,
}

/// A registered job: its number and Stop flag.
#[derive(Clone)]
pub struct Ticket {
    pub id: u32,
    pub flag: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct Registry {
    jobs: Mutex<(u32, Vec<Entry>)>,
}

impl Registry {
    fn lock(&self) -> MutexGuard<'_, (u32, Vec<Entry>)> {
        self.jobs.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// A new job for `account` in `channel_id`, or [`BUSY`] while one of its jobs runs
    /// (stopped but not yet ended counts too: its last request may still be out).
    pub fn try_register(&self, account: &str, channel_id: &str) -> Result<Ticket, &'static str> {
        let mut g = self.lock();
        if g.1.iter().any(|e| e.account == account) {
            return Err(BUSY);
        }
        g.0 = g.0.wrapping_add(1);
        let flag = Arc::new(AtomicBool::new(false));
        g.1.push(Entry { account: account.to_owned(), channel_id: channel_id.to_owned(), flag: flag.clone() });
        Ok(Ticket { id: g.0, flag })
    }

    pub fn unregister(&self, flag: &Arc<AtomicBool>) {
        self.lock().1.retain(|e| !Arc::ptr_eq(&e.flag, flag));
    }

    /// Stops `account`'s job in `channel_id`; without an account, every job there.
    pub fn stop(&self, account: Option<&str>, channel_id: &str) {
        for e in self.lock().1.iter() {
            if e.channel_id == channel_id && account.is_none_or(|a| a == e.account) {
                e.flag.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn running(&self, account: &str) -> bool {
        self.lock().1.iter().any(|e| e.account == account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_r21_one_job_per_account() {
        let r = Registry::default();
        let a = r.try_register("1", "c").unwrap();
        assert_eq!(r.try_register("1", "c").err(), Some(BUSY));
        assert_eq!(r.try_register("1", "other").err(), Some(BUSY), "per account, any channel");
        let b = r.try_register("2", "c").unwrap();
        assert_ne!(a.id, b.id);
        // Stopped but not ended: still busy.
        r.stop(Some("1"), "c");
        assert_eq!(r.try_register("1", "c").err(), Some(BUSY));
        r.unregister(&a.flag);
        assert!(!r.running("1"));
        assert!(r.try_register("1", "c").is_ok());
    }

    #[test]
    fn v107_r21_stop_hits_only_that_accounts_job() {
        let r = Registry::default();
        let a = r.try_register("1", "c").unwrap();
        let b = r.try_register("2", "c").unwrap();
        r.stop(Some("1"), "c");
        assert!(a.flag.load(Ordering::Relaxed));
        assert!(!b.flag.load(Ordering::Relaxed));
        r.stop(Some("2"), "elsewhere");
        assert!(!b.flag.load(Ordering::Relaxed));
        r.stop(None, "c");
        assert!(b.flag.load(Ordering::Relaxed));
    }
}
