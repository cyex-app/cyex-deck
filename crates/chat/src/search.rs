//! F1 Search with Chatterino's operators: `from:`, `in:`, `badge:`, `subtier:`,
//! `has:link`, `is:`, `regex:` and plain words. Everything must match; commas inside one
//! operator mean any of them (`from:a,b`); `!` or `-` before an operator turns it around.

use regex::{Regex, RegexBuilder};

use crate::model::{ChatMessage, Flag, Fragment, NoticeKind, plain};

#[derive(Debug)]
enum Term {
    From(Vec<String>),
    In(Vec<String>),
    Badge(Vec<String>),
    SubTier(Vec<u8>),
    HasLink,
    Is(Vec<String>),
    Regex(Option<Regex>),
    Word(String),
}

#[derive(Debug, Default)]
pub struct Query {
    terms: Vec<(Term, bool)>,
}

impl Query {
    pub fn parse(text: &str) -> Self {
        let mut terms = Vec::new();
        for token in tokens(text) {
            let (negated, body) = match token.strip_prefix(['!', '-']) {
                Some(rest) if rest.contains(':') => (true, rest),
                _ => (false, token.as_str()),
            };
            let Some((name, value)) = body
                .split_once(':')
                .filter(|(n, v)| !v.is_empty() && n.chars().all(char::is_alphanumeric))
            else {
                terms.push((Term::Word(token.to_lowercase()), false));
                continue;
            };
            let value = value.trim_matches('"');
            let list = || {
                value
                    .split(',')
                    .filter(|v| !v.is_empty())
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
            };
            let term = match name.to_ascii_lowercase().as_str() {
                "from" => Term::From(
                    list()
                        .into_iter()
                        .map(|v| v.trim_start_matches('@').to_owned())
                        .collect(),
                ),
                "in" => Term::In(
                    list()
                        .into_iter()
                        .map(|v| v.trim_start_matches('#').to_owned())
                        .collect(),
                ),
                "badge" => Term::Badge(
                    list()
                        .into_iter()
                        .map(|b| match b.as_str() {
                            "mod" => "moderator".into(),
                            "sub" => "subscriber".into(),
                            "prime" => "premium".into(),
                            _ => b,
                        })
                        .collect(),
                ),
                "subtier" => Term::SubTier(list().iter().filter_map(|t| t.parse().ok()).collect()),
                "has" if value.eq_ignore_ascii_case("link") => Term::HasLink,
                "is" => Term::Is(list()),
                "regex" => Term::Regex(RegexBuilder::new(value).case_insensitive(true).build().ok()),
                _ => Term::Word(token.to_lowercase()),
            };
            terms.push((term, negated));
        }
        Self { terms }
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// `channel` is the login the message was seen in.
    pub fn matches(&self, m: &ChatMessage, channel: &str) -> bool {
        let text = plain(&m.body);
        self.terms
            .iter()
            .all(|(t, negated)| term(t, m, channel, &text) != *negated)
    }
}

/// Splits on spaces, keeping `name:"two words"` together.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn term(t: &Term, m: &ChatMessage, channel: &str, text: &str) -> bool {
    let s = m.sender.as_ref();
    match t {
        Term::From(names) => s.is_some_and(|s| {
            names
                .iter()
                .any(|n| s.login == *n || s.display.to_lowercase() == *n)
        }),
        Term::In(chans) => chans.iter().any(|c| c == channel),
        Term::Badge(sets) => s.is_some_and(|s| s.badges.iter().any(|b| sets.contains(&b.set))),
        Term::SubTier(tiers) => s
            .and_then(|s| s.badges.iter().find(|b| b.set == "subscriber"))
            .is_some_and(|b| tiers.contains(&tier(&b.version))),
        Term::HasLink => m.body.iter().any(|f| matches!(f, Fragment::Link { .. })),
        Term::Is(kinds) => kinds.iter().any(|k| is(k, m, kinds)),
        Term::Regex(re) => re.as_ref().is_some_and(|re| re.is_match(text)),
        Term::Word(w) => {
            let names = s
                .map(|s| format!("{} {}", s.display, s.login))
                .unwrap_or_default();
            let notice = m.notice.as_ref().map(|n| n.text.as_str()).unwrap_or_default();
            format!("{names}: {notice} {text}")
                .to_lowercase()
                .contains(w.as_str())
        }
    }
}

/// `3012` → tier 3; plain months versions are tier 1.
fn tier(version: &str) -> u8 {
    if version.len() <= 3 {
        1
    } else {
        version[..1].parse().unwrap_or(1)
    }
}

fn is(kind: &str, m: &ChatMessage, all: &[String]) -> bool {
    let notice = m.notice.as_ref().map(|n| n.kind);
    match kind {
        "deleted" | "disabled" => m.has(Flag::Deleted),
        "sub" | "subscription" => matches!(notice, Some(NoticeKind::Sub | NoticeKind::Gift)),
        "timeout" | "ban" => notice == Some(NoticeKind::Moderation),
        "highlighted" => m.meta.highlight.is_some() || m.has(Flag::Highlighted),
        "system" => {
            m.sender.is_none()
                && notice.is_some()
                && (notice != Some(NoticeKind::Moderation) || all.iter().any(|k| k == "timeout"))
        }
        "first-msg" => m.has(Flag::FirstMessage),
        "cheer-msg" => m.body.iter().any(|f| matches!(f, Fragment::Cheer { .. })),
        "redemption" => m.reward.is_some(),
        "reply" => m.reply.is_some(),
        "shared" => m.source.is_some(),
        "watch-streak" => notice == Some(NoticeKind::Milestone),
        "announcement" => notice == Some(NoticeKind::Announcement),
        "mention" => m.has(Flag::Mention),
        "similar" => m.has(Flag::Similar),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{Ctx, Incoming, build};
    use crate::irc::Line;

    fn msg(raw: &str) -> ChatMessage {
        let mut colors = Default::default();
        let mut ctx = Ctx::new(None, &mut colors, 0);
        match build(&Line::parse(raw).unwrap(), &mut ctx) {
            Some(Incoming::Message(m)) => m,
            _ => panic!(),
        }
    }

    #[test]
    fn operators() {
        let m = msg(
            "@badges=moderator/1,subscriber/3012;display-name=Bob;id=1;user-id=7 :bob!bob@bob PRIVMSG #forsen :look at https://x.com/a pls",
        );
        let q = |s: &str| Query::parse(s).matches(&m, "forsen");
        assert!(q("from:alice,BOB"));
        assert!(q("in:#forsen badge:mod subtier:3 has:link"));
        assert!(q("bob: look"), "names count as text");
        assert!(q(r#"regex:"at\s+https""#));
        assert!(!q("-from:bob"));
        assert!(q("!is:reply LOOK"));
        assert!(!q("is:deleted"));
        assert!(q("-nothing:here") == q("-nothing:here"));
        assert!(!q("missing"));
        assert!(Query::parse("  ").is_empty());
    }
}
