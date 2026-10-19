//! V107-P7 Messages we deleted ourselves, so their mod-action echo (EventSub/Hermes) adds
//! no second chat line: ours already says it.

use std::collections::VecDeque;

/// How long an own delete waits for its echo, in milliseconds.
const KEEP_MS: i64 = 60_000;
/// Most own deletes remembered at once.
const CAP: usize = 64;

#[derive(Default)]
pub struct OwnDeletes(VecDeque<(String, i64)>);

impl OwnDeletes {
    /// We're deleting message `id` now.
    pub fn mark(&mut self, id: &str, now_ms: i64) {
        self.expire(now_ms);
        if self.0.len() == CAP {
            self.0.pop_front();
        }
        self.0.push_back((id.to_owned(), now_ms));
    }

    /// Our delete of `id` failed: another mod's delete of it gets its line.
    pub fn forget(&mut self, id: &str) {
        self.0.retain(|(i, _)| i != id);
    }

    /// Whether the delete of `id` was ours. Each mark answers once.
    pub fn take(&mut self, id: &str, now_ms: i64) -> bool {
        self.expire(now_ms);
        let at = self.0.iter().position(|(i, _)| i == id);
        at.map(|i| self.0.remove(i)).is_some()
    }

    fn expire(&mut self, now_ms: i64) {
        self.0.retain(|(_, at)| now_ms - at < KEEP_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_p7_own_delete_echo_is_dropped_once() {
        let mut o = OwnDeletes::default();
        o.mark("a", 0);
        assert!(!o.take("b", 10), "another mod's delete");
        assert!(o.take("a", 10), "our echo");
        assert!(!o.take("a", 20), "only once");
        o.mark("c", 0);
        o.forget("c");
        assert!(!o.take("c", 10), "failed delete");
        o.mark("d", 0);
        assert!(!o.take("d", KEEP_MS), "too late");
    }

    #[test]
    fn v107_p7_own_deletes_are_capped() {
        let mut o = OwnDeletes::default();
        for i in 0..=CAP {
            o.mark(&i.to_string(), 0);
        }
        assert!(!o.take("0", 1), "oldest dropped");
        assert!(o.take(&CAP.to_string(), 1));
    }
}
