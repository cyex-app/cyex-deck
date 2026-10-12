//! V104-01, V104-02 Mentions ready to match. Built once per settings change and run on
//! every message, also the firehose's few hundred a second, so: plain ASCII words are a
//! byte search with no regex and no allocation, other words and regexes go to
//! `fancy-regex` (which hands anything without lookaround or backreferences to the
//! regex crate's automata), and text is folded for look-alikes only when it has a
//! non-ASCII character and some mention asks for it.

use cyex_settings::schema::{MentionAlert, MentionRule as Mention};
use fancy_regex::{Regex, RegexBuilder};
use serde::Serialize;
use ts_rs::TS;

use super::flavor::{self, Translated};
use crate::lookalike;

/// Steps a regex may take on one message before it counts as no match.
const BACKTRACK_LIMIT: usize = 100_000;
/// Memory one compiled pattern may use.
const SIZE_LIMIT: usize = 1 << 20;

enum Find {
    /// ASCII words: bytes compared directly, ASCII letters in either case unless
    /// case-sensitive.
    Ascii {
        needle: Box<[u8]>,
        case_sensitive: bool,
    },
    Regex(Regex),
}

pub struct Compiled {
    find: Find,
    whole_word: bool,
    fold: bool,
    /// Where it is in its settings list.
    pub index: usize,
    pub alert: MentionAlert,
}

/// What the editor shows under a pattern.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MentionCheck {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<String>,
    /// Works, but differently than in the flavor picked.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub note: Option<String>,
}

/// Whether a mention compiles, and what to tell the user about it.
pub fn check(m: &Mention) -> MentionCheck {
    if m.pattern.trim().is_empty() {
        return MentionCheck::default();
    }
    match compile(m, 0) {
        Ok((_, note)) => MentionCheck {
            error: None,
            note: note.map(str::to_owned),
        },
        Err(e) => MentionCheck {
            error: Some(e),
            note: None,
        },
    }
}

fn compile(m: &Mention, index: usize) -> Result<(Compiled, Option<&'static str>), String> {
    let pattern = if m.lookalikes {
        lookalike::fold(&m.pattern)
    } else {
        m.pattern.clone()
    };
    let mut note = None;
    let find = if !m.regex && pattern.is_ascii() {
        let needle: Box<[u8]> = pattern.trim().as_bytes().into();
        Find::Ascii {
            needle,
            case_sensitive: m.case_sensitive,
        }
    } else {
        let t = if m.regex {
            flavor::translate(&pattern, m.flavor)?
        } else {
            Translated {
                pattern: regex::escape(pattern.trim()),
                ignore_case: false,
                note: None,
            }
        };
        note = t.note;
        let body = if m.whole_word {
            format!(r"(?:^|[^\w])(?:{})(?:[^\w]|$)", t.pattern)
        } else {
            t.pattern
        };
        let re = RegexBuilder::new(&body)
            .case_insensitive(t.ignore_case || !m.case_sensitive)
            .backtrack_limit(BACKTRACK_LIMIT)
            .delegate_size_limit(SIZE_LIMIT)
            .delegate_dfa_size_limit(SIZE_LIMIT)
            .build()
            .map_err(|e| short_error(&e.to_string()))?;
        Find::Regex(re)
    };
    Ok((
        Compiled {
            find,
            // The regex carries its own word edges.
            whole_word: m.whole_word && !m.regex && pattern.is_ascii(),
            fold: m.lookalikes,
            index,
            alert: m.alert.clone(),
        },
        note,
    ))
}

/// fancy-regex's messages start with "Parsing error at position N: ".
fn short_error(e: &str) -> String {
    let e = e.split_once(": ").map_or(e, |(_, rest)| rest);
    let mut s = e.trim().to_owned();
    if let Some(first) = s.get(..1) {
        s.replace_range(..1, &first.to_uppercase());
    }
    if !s.ends_with('.') {
        s.push('.');
    }
    s
}

