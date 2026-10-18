//! 1.41 A small in-memory cache whose entries go stale after a while (emote cards).

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Ttl<K, V> {
    ttl: Duration,
    map: Mutex<HashMap<K, (Instant, V)>>,
}

impl<K: Eq + Hash, V: Clone> Ttl<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            map: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.get_at(key, Instant::now())
    }

    pub fn put(&self, key: K, value: V) {
        self.put_at(key, value, Instant::now());
    }

    fn get_at(&self, key: &K, now: Instant) -> Option<V> {
        let map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        map.get(key)
            .filter(|(at, _)| now.saturating_duration_since(*at) < self.ttl)
            .map(|(_, v)| v.clone())
    }

    /// Drops stale entries on the way, so the map stays as small as what's fresh.
    fn put_at(&self, key: K, value: V, now: Instant) {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        map.retain(|_, (at, _)| now.saturating_duration_since(*at) < self.ttl);
        map.insert(key, (now, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_go_stale_and_get_dropped() {
        let c = Ttl::new(Duration::from_secs(60));
        let t0 = Instant::now();
        c.put_at("a", 1, t0);
        assert_eq!(c.get_at(&"a", t0 + Duration::from_secs(59)), Some(1));
        assert_eq!(c.get_at(&"a", t0 + Duration::from_secs(60)), None);
        assert_eq!(c.get_at(&"b", t0), None);
        c.put_at("b", 2, t0 + Duration::from_secs(61));
        assert_eq!(c.map.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_new_value_replaces_the_old() {
        let c = Ttl::new(Duration::from_secs(60));
        let t0 = Instant::now();
        c.put_at("a", 1, t0);
        c.put_at("a", 2, t0);
        assert_eq!(c.get_at(&"a", t0), Some(2));
    }
}
