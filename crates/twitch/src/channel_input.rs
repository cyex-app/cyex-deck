//! 1.14 What someone typed to name a channel: a login, a user id, `@name`, `#name` or a
//! twitch.tv link.

/// A channel to look up. Digits alone can be a login or a user id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelQuery {
    Login(String),
    Id(String),
    Either(String),
}

/// First path parts on twitch.tv that aren't channels.
const NOT_CHANNELS: &[&str] = &[
    "directory",
    "videos",
    "settings",
    "subscriptions",
    "wallet",
    "inventory",
    "drops",
    "search",
    "downloads",
    "p",
    "jobs",
    "turbo",
];

/// Paths where the channel is the second part, e.g. `/popout/<login>/chat`.
const WRAPPERS: &[&str] = &["popout", "moderator", "embed", "u"];

pub fn parse(input: &str) -> Option<ChannelQuery> {
    let input = input.trim();
    if let Some(id) = input.strip_prefix("id:") {
        let id = id.trim();
        return is_id(id).then(|| ChannelQuery::Id(id.to_owned()));
    }
    let name = match link_path(input) {
        Some(path) => from_path(path)?,
        None => input.trim_start_matches(['@', '#']),
    };
    let name = name.to_ascii_lowercase();
    if !is_login(&name) {
        return None;
    }
    Some(if is_id(&name) {
        ChannelQuery::Either(name)
    } else {
        ChannelQuery::Login(name)
    })
}

/// The path of a twitch.tv link, without the leading slash.
fn link_path(input: &str) -> Option<&str> {
    let rest = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .unwrap_or(input);
    let (host, path) = rest.split_once('/')?;
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").or_else(|| host.strip_prefix("m.")).unwrap_or(&host);
    matches!(host, "twitch.tv" | "dashboard.twitch.tv").then_some(path)
}

fn from_path(path: &str) -> Option<&str> {
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let mut parts = path.split('/').filter(|p| !p.is_empty());
    let first = parts.next()?;
    let lower = first.to_ascii_lowercase();
    if WRAPPERS.contains(&lower.as_str()) {
        return parts.next();
    }
    (!NOT_CHANNELS.contains(&lower.as_str())).then_some(first)
}

fn is_login(s: &str) -> bool {
    (1..=25).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn is_id(s: &str) -> bool {
    (1..=20).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::ChannelQuery::*;
    use super::*;

    fn login(s: &str) -> Option<ChannelQuery> {
        Some(Login(s.into()))
    }

    #[test]
    fn names_and_mentions() {
        assert_eq!(parse("xQc"), login("xqc"));
        assert_eq!(parse("  @Pokimane "), login("pokimane"));
        assert_eq!(parse("#lirik"), login("lirik"));
        assert_eq!(parse("a_b_1"), login("a_b_1"));
    }

    #[test]
    fn digits_can_be_either() {
        assert_eq!(parse("71092938"), Some(Either("71092938".into())));
        assert_eq!(parse("id:71092938"), Some(Id("71092938".into())));
        assert_eq!(parse("id:abc"), None);
    }

    #[test]
    fn twitch_links() {
        assert_eq!(parse("https://www.twitch.tv/xqc"), login("xqc"));
        assert_eq!(parse("twitch.tv/xqc/videos?filter=all"), login("xqc"));
        assert_eq!(parse("https://m.twitch.tv/Shroud"), login("shroud"));
        assert_eq!(parse("https://www.twitch.tv/popout/xqc/chat?popout="), login("xqc"));
        assert_eq!(parse("https://www.twitch.tv/moderator/summit1g"), login("summit1g"));
        assert_eq!(parse("https://dashboard.twitch.tv/u/lirik/stream-manager"), login("lirik"));
        assert_eq!(parse("https://www.twitch.tv/directory/category/just-chatting"), None);
        assert_eq!(parse("https://youtube.com/xqc"), None);
    }

    #[test]
    fn junk_is_rejected() {
        for bad in ["", "  ", "two words", "this_name_is_much_too_long_for_twitch", "émile", "a-b"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }
}
