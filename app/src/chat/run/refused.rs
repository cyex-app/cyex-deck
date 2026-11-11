//! V110-G2 An automation that is refused tells the chat, but a spam of triggers can't flood it:
//! one line per rule and room in 30 s.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

const EVERY_MS: i64 = 30_000;

#[derive(Default)]
pub struct Told(HashMap<(String, usize), i64>);

impl Told {
    /// True when the notice for `rule` in `channel` may show now (and counts as shown).
    pub fn due(&mut self, channel: &str, rule: usize, now_ms: i64) -> bool {
        let at = self.0.entry((channel.to_owned(), rule)).or_insert(i64::MIN);
        if now_ms.saturating_sub(*at) < EVERY_MS {
            return false;
        }
        *at = now_ms;
        true
    }
}

static TOLD: LazyLock<Mutex<Told>> = LazyLock::new(Mutex::default);

pub fn due(channel: &str, rule: usize, now_ms: i64) -> bool {
    TOLD.lock().unwrap_or_else(|p| p.into_inner()).due(channel, rule, now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v110_g2_one_notice_per_rule_and_room_in_30_seconds() {
        let mut t = Told::default();
        assert!(t.due("a", 0, 1_000));
        assert!(!t.due("a", 0, 5_000), "a spam of triggers");
        assert!(t.due("a", 1, 5_000), "another rule");
        assert!(t.due("b", 0, 5_000), "another room");
        assert!(!t.due("a", 0, 30_999));
        assert!(t.due("a", 0, 31_000));
    }
}