impl Compiled {
    fn matches(&self, text: &str) -> bool {
        match &self.find {
            Find::Ascii {
                needle,
                case_sensitive,
            } => find_ascii(text, needle, *case_sensitive, |s, e| {
                !self.whole_word || edges(text, s, e)
            }),
            // A pattern that runs too long counts as no match.
            Find::Regex(re) => re.is_match(text).unwrap_or(false),
        }
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Not inside a longer word: no letter, digit or `_` right before or after.
fn edges(text: &str, start: usize, end: usize) -> bool {
    !text[..start].chars().next_back().is_some_and(is_word)
        && !text[end..].chars().next().is_some_and(is_word)
}

/// ASCII bytes never occur inside a multi-byte character, so a byte search finds only
/// whole characters, at character boundaries.
fn find_ascii(
    text: &str,
    needle: &[u8],
    case_sensitive: bool,
    mut ok: impl FnMut(usize, usize) -> bool,
) -> bool {
    let hay = text.as_bytes();
    let n = needle.len();
    if n == 0 || hay.len() < n {
        return false;
    }
    let first = needle[0];
    let (lo, hi) = if case_sensitive {
        (first, first)
    } else {
        (first.to_ascii_lowercase(), first.to_ascii_uppercase())
    };
    for i in 0..=hay.len() - n {
        let b = hay[i];
        if b != lo && b != hi {
            continue;
        }
        let window = &hay[i..i + n];
        let same = if case_sensitive {
            window == needle
        } else {
            window.eq_ignore_ascii_case(needle)
        };
        if same && ok(i, i + n) {
            return true;
        }
    }
    false
}

/// A list of mentions, compiled. Ones that are off, empty or don't compile are left out.
#[derive(Default)]
pub struct MentionSet {
    rules: Vec<Compiled>,
    fold: bool,
}

impl MentionSet {
    pub fn new(list: &[Mention]) -> Self {
        let rules: Vec<Compiled> = list
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled && !m.pattern.trim().is_empty())
            .filter_map(|(i, m)| compile(m, i).ok().map(|(c, _)| c))
            .collect();
        let fold = rules.iter().any(|r| r.fold);
        Self { rules, fold }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Calls `hit` for every mention in `text`, in list order. `scratch` holds the
    /// folded text when that's needed, so a caller on a hot path can keep one.
    pub fn each_match<'a>(&'a self, text: &str, scratch: &mut String, mut hit: impl FnMut(&'a Compiled)) {
        let folded = if self.fold && !text.is_ascii() {
            lookalike::fold_into(text, scratch);
            Some(scratch.as_str())
        } else {
            None
        };
        for r in &self.rules {
            let t = if r.fold { folded.unwrap_or(text) } else { text };
            if r.matches(t) {
                hit(r);
            }
        }
    }

    /// The first mention in `text`.
    pub fn first<'a>(&'a self, text: &str, scratch: &mut String) -> Option<&'a Compiled> {
        let mut found = None;
        // Short lists: stopping early isn't worth a second loop.
        self.each_match(text, scratch, |r| {
            found.get_or_insert(r);
        });
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_settings::schema::RegexFlavor;

    fn m(pattern: &str) -> Mention {
        Mention {
            pattern: pattern.into(),
            ..Mention::default()
        }
    }

    fn hits(list: &[Mention], text: &str) -> Vec<usize> {
        let set = MentionSet::new(list);
        let mut out = Vec::new();
        set.each_match(text, &mut String::new(), |r| out.push(r.index));
        out
    }

    #[test]
    fn words() {
        let list = [m("cyex")];
        assert_eq!(hits(&list, "hi CYEX!"), [0]);
        assert_eq!(hits(&list, "cyexdeck"), [] as [usize; 0], "whole word by default");
        assert_eq!(
            hits(&list, "über cyex_x"),
            [] as [usize; 0],
            "_ is part of a word"
        );
        assert_eq!(hits(&list, "über cyex"), [0], "other letters before it are fine");
        let loose = Mention {
            whole_word: false,
            ..m("cyex")
        };
        assert_eq!(hits(&[loose], "cyexdeck"), [0]);
        let exact = Mention {
            case_sensitive: true,
            ..m("Cyex")
        };
        assert_eq!(hits(std::slice::from_ref(&exact), "cyex Cyex"), [0]);
        assert_eq!(hits(&[exact], "cyex"), [] as [usize; 0]);
        assert_eq!(hits(&[m("@me")], "x @me y"), [0], "punctuation in the pattern");
        assert_eq!(
            hits(&[m("Grüße")], "GRÜSSE grüße"),
            [0],
            "non-ASCII words ignore case too"
        );
    }

    #[test]
    fn lookalikes() {
        let fold = Mention {
            lookalikes: true,
            ..m("paypal")
        };
        assert_eq!(hits(std::slice::from_ref(&fold), "free раураl here"), [0]);
        assert_eq!(hits(&[m("paypal")], "free раураl here"), [] as [usize; 0]);
        // A pattern typed with look-alikes matches plain text too.
        let typed = Mention {
            lookalikes: true,
            ..m("рауpal")
        };
        assert_eq!(hits(&[typed], "PAYPAL"), [0]);
    }

    #[test]
    fn regexes() {
        let re = |p: &str, flavor| Mention {
            regex: true,
            flavor,
            whole_word: false,
            ..m(p)
        };
        assert_eq!(
            hits(&[re(r"(\w)\1", RegexFlavor::Rust)], "hello"),
            [0],
            "backreference"
        );
        assert_eq!(
            hits(&[re(r"(?<!no )cyex", RegexFlavor::Rust)], "no cyex"),
            [] as [usize; 0]
        );
        assert_eq!(hits(&[re(r"/^CY\d+$/i", RegexFlavor::JavaScript)], "cy42"), [0]);
        assert_eq!(
            hits(&[re(r"\d", RegexFlavor::JavaScript)], "٣"),
            [] as [usize; 0],
            "JS \\d is ASCII"
        );
        assert_eq!(
            hits(&[re(r"\d", RegexFlavor::Python)], "٣"),
            [0],
            "Python \\d is Unicode"
        );
        assert_eq!(hits(&[re(r"(?P<a>x)(?P=a)", RegexFlavor::Python)], "axxb"), [0]);
        assert_eq!(
            hits(&[re(r"\bcafe\b", RegexFlavor::JavaScript)], "écafe"),
            [0],
            "JS \\b is ASCII"
        );
        assert_eq!(
            hits(&[re(r"\bcafe\b", RegexFlavor::Rust)], "écafe"),
            [] as [usize; 0]
        );
        let word = Mention {
            whole_word: true,
            ..re(r"cy.x", RegexFlavor::Rust)
        };
        assert_eq!(hits(std::slice::from_ref(&word), "a cyex b"), [0]);
        assert_eq!(hits(&[word], "acyexb"), [] as [usize; 0]);
        let slow = re(r"^(a+)+\1b$", RegexFlavor::Rust);
        assert_eq!(
            hits(&[slow], &"a".repeat(40)),
            [] as [usize; 0],
            "runaway backtracking stops"
        );
    }

    #[test]
    fn checks() {
        let bad = Mention {
            regex: true,
            ..m("(")
        };
        assert!(check(&bad).error.is_some());
        assert_eq!(hits(&[bad, m("x")], "x"), [1], "a broken one is skipped");
        let py = Mention {
            regex: true,
            flavor: RegexFlavor::Python,
            ..m(r"(?a)\w+")
        };
        let c = check(&py);
        assert!(c.error.is_none() && c.note.is_some());
        assert_eq!(check(&m("")), MentionCheck::default());
    }

    #[test]
    fn off_and_empty_are_left_out() {
        let off = Mention {
            enabled: false,
            ..m("x")
        };
        assert!(MentionSet::new(&[off, m("  ")]).is_empty());
    }
}
