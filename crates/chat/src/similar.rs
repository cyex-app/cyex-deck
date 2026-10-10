//! F6 Near-duplicates (Chatterino's local R9K) and C2 repeat counts (moltorino's
//! streaks). Both compare a sender's message with their own recent ones.

use std::collections::HashMap;

/// F6 Settings, in the room's terms.
#[derive(Clone, Debug, PartialEq)]
pub struct SimilarCheck {
    /// Greater than this counts, 0 to 1.
    pub threshold: f32,
    pub max_age_ms: i64,
    /// How many earlier messages to compare with.
    pub depth: usize,
    /// Only the same sender's messages.
    pub same_user: bool,
}

/// C2 Settings.
#[derive(Clone, Debug, PartialEq)]
pub struct RepeatCheck {
    /// 0 to 1; 1 means only the same text.
    pub threshold: f32,
    /// Shown from this many in a row.
    pub min: u32,
}

/// Longest common substring over the longer length (Chatterino).
pub fn lcs_ratio(a: &str, b: &str) -> f32 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    let mut prev = vec![0usize; b.len() + 1];
    let mut best = 0;
    for x in &a {
        let mut row = vec![0usize; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            if x == y {
                row[j + 1] = prev[j] + 1;
                best = best.max(row[j + 1]);
            }
        }
        prev = row;
    }
    best as f32 / longest as f32
}

/// Case folded, without the invisible characters used to dodge Twitch's duplicate
/// check, spaces collapsed, at most 512 characters (moltorino).
pub fn normalize(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        let word: String = word
            .chars()
            .filter(|c| *c != '\u{34F}' && !('\u{E0000}'..='\u{E007F}').contains(c))
            .flat_map(char::to_lowercase)
            .collect();
        if word.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&word);
    }
    out.chars().take(512).collect()
}

/// Sørensen–Dice over character pairs, counting repeats.
pub fn dice(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    let pairs = |s: &str| {
        let c: Vec<char> = s.chars().collect();
        let mut m: HashMap<(char, char), u32> = HashMap::new();
        for w in c.windows(2) {
            *m.entry((w[0], w[1])).or_default() += 1;
        }
        (m, c.len().saturating_sub(1))
    };
    let ((pa, na), (mut pb, nb)) = (pairs(a), pairs(b));
    if na == 0 || nb == 0 {
        return 0.0;
    }
    let mut common = 0;
    for (k, n) in pa {
        if let Some(m) = pb.get_mut(&k) {
            let take = n.min(*m);
            common += take;
            *m -= take;
        }
    }
    2.0 * common as f32 / (na + nb) as f32
}

/// A sender is gone from the counter after this long without a repeat.
const STREAK_TTL_MS: i64 = 5 * 60_000;
/// Misses a running streak survives.
const MISSES: u32 = 2;
const MAX_USERS: usize = 2048;

struct Streak {
    text: String,
    count: u32,
    misses: u32,
    at_ms: i64,
}

/// C2 Per-sender streaks of the same message.
#[derive(Default)]
pub struct Repeats {
    users: HashMap<String, Streak>,
}

impl Repeats {
    /// Counts the message; returns the count once it's a repeat worth showing.
    pub fn count(&mut self, check: &RepeatCheck, user: &str, text: &str, now_ms: i64) -> Option<u32> {
        let text = normalize(text);
        if text.is_empty() {
            return None;
        }
        if self.users.len() >= MAX_USERS && !self.users.contains_key(user) {
            self.users.retain(|_, s| now_ms - s.at_ms < STREAK_TTL_MS);
            if self.users.len() >= MAX_USERS {
                return None;
            }
        }
        let s = self.users.entry(user.to_owned()).or_insert(Streak {
            text: String::new(),
            count: 0,
            misses: 0,
            at_ms: now_ms,
        });
        let fresh = now_ms - s.at_ms < STREAK_TTL_MS;
        let same = fresh
            && !s.text.is_empty()
            && (s.text == text || (check.threshold < 1.0 && dice(&s.text, &text) >= check.threshold));
        let running = s.count >= check.min.max(2);
        if same {
            s.count += 1;
            s.misses = 0;
            s.at_ms = now_ms;
            return (s.count >= check.min.max(2)).then_some(s.count);
        }
        if running && fresh && s.misses < MISSES {
            s.misses += 1;
            return None;
        }
        *s = Streak {
            text,
            count: 1,
            misses: 0,
            at_ms: now_ms,
        };
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores() {
        assert!((lcs_ratio("hello world", "hello world!") - 11.0 / 12.0).abs() < 1e-6);
        assert_eq!(lcs_ratio("", ""), 1.0);
        assert_eq!(normalize("  KEKW \u{34F} kekw  "), "kekw kekw");
        assert_eq!(dice("night", "nacht"), 0.25);
        assert_eq!(dice("a", "b"), 0.0);
    }

    #[test]
    fn streaks_count_and_survive_misses() {
        let check = RepeatCheck {
            threshold: 0.8,
            min: 2,
        };
        let mut r = Repeats::default();
        assert_eq!(r.count(&check, "1", "buy followers now", 0), None);
        assert_eq!(r.count(&check, "1", "buy followers now!", 1), Some(2));
        assert_eq!(r.count(&check, "2", "buy followers now", 2), None, "per sender");
        assert_eq!(r.count(&check, "1", "something else", 3), None);
        assert_eq!(r.count(&check, "1", "BUY followers now", 4), Some(3));
        assert_eq!(r.count(&check, "1", "x", 5), None);
        assert_eq!(r.count(&check, "1", "y", 6), None);
        assert_eq!(r.count(&check, "1", "z", 7), None, "third miss resets");
        assert_eq!(r.count(&check, "1", "buy followers now", 8), None);
        assert_eq!(
            r.count(&check, "1", "buy followers now", 8 + STREAK_TTL_MS),
            None,
            "too late"
        );
    }
}
