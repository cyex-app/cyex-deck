//! V107-Bn Whether a channel takes cheers, by login, from the last rules read. With Bits
//! off, cheer words are plain chat. Unknown means a read is on its way; an old answer still
//! counts while it is read again.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Read again after this.
const FRESH: Duration = Duration::from_secs(600);

#[derive(Default)]
pub struct BitsOn {
    known: HashMap<String, (bool, Instant)>,
    loading: HashSet<String>,
}

impl BitsOn {
    /// What's known, and whether the caller should start a read (marked as started).
    pub fn lookup(&mut self, login: &str, now: Instant) -> (Option<bool>, bool) {
        let known = self.known.get(login).copied();
        let old = known.is_none_or(|(_, at)| now.duration_since(at) >= FRESH);
        let read = old && self.loading.insert(login.to_owned());
        (known.map(|(on, _)| on), read)
    }

    pub fn set(&mut self, login: &str, on: bool, now: Instant) {
        self.loading.remove(login);
        self.known.insert(login.to_owned(), (on, now));
    }

    /// The read failed: the next lookup tries again.
    pub fn failed(&mut self, login: &str) {
        self.loading.remove(login);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_bn_unknown_reads_once_then_knows() {
        let mut b = BitsOn::default();
        let t = Instant::now();
        assert_eq!(b.lookup("x", t), (None, true));
        assert_eq!(b.lookup("x", t), (None, false), "one read at a time");
        b.failed("x");
        assert_eq!(b.lookup("x", t), (None, true));
        b.set("x", false, t);
        assert_eq!(b.lookup("x", t), (Some(false), false));
        assert_eq!(b.lookup("y", t), (None, true), "per channel");
    }

    #[test]
    fn v107_bn_an_old_answer_counts_while_read_again() {
        let mut b = BitsOn::default();
        let t = Instant::now();
        b.set("x", true, t);
        let later = t + FRESH;
        assert_eq!(b.lookup("x", later), (Some(true), true));
        assert_eq!(b.lookup("x", later), (Some(true), false));
        b.set("x", false, later);
        assert_eq!(b.lookup("x", later), (Some(false), false));
    }
}
