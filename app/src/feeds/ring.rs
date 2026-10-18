//! V106-07 A capped list for a session feed. Pure, so it's tested without the app.

use std::collections::VecDeque;

pub struct Ring<T> {
    items: VecDeque<T>,
    cap: usize,
}

impl<T: Clone> Ring<T> {
    pub fn new(cap: usize) -> Self {
        Self {
            items: VecDeque::new(),
            cap,
        }
    }

    /// Adds at the end. Past the cap drops the oldest item `keep` doesn't protect, else the oldest.
    /// Returns the dropped item.
    pub fn push(&mut self, item: T, keep: impl Fn(&T) -> bool) -> Option<T> {
        self.items.push_back(item);
        if self.items.len() <= self.cap {
            return None;
        }
        let at = self.items.iter().position(|i| !keep(i)).unwrap_or(0);
        self.items.remove(at)
    }

    /// Changes the first item `hit` finds; returns the changed copy.
    pub fn update(&mut self, hit: impl Fn(&T) -> bool, f: impl FnOnce(&mut T)) -> Option<T> {
        let item = self.items.iter_mut().find(|i| hit(i))?;
        f(item);
        Some(item.clone())
    }

    /// Drops what `gone` names; returns the dropped items.
    pub fn remove(&mut self, gone: impl Fn(&T) -> bool) -> Vec<T> {
        let mut dropped = Vec::new();
        self.items.retain(|i| {
            if gone(i) {
                dropped.push(i.clone());
                false
            } else {
                true
            }
        });
        dropped
    }

    pub fn items(&self) -> Vec<T> {
        self.items.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// R18 Oldest first.
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.items.iter()
    }

    #[cfg(test)]
    pub fn front(&self) -> Option<&T> {
        self.items.front()
    }

    #[cfg(test)]
    pub fn back(&self) -> Option<&T> {
        self.items.back()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_drops_the_oldest() {
        let mut r = Ring::new(2);
        assert_eq!(r.push(1, |_| false), None);
        r.push(2, |_| false);
        assert_eq!(r.push(3, |_| false), Some(1));
        assert_eq!(r.items(), [2, 3]);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn keep_protects() {
        let mut r = Ring::new(2);
        r.push(1, |_| false);
        r.push(2, |_| false);
        // Odd numbers are protected: 2 goes.
        assert_eq!(r.push(3, |i| i % 2 == 1), Some(2));
        assert_eq!(r.items(), [1, 3]);
        // All protected: the oldest goes.
        assert_eq!(r.push(5, |i| i % 2 == 1), Some(1));
        assert_eq!(r.items(), [3, 5]);
    }

    #[test]
    fn update_remove_clear() {
        let mut r = Ring::new(5);
        for i in 1..=4 {
            r.push(i, |_| false);
        }
        assert_eq!(r.update(|i| *i == 2, |i| *i = 20), Some(20));
        assert_eq!(r.update(|i| *i == 9, |i| *i = 90), None);
        assert_eq!(r.remove(|i| *i > 3), [20, 4]);
        assert_eq!(r.items(), [1, 3]);
        r.clear();
        assert_eq!(r.len(), 0);
    }
}
