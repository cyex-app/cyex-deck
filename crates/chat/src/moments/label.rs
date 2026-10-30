//! V108-N25 What a spike was about: the emote or word most messages in it repeated.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::model::{ChatMessage, Fragment};

/// How far back the spike's words are looked at, ms.
pub const SPAN_MS: i64 = 15_000;
/// Messages kept for it; a chat busier than this still labels from its newest.
const MAX_LINES: usize = 1_500;
/// A label needs this many messages saying it, and this share of the window's messages.
const MIN_COUNT: u32 = 3;
const MIN_SHARE: f32 = 0.15;
/// Longest word kept, in characters.
const MAX_LEN: usize = 24;

const STOP: &[&str] = &[
    "the", "and", "you", "for", "this", "that", "with", "are", "was", "its", "not", "but", "have", "has",
    "just", "what", "how", "can", "all", "his", "her", "they", "them", "from", "your", "who", "why", "out",
    "get", "got", "now", "yes", "will", "one", "too", "very", "than", "then", "when", "there", "here",
    "like", "really", "dont", "im", "it's", "i'm",
];

/// The distinct terms of one message: emote names as they are, words lowercased without
/// edge punctuation. Links, mentions and short or common words are left out.
pub fn terms(body: &[Fragment]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut add = |t: String, out: &mut Vec<String>| {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    };
    for f in body {
        match f {
            Fragment::Emote { name, .. } if !name.is_empty() => add(name.clone(), &mut out),
            Fragment::Text { text } => {
                for w in text.split_whitespace() {
                    let w = w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
                    let n = w.chars().count();
                    if (2..=MAX_LEN).contains(&n)
                        && !STOP.contains(&w.as_str())
                        && w.chars().any(char::is_alphanumeric)
                    {
                        add(w, &mut out);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// What a kept line's words are read from.
pub trait Body {
    fn body(&self) -> &[Fragment];
}

impl Body for Arc<ChatMessage> {
    fn body(&self) -> &[Fragment] {
        &self.body
    }
}

impl Body for Vec<Fragment> {
    fn body(&self) -> &[Fragment] {
        self
    }
}

/// The last lines, kept as they are (a shared message costs nothing more): their words are
/// read only when a spike asks for its label, not for every line.
#[derive(Debug)]
pub struct Recent<B = Arc<ChatMessage>> {
    lines: VecDeque<(i64, B)>,
}

impl<B> Default for Recent<B> {
    fn default() -> Self {
        Self { lines: VecDeque::new() }
    }
}

impl<B: Body> Recent<B> {
    pub fn record(&mut self, now_ms: i64, line: B) {
        self.lines.push_back((now_ms, line));
        self.prune(now_ms);
        while self.lines.len() > MAX_LINES {
            self.lines.pop_front();
        }
    }

    fn prune(&mut self, now_ms: i64) {
        while self.lines.front().is_some_and(|(t, _)| *t < now_ms - SPAN_MS) {
            self.lines.pop_front();
        }
    }

    /// The most repeated term in the last `SPAN_MS`, if enough messages share one. Ties go
    /// to the term seen first.
    pub fn top(&self, now_ms: i64) -> Option<String> {
        let live: Vec<Vec<String>> = self
            .lines
            .iter()
            .filter(|(t, _)| *t >= now_ms - SPAN_MS)
            .map(|(_, l)| terms(l.body()))
            .collect();
        if live.is_empty() {
            return None;
        }
        let mut counts: HashMap<&str, (u32, usize)> = HashMap::new();
        let mut order = 0usize;
        for terms in &live {
            for t in terms.iter() {
                let e = counts.entry(t.as_str()).or_insert_with(|| {
                    order += 1;
                    (0, order)
                });
                e.0 += 1;
            }
        }
        let need = MIN_COUNT.max((live.len() as f32 * MIN_SHARE).ceil() as u32);
        counts
            .into_iter()
            .filter(|(_, (n, _))| *n >= need)
            .max_by(|a, b| a.1.0.cmp(&b.1.0).then(b.1.1.cmp(&a.1.1)))
            .map(|(t, _)| t.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Vec<Fragment> {
        vec![Fragment::Text { text: s.to_owned() }]
    }

    #[test]
    fn terms_are_distinct_lowercase_and_skip_noise() {
        assert_eq!(terms(&text("LETS GO!! lets go, THE")), vec!["lets", "go"]);
        assert_eq!(terms(&text("a b I x")), Vec::<String>::new());
        assert_eq!(terms(&text("\u{1F602}\u{1F602} \u{1F602}")), Vec::<String>::new());
    }

    #[test]
    fn the_most_repeated_term_wins() {
        let mut r = Recent::<Vec<Fragment>>::default();
        for i in 0..10 {
            r.record(1_000 + i, text("KEKW so funny"));
        }
        for i in 0..3 {
            r.record(2_000 + i, text("pog"));
        }
        assert_eq!(r.top(3_000).as_deref(), Some("kekw"));
    }

    #[test]
    fn one_person_spamming_inside_a_message_counts_once() {
        let mut r = Recent::<Vec<Fragment>>::default();
        r.record(0, text("wow wow wow wow wow"));
        r.record(1, text("wow"));
        assert_eq!(r.top(2), None);
    }

    #[test]
    fn nothing_when_no_term_is_shared_enough() {
        let mut r = Recent::<Vec<Fragment>>::default();
        for (i, w) in ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"]
            .iter()
            .enumerate()
        {
            r.record(i as i64, text(w));
        }
        assert_eq!(r.top(10), None);
        assert_eq!(Recent::<Vec<Fragment>>::default().top(10), None);
    }

    #[test]
    fn old_lines_fall_out() {
        let mut r = Recent::<Vec<Fragment>>::default();
        for i in 0..5 {
            r.record(i, text("old news"));
        }
        for i in 0..4 {
            r.record(SPAN_MS + 100 + i, text("fresh take"));
        }
        assert_eq!(r.top(SPAN_MS + 200).as_deref(), Some("fresh"));
    }

    #[test]
    fn rtl_and_long_words_are_handled() {
        let long = "x".repeat(60);
        assert!(terms(&text(&long)).is_empty());
        assert_eq!(terms(&text("\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}")).len(), 1);
    }
}
