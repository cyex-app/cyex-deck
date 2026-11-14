//! V111-13 A small cache with an age limit for the usercard's outside lookups (api.ivr.fi,
//! roles.tv): reopening a card within the limit asks nobody. Bounded, oldest out first.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct TtlCache<K, V> {
    ttl: Duration,
    cap: usize,
    map: Mutex<HashMap<K, (Instant, V)>>,
}

impl<K: Eq + Hash + Clone, V: Clone> TtlCache<K, V> {
    pub fn new(ttl: Duration, cap: usize) -> Self {
        Self { ttl, cap: cap.max(1), map: Mutex::new(HashMap::new()) }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.get_at(key, Instant::now())
    }

    pub fn put(&self, key: K, value: V) {
        self.put_at(key, value, Instant::now());
    }

    fn get_at(&self, key: &K, now: Instant) -> Option<V> {
        let map = self.map.lock().unwrap_or_else(|p| p.into_inner());
        map.get(key).filter(|(at, _)| now.saturating_duration_since(*at) < self.ttl).map(|(_, v)| v.clone())
    }

    fn put_at(&self, key: K, value: V, now: Instant) {
        let mut map = self.map.lock().unwrap_or_else(|p| p.into_inner());
        map.retain(|_, (at, _)| now.saturating_duration_since(*at) < self.ttl);
        while map.len() >= self.cap && !map.contains_key(&key) {
            let Some(oldest) = map.iter().min_by_key(|(_, (at, _))| *at).map(|(k, _)| k.clone()) else { break };
            map.remove(&oldest);
        }
        map.insert(key, (now, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v111_13_fresh_entries_answer_old_ones_dont() {
        let c = TtlCache::new(Duration::from_secs(60), 8);
        let t0 = Instant::now();
        c.put_at("a", 1, t0);
        assert_eq!(c.get_at(&"a", t0 + Duration::from_secs(59)), Some(1));
        assert_eq!(c.get_at(&"a", t0 + Duration::from_secs(60)), None, "expired: asked again");
        assert_eq!(c.get_at(&"b", t0), None);
    }

    #[test]
    fn v111_13_stays_bounded() {
        let c = TtlCache::new(Duration::from_secs(60), 2);
        let t0 = Instant::now();
        c.put_at(1, "one", t0);
        c.put_at(2, "two", t0 + Duration::from_secs(1));
        c.put_at(3, "three", t0 + Duration::from_secs(2));
        let now = t0 + Duration::from_secs(3);
        assert_eq!(c.get_at(&1, now), None, "the oldest went");
        assert_eq!(c.get_at(&2, now), Some("two"));
        assert_eq!(c.get_at(&3, now), Some("three"));
        c.put_at(3, "again", now);
        assert_eq!(c.get_at(&2, now), Some("two"), "replacing a key drops nothing");
        assert_eq!(c.get_at(&3, now), Some("again"));
    }
}
