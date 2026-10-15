//! V105-20 App versions as GitHub tags write them: `v0.1.5`, `0.1.04` (read as 0.1.4),
//! `0.2.0-beta.2`. Ordered like semver; leading zeros allowed.

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ident {
    Num(u64),
    Text(String),
}

impl Ord for Ident {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Ident::Num(a), Ident::Num(b)) => a.cmp(b),
            (Ident::Num(_), Ident::Text(_)) => Ordering::Less,
            (Ident::Text(_), Ident::Num(_)) => Ordering::Greater,
            (Ident::Text(a), Ident::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                // Pairwise, then the longer list is greater (Vec's order does both).
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (i, id) in self.pre.iter().enumerate() {
            f.write_str(if i == 0 { "-" } else { "." })?;
            match id {
                Ident::Num(n) => write!(f, "{n}")?,
                Ident::Text(t) => f.write_str(t)?,
            }
        }
        Ok(())
    }
}

fn number(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// `v0.1.5`, `0.1.04`, `0.2.0-beta.2`, `1.0.0+build`; anything else is `None`.
pub fn parse(s: &str) -> Option<Version> {
    let s = s.trim();
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let s = s.split('+').next()?;
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (s, None),
    };
    let mut parts = core.split('.');
    let major = number(parts.next()?)?;
    let minor = number(parts.next()?)?;
    let patch = number(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    let pre = match pre {
        None => Vec::new(),
        Some(p) => p
            .split('.')
            .map(|id| {
                if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                    None
                } else if let Some(n) = number(id) {
                    Some(Ident::Num(n))
                } else {
                    Some(Ident::Text(id.to_owned()))
                }
            })
            .collect::<Option<Vec<_>>>()?,
    };
    Some(Version { major, minor, patch, pre })
}

/// The running app's version.
pub fn current() -> Version {
    parse(env!("CARGO_PKG_VERSION")).expect("the crate version is a version")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        parse(s).unwrap_or_else(|| panic!("{s}"))
    }

    #[test]
    fn leading_zeros_read_as_numbers() {
        assert_eq!(v("v0.1.04"), v("0.1.4"));
        assert!(v("0.1.04") < v("0.1.5"));
        assert!(v("0.1.5") > v("0.1.4"));
        assert_eq!(v("0.1.5"), v("v0.1.5"));
    }

    #[test]
    fn pre_releases_order_like_semver() {
        assert!(v("0.1.5-beta") < v("0.1.5"));
        assert!(v("0.1.5-beta.2") < v("0.1.5-beta.10"));
        assert!(v("0.1.5-alpha") < v("0.1.5-beta"));
        assert!(v("0.1.5-1") < v("0.1.5-alpha"));
        assert!(v("0.1.5-beta") < v("0.1.5-beta.1"));
        assert_eq!(v("1.0.0+build"), v("1.0.0"));
    }

    #[test]
    fn junk_is_none() {
        for s in ["latest", "0.1", "v", "0.1.x", "", "1.2.3.4", "1.2.3-", "1.2.3-a..b", "-1.2.3"] {
            assert!(parse(s).is_none(), "{s}");
        }
    }

    #[test]
    fn display_round_trips() {
        assert_eq!(v("0.1.04").to_string(), "0.1.4");
        assert_eq!(v("v0.2.0-beta.2").to_string(), "0.2.0-beta.2");
        assert_eq!(v(&v("1.2.3-rc.1").to_string()), v("1.2.3-rc.1"));
        assert!(current().to_string().starts_with("0."));
    }
}
