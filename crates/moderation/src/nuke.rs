//! M1 `/nuke`: find everyone who sent a matching message in the last N seconds, show a
//! preview, then time out or ban them. The app feeds in the room's recent messages; this
//! decides who is hit. Matching logic only, no I/O.
//!
//! `/nuke <phrase> [window] [length]` — `phrase` is plain text, or `/.../` for a regex.
//! `window` (e.g. `60s`, default 60s) is how far back to look, `length` (e.g. `10m`,
//! default: ban) how long to time out; `ban` or `0` bans.

use cyex_twitch::helix::moderation::MAX_TIMEOUT_SECS;
use regex::RegexBuilder;

use crate::duration;

/// How a `/nuke` punishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punish {
    Ban,
    Timeout(u32),
}

/// A parsed `/nuke`.
#[derive(Debug)]
pub struct Nuke {
    matcher: Matcher,
    /// Seconds to look back.
    pub window_secs: u64,
    pub punish: Punish,
}

#[derive(Debug)]
enum Matcher {
    Phrase(String),
    Regex(regex::Regex),
}

/// One message to test.
pub struct Line<'a> {
    pub user_id: &'a str,
    pub login: &'a str,
    pub display: &'a str,
    pub text: &'a str,
    pub ts_ms: i64,
}

/// A user the nuke would hit, and one message of theirs that matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub user_id: String,
    pub login: String,
    pub display: String,
    pub sample: String,
}

const DEFAULT_WINDOW: u64 = 60;
/// Keeps a preview and a round of actions sane.
pub const MAX_HITS: usize = 200;

impl Nuke {
    /// `args` is everything after `/nuke`. The phrase may be quoted or `/regex/`.
    pub fn parse(args: &str) -> Result<Self, String> {
        let args = args.trim();
        if args.is_empty() {
            return Err("Usage: /nuke phrase [60s] [10m|ban]".into());
        }
        let (phrase, rest) = take_phrase(args);
        let matcher = if let Some(src) = phrase.strip_prefix('/').and_then(|s| s.strip_suffix('/')) {
            Matcher::Regex(
                RegexBuilder::new(src)
                    .case_insensitive(true)
                    .size_limit(1 << 20)
                    .build()
                    .map_err(|_| "That regex doesn't read.".to_owned())?,
            )
        } else if phrase.is_empty() {
            return Err("Usage: /nuke phrase [60s] [10m|ban]".into());
        } else {
            Matcher::Phrase(phrase.to_ascii_lowercase())
        };
        let mut window = DEFAULT_WINDOW;
        let mut punish = Punish::Ban;
        // The first duration is the window to look back, the second a timeout length.
        let mut durations = 0;
        for word in rest.split_whitespace() {
            if matches!(word.to_ascii_lowercase().as_str(), "ban" | "0") {
                punish = Punish::Ban;
            } else if let Some(secs) = duration::parse(word, 1) {
                match durations {
                    0 => window = secs.max(1),
                    _ => punish = Punish::Timeout(secs.min(u64::from(MAX_TIMEOUT_SECS)).max(1) as u32),
                }
                durations += 1;
            } else {
                return Err(format!("Didn't understand “{word}”. Usage: /nuke phrase [60s] [10m|ban]"));
            }
        }
        Ok(Self {
            matcher,
            window_secs: window,
            punish,
        })
    }

    fn hit(&self, text: &str) -> bool {
        match &self.matcher {
            Matcher::Phrase(p) => text.to_ascii_lowercase().contains(p),
            Matcher::Regex(r) => r.is_match(text),
        }
    }

    /// Everyone matched in the window, newest message kept per user, most recent first.
    pub fn scan(&self, lines: &[Line<'_>], now_ms: i64) -> Vec<Hit> {
        let cutoff = now_ms - self.window_secs as i64 * 1000;
        let mut hits: Vec<Hit> = Vec::new();
        for l in lines.iter().rev() {
            if l.ts_ms < cutoff {
                continue;
            }
            if l.user_id.is_empty() || hits.iter().any(|h| h.user_id == l.user_id) || !self.hit(l.text) {
                continue;
            }
            hits.push(Hit {
                user_id: l.user_id.to_owned(),
                login: l.login.to_owned(),
                display: if l.display.is_empty() { l.login } else { l.display }.to_owned(),
                sample: l.text.chars().take(120).collect(),
            });
            if hits.len() >= MAX_HITS {
                break;
            }
        }
        hits
    }
}

/// Splits a leading quoted or `/regex/` phrase from the rest; else the first word.
fn take_phrase(args: &str) -> (&str, &str) {
    let bytes = args.as_bytes();
    let close = match bytes[0] {
        b'"' => Some(b'"'),
        b'/' => Some(b'/'),
        _ => None,
    };
    if let Some(close) = close
        && let Some(end) = args[1..].find(close as char)
    {
        let phrase = &args[if close == b'"' { 1 } else { 0 }..if close == b'"' { end + 1 } else { end + 2 }];
        return (phrase, args[end + 2..].trim());
    }
    match args.split_once(char::is_whitespace) {
        Some((a, b)) => (a, b.trim()),
        None => (args, ""),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn line<'a>(user: &'a str, text: &'a str, ts: i64) -> Line<'a> {
        Line {
            user_id: user,
            login: user,
            display: user,
            text,
            ts_ms: ts,
        }
    }

    #[test]
    fn parses_window_and_punishment() {
        let n = Nuke::parse("spam").unwrap();
        assert_eq!((n.window_secs, n.punish), (60, Punish::Ban));
        let n = Nuke::parse("spam 30s 10m").unwrap();
        assert_eq!((n.window_secs, n.punish), (30, Punish::Timeout(600)));
        let n = Nuke::parse("\"buy followers\" 2m ban").unwrap();
        assert_eq!((n.window_secs, n.punish), (120, Punish::Ban));
        assert!(Nuke::parse("/(free|cheap) viewers/ 45s 1h").is_ok());
        assert!(Nuke::parse("").is_err());
        assert!(Nuke::parse("/[/ 30s").is_err());
    }

    #[test]
    fn scans_the_window_once_per_user() {
        let now = 100_000;
        let lines = [
            line("1", "buy my FOLLOWERS now", now - 5_000),
            line("1", "buy followers cheap", now - 4_000),
            line("2", "hello", now - 3_000),
            line("3", "FOLLOWERS for sale", now - 90_000), // too old
        ];
        let n = Nuke::parse("followers 60s").unwrap();
        let hits = n.scan(&lines, now);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].user_id, "1");
        // Newest matching message is the sample.
        assert_eq!(hits[0].sample, "buy followers cheap");
    }

    #[test]
    fn regex_matches() {
        let now = 10_000;
        let lines = [line("1", "get CHEAP viewers", now - 1_000), line("2", "hi", now)];
        let n = Nuke::parse("/(free|cheap) viewers/").unwrap();
        assert_eq!(n.scan(&lines, now).len(), 1);
    }
}
