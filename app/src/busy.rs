//! A flag held for the length of one job; dropping the guard frees it again.

use std::sync::atomic::{AtomicBool, Ordering};

pub struct Busy<'a>(&'a AtomicBool);

impl<'a> Busy<'a> {
    /// `None` when the flag is already set (another job is running).
    pub fn take(flag: &'a AtomicBool) -> Option<Self> {
        (!flag.swap(true, Ordering::SeqCst)).then_some(Self(flag))
    }
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_at_a_time() {
        let flag = AtomicBool::new(false);
        let a = Busy::take(&flag);
        assert!(a.is_some() && Busy::take(&flag).is_none());
        drop(a);
        assert!(Busy::take(&flag).is_some());
    }
}
