//! V110-U65 Connection health: how often each socket (IRC per account, EventSub, 7TV, BTTV,
//! Hermes) had to reconnect in the last hour, and its last error. Each socket's task calls
//! `note` where its backoff starts; the debug panel reads `snapshot`. A small ring of
//! timestamps per socket, so memory stays flat however flaky the link is.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Reconnects older than this no longer count.
pub const WINDOW_MS: u64 = 60 * 60 * 1000;
/// Most stamps kept per socket; a count at the cap reads "at least".
pub const RING: usize = 64;
/// Most sockets tracked; a new one past this is ignored.
const LABELS: usize = 48;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub label: String,
    /// Reconnects in the last hour (capped at `RING`).
    pub count: u32,
    /// When the last one was, ms since the epoch.
    pub last_at: u64,
    /// Why, short; empty when not known.
    pub last_error: String,
}

#[derive(Default)]
struct Entry {
    stamps: VecDeque<u64>,
    last_at: u64,
    last_error: String,
}

#[derive(Default)]
pub struct Health {
    links: HashMap<String, Entry>,
}

impl Health {
    pub fn note(&mut self, now: u64, label: &str, error: &str) {
        if !self.links.contains_key(label) && self.links.len() >= LABELS {
            return;
        }
        let e = self.links.entry(label.to_owned()).or_default();
        prune(&mut e.stamps, now);
        if e.stamps.len() >= RING {
            e.stamps.pop_front();
        }
        e.stamps.push_back(now);
        e.last_at = now;
        error.clone_into(&mut e.last_error);
    }

    /// Sockets with a reconnect in the last hour or on record, most recent first.
    pub fn snapshot(&self, now: u64) -> Vec<Link> {
        let mut list: Vec<Link> = self
            .links
            .iter()
            .map(|(label, e)| Link {
                label: label.clone(),
                count: e
                    .stamps
                    .iter()
                    .filter(|&&t| now.saturating_sub(t) < WINDOW_MS)
                    .count() as u32,
                last_at: e.last_at,
                last_error: e.last_error.clone(),
            })
            .collect();
        list.sort_by(|a, b| b.last_at.cmp(&a.last_at).then_with(|| a.label.cmp(&b.label)));
        list
    }
}

fn prune(stamps: &mut VecDeque<u64>, now: u64) {
    while stamps
        .front()
        .is_some_and(|&t| now.saturating_sub(t) >= WINDOW_MS)
    {
        stamps.pop_front();
    }
}

fn global() -> &'static Mutex<Health> {
    static G: OnceLock<Mutex<Health>> = OnceLock::new();
    G.get_or_init(Mutex::default)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// A socket dropped and will retry. `error` is short ("connect failed", "dropped").
pub fn note(label: &str, error: &str) {
    global()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .note(now_ms(), label, error);
}

/// Every socket seen so far with its count for the last hour.
pub fn snapshot() -> Vec<Link> {
    global()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .snapshot(now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: u64 = WINDOW_MS;

    #[test]
    fn u65_counts_only_the_last_hour() {
        let mut h = Health::default();
        h.note(0, "7TV", "dropped");
        h.note(H / 2, "7TV", "dropped");
        h.note(H + 10, "7TV", "idle");
        let l = h.snapshot(H + 10);
        assert_eq!(l[0].count, 2);
        assert_eq!(l[0].last_error, "idle");
        assert_eq!(h.snapshot(2 * H + 20)[0].count, 0);
    }

    #[test]
    fn u65_ring_is_capped() {
        let mut h = Health::default();
        for i in 0..(RING as u64 * 3) {
            h.note(i, "IRC", "");
        }
        assert_eq!(h.links["IRC"].stamps.len(), RING);
        assert_eq!(h.snapshot(RING as u64 * 3)[0].count, RING as u32);
    }

    #[test]
    fn u65_newest_first_and_labels_are_capped() {
        let mut h = Health::default();
        h.note(5, "a", "");
        h.note(9, "b", "");
        assert_eq!(
            h.snapshot(10)
                .iter()
                .map(|l| l.label.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
        for i in 0..100 {
            h.note(10, &format!("x{i}"), "");
        }
        assert_eq!(h.links.len(), LABELS);
    }
}
