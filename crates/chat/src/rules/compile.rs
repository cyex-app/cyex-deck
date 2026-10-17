//! Settings rules to ready matchers. A rule whose regex doesn't compile never matches.

use cyex_settings::schema::{Alert, IgnoreRule, SentFrom, UserRule};
use fancy_regex::RegexBuilder as FancyBuilder;
use regex::{NoExpand, Regex, RegexBuilder};

use crate::model::Client;

/// V105-16 Highlight patterns for people (and `import_regex`): Rust syntax with lookaround
/// and backreferences (fancy-regex, like mentions), so converted Chatterino patterns run.
/// Same limits as mentions. Words are a `MentionSet` (V106-33). Ignores keep `Matcher`.
pub struct HighlightMatcher(Option<fancy_regex::Regex>);

impl HighlightMatcher {
    /// `pub(crate)` for `pcre`'s tests.
    pub(crate) fn build(pattern: &str, case_sensitive: bool) -> Result<Self, String> {
        FancyBuilder::new(pattern)
            .case_insensitive(!case_sensitive)
            .backtrack_limit(crate::mentions::BACKTRACK_LIMIT)
            .delegate_size_limit(crate::mentions::SIZE_LIMIT)
            .delegate_dfa_size_limit(crate::mentions::SIZE_LIMIT)
            .build()
            .map(|re| Self(Some(re)))
            .map_err(|e| crate::mentions::short_error(&e.to_string()))
    }

    /// A rule whose regex doesn't compile never matches.
    fn or_never(pattern: &str, case_sensitive: bool) -> Self {
        Self::build(pattern, case_sensitive).unwrap_or(Self(None))
    }

    /// Over the backtrack limit counts as no match.
    pub fn is_match(&self, text: &str) -> bool {
        self.0.as_ref().is_some_and(|re| re.is_match(text).unwrap_or(false))
    }
}

/// V105-16 Why a highlight pattern can't run, or `None`.
pub fn highlight_error(pattern: &str) -> Option<String> {
    HighlightMatcher::build(pattern, false).err()
}

pub struct Matcher(Option<Regex>);

impl Matcher {
    fn build(pattern: &str, case_sensitive: bool) -> Self {
        Self(
            RegexBuilder::new(pattern)
                .case_insensitive(!case_sensitive)
                .build()
                .ok(),
        )
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.0.as_ref().is_some_and(|re| re.is_match(text))
    }

    /// `None` when nothing matched. `\1` in `with` is the first group (regex rules).
    pub fn replace(&self, text: &str, with: &Replacement) -> Option<String> {
        let re = self.0.as_ref()?;
        if !re.is_match(text) {
            return None;
        }
        Some(match with {
            Replacement::Plain(s) => re.replace_all(text, NoExpand(s)).into_owned(),
            Replacement::Groups(s) => re.replace_all(text, s.as_str()).into_owned(),
        })
    }
}

pub enum Replacement {
    Plain(String),
    /// Already in the regex crate's syntax.
    Groups(String),
}

pub struct Rule {
    pub matcher: HighlightMatcher,
    pub alert: Alert,
}

pub struct Ignore {
    pub matcher: Matcher,
    pub block: bool,
    pub replace: Replacement,
    pub route: String,
}

/// V105-12 Empty rules (one being filled in) match nothing.
pub fn users(rules: &[UserRule]) -> Vec<Rule> {
    rules
        .iter()
        .filter(|r| r.enabled && !r.is_blank())
        .map(|r| Rule {
            matcher: if r.regex {
                HighlightMatcher::or_never(&r.login, false)
            } else {
                HighlightMatcher::or_never(&format!("^{}$", regex::escape(&r.login)), false)
            },
            alert: r.alert.clone(),
        })
        .collect()
}

pub fn ignores(rules: &[IgnoreRule]) -> Vec<Ignore> {
    rules
        .iter()
        .filter(|r| r.enabled && !r.is_blank())
        .map(|r| Ignore {
            matcher: if r.regex {
                Matcher::build(&r.pattern, r.case_sensitive)
            } else {
                Matcher::build(&regex::escape(&r.pattern), r.case_sensitive)
            },
            block: r.block,
            replace: if r.regex {
                Replacement::Groups(groups(&r.replace))
            } else {
                Replacement::Plain(r.replace.clone())
            },
            route: r.route.clone(),
        })
        .collect()
}

