//! S7 A map that keeps its most recently used entries: each insert and read stamps a
//! tick, and past `KEEP` plus a tenth the least recent go, down to `KEEP` (V107-R1/M3).
//! Reads stamp through `&self`, so drawing a user counts as using them.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub struct Capped<K, V, const KEEP: usize> {
    map: HashMap<K, (V, AtomicU64)>,
    tick: AtomicU64,
}

impl<K, V, const KEEP: usize> Default for Capped<K, V, KEEP> {
    fn default() -> Self {
        Capped {
            map: HashMap::new(),
            tick: AtomicU64::new(0),
        }
    }
}

impl<K: Eq + Hash + Clone, V, const KEEP: usize> Capped<K, V, KEEP> {
    fn next(&self) -> u64 {
        self.tick.fetch_add(1, Relaxed) + 1
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Without counting as a use.
    pub fn contains_key<Q: Hash + Eq + ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.map.contains_key(k)
    }

    /// Reads and counts as a use.
    pub fn get<Q: Hash + Eq + ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        let (v, seen) = self.map.get(k)?;
        seen.store(self.next(), Relaxed);
        Some(v)
    }

    /// Counts as a use; false when it isn't there.
    pub fn touch<Q: Hash + Eq + ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.get(k).is_some()
    }

    pub fn get_mut<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
    {
        let tick = self.next();
        let (v, seen) = self.map.get_mut(k)?;
        *seen.get_mut() = tick;
        Some(v)
    }

    /// The entry, made with `V::default()` when missing; counts as a use.
    pub fn get_or_default(&mut self, k: K) -> &mut V
    where
        V: Default,
    {
        let tick = self.next();
        let (v, seen) = self.map.entry(k).or_insert_with(|| (V::default(), AtomicU64::new(0)));
        *seen.get_mut() = tick;
        v
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let tick = AtomicU64::new(self.next());
        self.map.insert(k, (v, tick)).map(|(v, _)| v)
    }

    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.map.remove(k).map(|(v, _)| v)
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.map.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.map.values().map(|(v, _)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter().map(|(k, (v, _))| (k, v))
    }

    /// Past `KEEP` plus a tenth: drops the least recently used down to `KEEP`, and gives
    /// them back.
    pub fn trim(&mut self) -> Vec<(K, V)> {
        if self.map.len() > KEEP + KEEP / 10 {
            self.trim_to(KEEP)
        } else {
            Vec::new()
        }
    }

    /// Drops the least recently used down to `keep`, and gives them back.
    pub fn trim_to(&mut self, keep: usize) -> Vec<(K, V)> {
        if self.map.len() <= keep {
            return Vec::new();
        }
        let mut seen: Vec<u64> = self.map.values().map(|(_, s)| s.load(Relaxed)).collect();
        let cut = seen.len() - keep;
        let (_, &mut oldest_kept, _) = seen.select_nth_unstable(cut);
        let gone: Vec<K> = self
            .map
            .iter()
            .filter(|(_, (_, s))| s.load(Relaxed) < oldest_kept)
            .map(|(k, _)| k.clone())
            .collect();
        gone.into_iter()
            .filter_map(|k| self.map.remove(&k).map(|(v, _)| (k, v)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_recent_with_some_slack() {
        let mut c: Capped<u32, (), 10> = Capped::default();
        for i in 0..11 {
            c.insert(i, ());
        }
        assert!(c.trim().is_empty(), "within the slack");
        c.insert(11, ());
        c.touch(&0);
        let gone = c.trim();
        assert_eq!(c.len(), 10);
        assert_eq!(gone.len(), 2);
        assert!(c.contains_key(&0), "read last, so kept");
        assert!(!c.contains_key(&1) && !c.contains_key(&2));
    }

    #[test]
    fn writes_count_as_uses() {
        let mut c: Capped<&str, Vec<u8>, 100> = Capped::default();
        c.get_or_default("a").push(1);
        c.insert("b", Vec::new());
        c.get_mut("a").unwrap().push(2);
        assert_eq!(c.trim_to(1), [("b", Vec::new())]);
        assert_eq!(c.get("a"), Some(&vec![1, 2]));
        assert!(c.remove("a").is_some() && c.is_empty());
    }
}
