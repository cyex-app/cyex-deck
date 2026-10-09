//! Random ids for GQL sessions and transactions. Not for secrets.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

/// `bytes` random bytes as lowercase hex.
pub fn hex(bytes: usize) -> String {
    let mut out = String::with_capacity(bytes * 2 + 16);
    while out.len() < bytes * 2 {
        // Each RandomState gets fresh keys.
        let mut h = RandomState::new().build_hasher();
        h.write_usize(out.len());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out.truncate(bytes * 2);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_ids_differ() {
        let (a, b) = (hex(16), hex(16));
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
