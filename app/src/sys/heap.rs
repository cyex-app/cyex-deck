//! V107-R2 Test builds only: the system allocator with
//! counters, so a soak can tell live heap from what the process holds (fragmentation,
//! allocator caches).

use std::alloc::{GlobalAlloc, Layout};
use std::alloc::System as Inner;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

pub struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static BLOCKS: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);

fn grew(n: usize) {
    let now = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(now, Relaxed);
}

// SAFETY: every call goes straight to `Inner`; the counters are plain atomics.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { Inner.alloc(l) };
        if !p.is_null() {
            grew(l.size());
            BLOCKS.fetch_add(1, Relaxed);
            TOTAL.fetch_add(1, Relaxed);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { Inner.alloc_zeroed(l) };
        if !p.is_null() {
            grew(l.size());
            BLOCKS.fetch_add(1, Relaxed);
            TOTAL.fetch_add(1, Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { Inner.dealloc(p, l) };
        LIVE.fetch_sub(l.size(), Relaxed);
        BLOCKS.fetch_sub(1, Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        let q = unsafe { Inner.realloc(p, l, new) };
        if !q.is_null() {
            if new >= l.size() {
                grew(new - l.size());
            } else {
                LIVE.fetch_sub(l.size() - new, Relaxed);
            }
            TOTAL.fetch_add(1, Relaxed);
        }
        q
    }
}

/// (live bytes, peak bytes, live blocks, allocations so far).
pub fn stats() -> (usize, usize, usize, usize) {
    (LIVE.load(Relaxed), PEAK.load(Relaxed), BLOCKS.load(Relaxed), TOTAL.load(Relaxed))
}
