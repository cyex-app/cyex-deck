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

/// V107-Bn A random UUID v4 with dashes, like `crypto.randomUUID()`.
pub fn uuid() -> String {
    let mut h: Vec<char> = hex(16).chars().collect();
    h[12] = '4';
    h[16] = ['8', '9', 'a', 'b'][h[16].to_digit(16).unwrap_or(0) as usize % 4];
    let s: String = h.into_iter().collect();
    format!("{}-{}-{}-{}-{}", &s[..8], &s[8..12], &s[12..16], &s[16..20], &s[20..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_bn_uuid_is_v4() {
        let u = uuid();
        let parts: Vec<&str> = u.split('-').collect();
        assert_eq!(parts.iter().map(|p| p.len()).collect::<Vec<_>>(), [8, 4, 4, 4, 12]);
        assert!(parts[2].starts_with('4'));
        assert!(matches!(&parts[3][..1], "8" | "9" | "a" | "b"));
        assert_ne!(u, uuid());
    }

    #[test]
    fn hex_ids_differ() {
        let (a, b) = (hex(16), hex(16));
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
