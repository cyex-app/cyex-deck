//! Which live channels get the watch slots (1.30, 1.33, 1.44). Twitch credits watch time
//! for two channels at once; like the reference script, the slots go to the two channels
//! picked first, then by the rules in order, then by the list.

use cyex_settings::schema::Rule;

/// Twitch credits watch time for this many channels at once.
pub const SLOTS: usize = 2;
/// A stream counts once it's been up this long (the script waits 30 s).
pub const SETTLE_MS: i64 = 30_000;
/// A break shorter than this keeps the old stream's watch streak.
const STREAK_BREAK_MS: i64 = 30 * 60_000;
/// Twitch grants the streak within the first few minutes of watching.
const STREAK_WATCH_SECS: u64 = 7 * 60;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Candidate {
    pub id: String,
    /// Live, and up for at least `SETTLE_MS`.
    pub ready: bool,
    pub streak_due: bool,
    pub drops: bool,
    /// Sum of active point multipliers.
    pub multiplier: f64,
    pub balance: u64,
}

/// Up to `slots` channel ids, from `list` in list order.
pub fn pick(list: &[Candidate], first: &[String], order: &[Rule], slots: usize) -> Vec<String> {
    let ready: Vec<&Candidate> = list.iter().filter(|c| c.ready).collect();
    let mut out = Vec::new();
    for id in first {
        if let Some(c) = ready.iter().find(|c| &c.id == id) {
            take(&mut out, c, slots);
        }
    }
    for rule in order {
        let mut matching: Vec<&Candidate> = match rule {
            Rule::Streak => ready.iter().copied().filter(|c| c.streak_due).collect(),
            Rule::Drops => ready.iter().copied().filter(|c| c.drops).collect(),
            Rule::Multiplier => ready.iter().copied().filter(|c| c.multiplier > 0.0).collect(),
            Rule::FewestPoints | Rule::MostPoints => ready.clone(),
        };
        match rule {
            Rule::Multiplier => matching.sort_by(|a, b| b.multiplier.total_cmp(&a.multiplier)),
            Rule::FewestPoints => matching.sort_by_key(|c| c.balance),
            Rule::MostPoints => matching.sort_by_key(|c| std::cmp::Reverse(c.balance)),
            _ => {}
        }
        for c in matching {
            take(&mut out, c, slots);
        }
    }
    for c in ready {
        take(&mut out, c, slots);
    }
    out
}

fn take(out: &mut Vec<String>, c: &Candidate, slots: usize) {
    if out.len() < slots && !out.contains(&c.id) {
        out.push(c.id.clone());
    }
}

/// Watch streak bookkeeping for one channel, the way the reference script guesses it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Streak {
    /// Not earned this stream yet.
    pub missing: bool,
    /// When the channel last went offline (ms); 0 if not seen.
    pub offline_at: i64,
    /// Watched this stream, in seconds.
    pub watched_secs: u64,
}

impl Default for Streak {
    fn default() -> Self {
        Self {
            missing: true,
            offline_at: 0,
            watched_secs: 0,
        }
    }
}

impl Streak {
    pub fn went_live(&mut self) {
        self.missing = true;
        self.watched_secs = 0;
    }

    pub fn went_offline(&mut self, now_ms: i64) {
        self.offline_at = now_ms;
    }

    pub fn earned(&mut self) {
        self.missing = false;
    }

    /// Worth a slot now: not earned yet, a real new stream (the break before it was long
    /// enough), and not watched long enough for Twitch to have granted it.
    pub fn due(&self, now_ms: i64) -> bool {
        self.missing
            && (self.offline_at == 0 || now_ms - self.offline_at > STREAK_BREAK_MS)
            && self.watched_secs < STREAK_WATCH_SECS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str) -> Candidate {
        Candidate {
            id: id.into(),
            ready: true,
            ..Candidate::default()
        }
    }

    #[test]
    fn list_order_fills_the_slots() {
        let list = [
            c("a"),
            Candidate {
                ready: false,
                ..c("b")
            },
            c("c"),
            c("d"),
        ];
        assert_eq!(pick(&list, &[], &[], SLOTS), ["a", "c"]);
        assert!(pick(&list, &[], &[], 0).is_empty());
    }

    #[test]
    fn first_then_rules_then_list() {
        let list = [
            c("a"),
            Candidate {
                drops: true,
                ..c("b")
            },
            Candidate {
                streak_due: true,
                ..c("c")
            },
            c("d"),
        ];
        let order = [Rule::Streak, Rule::Drops];
        assert_eq!(pick(&list, &[], &order, 2), ["c", "b"]);
        assert_eq!(pick(&list, &["d".into()], &order, 2), ["d", "c"]);
        // An offline first channel doesn't hold a slot.
        assert_eq!(pick(&list, &["x".into()], &order, 3), ["c", "b", "a"]);
    }

    #[test]
    fn points_and_multipliers_sort() {
        let list = [
            Candidate {
                balance: 50,
                ..c("a")
            },
            Candidate {
                balance: 10,
                multiplier: 0.2,
                ..c("b")
            },
            Candidate {
                balance: 90,
                multiplier: 1.0,
                ..c("c")
            },
        ];
        assert_eq!(pick(&list, &[], &[Rule::FewestPoints], 2), ["b", "a"]);
        assert_eq!(pick(&list, &[], &[Rule::MostPoints], 2), ["c", "a"]);
        assert_eq!(pick(&list, &[], &[Rule::Multiplier], 3), ["c", "b", "a"]);
    }

    #[test]
    fn streaks() {
        let mut s = Streak::default();
        assert!(s.due(1_000_000));
        s.watched_secs = 7 * 60;
        assert!(!s.due(1_000_000));
        s.went_offline(1_000_000);
        s.went_live();
        // Back after 10 minutes: same stream as far as streaks go.
        assert!(!s.due(1_000_000 + 10 * 60_000));
        assert!(s.due(1_000_000 + 31 * 60_000));
        s.earned();
        assert!(!s.due(1_000_000 + 31 * 60_000));
    }
}
