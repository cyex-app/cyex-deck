//! Settings rules to ready matchers. A rule whose regex doesn't compile never matches.

use cyex_settings::schema::{Alert, IgnoreRule, PhraseRule, SentFrom, UserRule};
use regex::{NoExpand, Regex, RegexBuilder};

use crate::model::Client;

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

    /// 2.6 Plain highlight phrases match whole words.
    fn words(pattern: &str, case_sensitive: bool) -> Self {
        Self::build(
            &format!(r"(?:\b|\s|^){}(?:\b|\s|$)", regex::escape(pattern)),
            case_sensitive,
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
    pub matcher: Matcher,
    pub alert: Alert,
}

pub struct Ignore {
    pub matcher: Matcher,
    pub block: bool,
    pub replace: Replacement,
    pub route: String,
}

/// V105-12 Empty rules (one being filled in) match nothing.
pub fn phrases(rules: &[PhraseRule]) -> Vec<Rule> {
    rules
        .iter()
        .filter(|r| r.enabled && !r.is_blank())
        .map(|r| Rule {
            matcher: if r.regex {
                Matcher::build(&r.pattern, r.case_sensitive)
            } else {
                Matcher::words(&r.pattern, r.case_sensitive)
            },
            alert: r.alert.clone(),
        })
        .collect()
}

pub fn users(rules: &[UserRule]) -> Vec<Rule> {
    rules
        .iter()
        .filter(|r| r.enabled && !r.is_blank())
        .map(|r| Rule {
            matcher: if r.regex {
                Matcher::build(&r.login, false)
            } else {
                Matcher::build(&format!("^{}$", regex::escape(&r.login)), false)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matchers() {
        let w = Matcher::words("pog", false);
        assert!(w.is_match("so POG man") && w.is_match("pog") && !w.is_match("pogchamp"));
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
