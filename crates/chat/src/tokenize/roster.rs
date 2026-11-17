//! V111-48 Names typed without `@`: who is known in a room, and which word is one of them.

use std::collections::{HashSet, VecDeque};

/// Most names a room keeps; the oldest go first.
pub const CAP: usize = 5_000;
/// Twitch logins are 25 characters at most.
const LONGEST: usize = 25;
const SHORTEST: usize = 3;

/// Logins (lowercase) of people who chatted or joined in one room. Bounded.
#[derive(Debug, Default)]
pub struct Roster {
    names: HashSet<Box<str>>,
    order: VecDeque<Box<str>>,
    cap: usize,
}

impl Roster {
    pub fn new() -> Self {
        Self::with_cap(CAP)
    }

    pub fn with_cap(cap: usize) -> Self {
        Self { names: HashSet::new(), order: VecDeque::new(), cap: cap.max(1) }
    }

    /// Adds `login` (any case). Names shorter than a match could be aren't kept. Known names
    /// cost one lookup and no allocation.
    pub fn add(&mut self, login: &str) {
        let n = login.len();
        if !(SHORTEST..=LONGEST).contains(&n) {
            return;
        }
        let mut buf = [0u8; LONGEST];
        let Some(lower) = lower(login, &mut buf) else { return };
        if self.names.contains(lower) {
            return;
        }
        if self.order.len() >= self.cap
            && let Some(old) = self.order.pop_front()
        {
            self.names.remove(&old);
        }
        let b: Box<str> = lower.into();
        self.names.insert(b.clone());
        self.order.push_back(b);
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    fn has(&self, lower: &str) -> bool {
        self.names.contains(lower)
    }
}

/// `login` lowercased into `buf`, if it is ASCII name characters only.
fn lower<'b>(login: &str, buf: &'b mut [u8; LONGEST]) -> Option<&'b str> {
    let n = login.len();
    if n > LONGEST || !login.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    for (o, b) in buf.iter_mut().zip(login.bytes()) {
        *o = b.to_ascii_lowercase();
    }
    std::str::from_utf8(&buf[..n]).ok()
}

/// What the tokenizer asks: the room's names, and one login that never matches (the sender).
#[derive(Clone, Copy)]
pub struct Known<'a> {
    pub roster: &'a Roster,
    pub sender: &'a str,
}

impl Known<'_> {
    /// `word` without a trailing `, . ! ? : ;` run, when that is a known name: the name as
    /// typed and the lowercase login. No allocation unless it matches.
    pub fn find<'w>(&self, word: &'w str) -> Option<(&'w str, String)> {
        let core = word.trim_end_matches([',', '.', '!', '?', ':', ';']);
        if !(SHORTEST..=LONGEST).contains(&core.len()) || self.roster.is_empty() {
            return None;
        }
        let mut buf = [0u8; LONGEST];
        let l = lower(core, &mut buf)?;
        (self.roster.has(l) && !l.eq_ignore_ascii_case(self.sender)).then(|| (core, l.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_oldest_out() {
        let mut r = Roster::with_cap(3);
        for n in ["aaa", "bbb", "ccc", "ddd"] {
            r.add(n);
        }
        assert_eq!(r.len(), 3);
        let k = Known { roster: &r, sender: "" };
        assert!(k.find("aaa").is_none());
        assert!(k.find("ddd").is_some());
        r.add("DDD");
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn keeps_only_possible_names() {
        let mut r = Roster::new();
        for n in ["ab", "ünï", "has space", "abcdefghijklmnopqrstuvwxyz"] {
            r.add(n);
        }
        assert!(r.is_empty());
    }

    #[test]
    fn finds_whole_words_only() {
        let mut r = Roster::new();
        r.add("Foo_Bar");
        let k = Known { roster: &r, sender: "me_" };
        assert_eq!(k.find("FOO_bar,"), Some(("FOO_bar", "foo_bar".to_owned())));
        assert!(k.find("foo_bar?!").is_some());
        assert!(k.find("foo").is_none());
        assert!(k.find("(foo_bar)").is_none());
        assert!(k.find("foo_bar's").is_none());
        let own = Known { roster: &r, sender: "FOO_BAR" };
        assert!(own.find("foo_bar").is_none());
    }
}