/// Chatterino's `\1` to the regex crate's `${1}`; a lone `$` stays a `$`.
fn groups(with: &str) -> String {
    let mut out = String::new();
    let mut chars = with.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek().is_some_and(char::is_ascii_digit) => {
                let mut n = String::new();
                while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                    n.push(*d);
                    chars.next();
                }
                out.push_str(&format!("${{{n}}}"));
            }
            '$' => out.push_str("$$"),
            c => out.push(c),
        }
    }
    out
}

pub fn client(app: SentFrom) -> Client {
    match app {
        SentFrom::Web => Client::Web,
        SentFrom::Android => Client::Android,
        SentFrom::Ios => Client::Ios,
        SentFrom::Mobile => Client::Mobile,
        SentFrom::Odd => Client::Odd,
    }
}

/// V105-16 What a badge rule asks for: any version, one exact version, or a subscriber tier.
pub enum BadgeMatch {
    Any,
    Exact(String),
    /// `subscriber1` to `subscriber3`: a subscriber badge version is the months, plus 1000 per tier above the first.
    Tier(u8),
}

/// `moderator`, `subscriber/3012`, `subscriber2` -> the badge set and what it asks for.
pub fn badge(rule: &str) -> (String, BadgeMatch) {
    let rule = rule.trim();
    if let Some((set, v)) = rule.split_once('/') {
        return (set.to_owned(), BadgeMatch::Exact(v.to_owned()));
    }
    match rule.strip_prefix("subscriber").map(str::as_bytes) {
        Some([t @ b'1'..=b'3']) => ("subscriber".to_owned(), BadgeMatch::Tier(t - b'0')),
        _ => (rule.to_owned(), BadgeMatch::Any),
    }
}

impl BadgeMatch {
    pub fn matches(&self, version: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(v) => v == version,
            Self::Tier(t) => version
                .parse::<u32>()
                .is_ok_and(|n| (n / 1000).clamp(1, 3) == u32::from(*t)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // V105-16
    #[test]
    fn badge_tiers() {
        let (set, m) = badge("subscriber2");
        assert_eq!(set, "subscriber");
        assert!(m.matches("2000") && m.matches("2012") && !m.matches("12") && !m.matches("3012"));
        let (_, t1) = badge("subscriber1");
        assert!(t1.matches("0") && t1.matches("12") && t1.matches("1999") && !t1.matches("2000"));
        let (_, t3) = badge("subscriber3");
        assert!(t3.matches("3000") && t3.matches("3024") && !t3.matches("2999"));
        let (set, any) = badge("subscriber");
        assert!(set == "subscriber" && any.matches("3012") && any.matches("1"));
        let (set, exact) = badge("subscriber/12");
        assert!(set == "subscriber" && exact.matches("12") && !exact.matches("2012"));
        assert!(
            matches!(badge("subscriber4").1, BadgeMatch::Any),
            "not a tier name"
        );
        assert!(badge("vip").1.matches("1"));
    }

    // V105-16
    #[test]
    fn highlights_run_on_fancy_regex() {
        let look = HighlightMatcher::or_never(r"(?<!@)cyex", false);
        assert!(look.is_match("hey cyex") && !look.is_match("@cyex"));
        let back = HighlightMatcher::or_never(r"(\w)\1", false);
        assert!(back.is_match("aa") && !back.is_match("ab"));
        let slow = HighlightMatcher::or_never(r"^(a+)+\1b$", false);
        assert!(!slow.is_match(&"a".repeat(30)));
        assert!(highlight_error("(abc").is_some());
        assert!(highlight_error(r"\d+").is_none());
        let u = &users(&[UserRule {
            login: "^bot_".into(),
            regex: true,
            ..UserRule::default()
        }])[0];
        assert!(u.matcher.is_match("bot_x"));
    }

    #[test]
    fn matchers() {
        assert!(
            !Matcher::build("(", false).is_match("("),
            "bad regex never matches"
        );
        let ig = &ignores(&[IgnoreRule {
            pattern: r"(\w+)@spam\.com".into(),
            regex: true,
            replace: r"[\1 hidden $5]".into(),
            ..IgnoreRule::default()
        }])[0];
        assert_eq!(
            ig.matcher.replace("mail bob@spam.com", &ig.replace).as_deref(),
            Some("mail [bob hidden $5]")
        );
        let plain = &ignores(&[IgnoreRule {
            pattern: "darn".into(),
            ..IgnoreRule::default()
        }])[0];
        assert_eq!(
            plain.matcher.replace("darn it", &plain.replace).as_deref(),
            Some("*** it")
        );
        assert_eq!(
            plain.matcher.replace("Darn it", &plain.replace).as_deref(),
            Some("*** it")
        );
    }
}
