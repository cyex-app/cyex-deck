//! Link detection (X2). A word is a link if it has an http(s) scheme, or looks like
//! `host.tld/...` with a known TLD. The host is shown as Twitch resolves it: IDNs come
//! out as punycode, so look-alike domains stand out.

use url::Url;

use crate::model::LinkTarget;

/// Common TLDs for links typed without a scheme. Sorted for binary search.
const TLDS: &[&str] = &[
    "ai", "am", "app", "ar", "art", "at", "au", "be", "bio", "biz", "blog", "br", "ca", "cc", "ch",
    "chat", "cl", "click", "cloud", "club", "cn", "co", "com", "cz", "de", "dev", "dk", "edu", "es",
    "eu", "fi", "fm", "fr", "fun", "game", "games", "gay", "gg", "gl", "gov", "gr", "hk", "hu", "id",
    "ie", "il", "in", "info", "io", "is", "it", "jp", "kr", "la", "link", "live", "lol", "ly", "me",
    "media", "moe", "mx", "my", "net", "news", "nl", "no", "nu", "nz", "online", "org", "page", "ph",
    "pl", "pro", "pt", "pw", "ro", "rs", "ru", "se", "sg", "sh", "shop", "site", "sk", "so", "social",
    "store", "stream", "su", "tech", "th", "tk", "to", "top", "tr", "tv", "tw", "ua", "uk", "us",
    "video", "vn", "wiki", "ws", "xyz", "za", "zone",
];

#[derive(Debug, PartialEq, Eq)]
pub struct Link<'a> {
    /// Text before the link, e.g. an opening bracket.
    pub lead: &'a str,
    pub text: &'a str,
    /// Trailing punctuation that isn't part of the link.
    pub tail: &'a str,
    pub url: String,
    pub host: String,
    /// Twitch clips and VODs open in the app.
    pub target: Option<LinkTarget>,
}

pub fn detect(word: &str) -> Option<Link<'_>> {
    let start = word.find(|c: char| !matches!(c, '(' | '<' | '"' | '\'' | '[')).unwrap_or(word.len());
    let (lead, rest) = word.split_at(start);
    let text = trim_tail(rest);
    if text.len() < 4 {
        return None;
    }
    let tail = &rest[text.len()..];

    let lower = text.get(..8).unwrap_or(text).to_ascii_lowercase();
    let url = if lower.starts_with("https://") || lower.starts_with("http://") {
        Url::parse(text).ok()?
    } else {
        if text.contains('@') || !text.contains('.') {
            return None;
        }
        let url = Url::parse(&format!("https://{text}")).ok()?;
        if !known_tld(url.host_str()?) {
            return None;
        }
        url
    };
    let host = url.host_str()?.trim_start_matches("www.").to_owned();
    if !host.contains('.') && url.host_str() != Some("localhost") {
        return None;
    }
    Some(Link {
        lead,
        text,
        tail,
        target: super::targets::target(&url),
        url: url.into(),
        host,
    })
}

fn known_tld(host: &str) -> bool {
    let mut labels = host.rsplit('.');
    let tld = labels.next().unwrap_or("");
    labels.next().is_some_and(|l| !l.is_empty()) && TLDS.binary_search(&tld).is_ok()
}

/// Drops sentence punctuation after a link, and closing brackets without an opening one.
fn trim_tail(s: &str) -> &str {
    let unbalanced = |s: &str, open: char, close: char| s.matches(close).count() > s.matches(open).count();
    let mut s = s;
    loop {
        let before = s.len();
        s = s.trim_end_matches(['.', ',', '!', '?', ';', ':', '"', '\'', '>']);
        if s.ends_with(')') && unbalanced(s, '(', ')') || s.ends_with(']') && unbalanced(s, '[', ']') {
            s = &s[..s.len() - 1];
        }
        if s.len() == before {
            return s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(word: &str) -> Option<String> {
        detect(word).map(|l| l.host)
    }

    #[test]
    fn schemes_and_bare_domains() {
        assert_eq!(host("https://www.twitch.tv/foo").as_deref(), Some("twitch.tv"));
        assert_eq!(host("HTTP://Example.com").as_deref(), Some("example.com"));
        assert_eq!(host("youtube.com/watch?v=x").as_deref(), Some("youtube.com"));
        let l = detect("clips.twitch.tv/abc").unwrap();
        assert_eq!(l.url, "https://clips.twitch.tv/abc");
    }

    #[test]
    fn not_links() {
        for w in ["file.txt", "e.g.", "a@b.com", "hello", "...", "1.5", "https://", ".com"] {
            assert_eq!(detect(w), None, "{w}");
        }
    }

    #[test]
    fn punctuation_around_links() {
        let l = detect("https://a.com/x_(y)).").unwrap();
        assert_eq!((l.text, l.tail), ("https://a.com/x_(y)", ")."));
        let l = detect("(twitch.tv)").unwrap();
        assert_eq!((l.lead, l.text, l.tail), ("(", "twitch.tv", ")"));
        let l = detect("x.com.").unwrap();
        assert_eq!((l.text, l.tail), ("x.com", "."));
    }

    #[test]
    fn twitch_links_carry_what_they_open() {
        let target = |w: &str| detect(w).and_then(|l| l.target);
        assert!(matches!(target("clips.twitch.tv/Funny-AbC"), Some(LinkTarget::Clip { slug }) if slug == "Funny-AbC"));
        assert!(matches!(target("twitch.tv/videos/12"), Some(LinkTarget::Video { id, at: 0 }) if id == "12"));
        assert_eq!(target("twitch.tv/xqc"), None);
    }

    #[test]
    fn idn_hosts_show_as_punycode() {
        let l = detect("https://twіtch.tv/login").unwrap(); // Cyrillic і
        assert!(l.host.starts_with("xn--"), "{}", l.host);
    }

    #[test]
    fn tld_list_is_sorted() {
        assert!(TLDS.windows(2).all(|w| w[0] < w[1]));
    }
}
