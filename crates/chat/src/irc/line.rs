//! One IRCv3 line, parsed without copying: `@tags :prefix COMMAND params :trailing`.
//! Tag values stay escaped until read with [`Line::tag`].

use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct Line<'a> {
    tags: &'a str,
    prefix: &'a str,
    pub command: &'a str,
    middle: &'a str,
    trailing: Option<&'a str>,
}

impl<'a> Line<'a> {
    /// `None` for an empty line or one without a command.
    pub fn parse(raw: &'a str) -> Option<Self> {
        let mut rest = raw.trim_end_matches(['\r', '\n']);
        let mut tags = "";
        if let Some(r) = rest.strip_prefix('@') {
            (tags, rest) = r.split_once(' ').unwrap_or((r, ""));
        }
        rest = rest.trim_start_matches(' ');
        let mut prefix = "";
        if let Some(r) = rest.strip_prefix(':') {
            (prefix, rest) = r.split_once(' ').unwrap_or((r, ""));
        }
        rest = rest.trim_start_matches(' ');
        let (command, rest) = rest.split_once(' ').unwrap_or((rest, ""));
        if command.is_empty() {
            return None;
        }
        let (middle, trailing) = match rest.find(" :") {
            Some(i) => (&rest[..i], Some(&rest[i + 2..])),
            None => match rest.strip_prefix(':') {
                Some(t) => ("", Some(t)),
                None => (rest, None),
            },
        };
        Some(Self {
            tags,
            prefix,
            command,
            middle,
            trailing,
        })
    }

    /// Unescaped tag value. `None` if absent; `Some("")` if present but empty.
    pub fn tag(&self, key: &str) -> Option<Cow<'a, str>> {
        self.raw_tag(key).map(unescape)
    }

    /// Tag value as sent, still escaped. Fine for ids, numbers and lists.
    pub fn raw_tag(&self, key: &str) -> Option<&'a str> {
        self.tags.split(';').find_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (k == key).then_some(v)
        })
    }

    /// Non-empty raw tag value.
    pub fn tag_nonempty(&self, key: &str) -> Option<&'a str> {
        self.raw_tag(key).filter(|v| !v.is_empty())
    }

    pub fn tag_num<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.raw_tag(key)?.parse().ok()
    }

    /// `1` = true; anything else, or missing, = false.
    pub fn tag_flag(&self, key: &str) -> bool {
        self.raw_tag(key) == Some("1")
    }

    /// All tags as (key, escaped value).
    pub fn tags(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.tags
            .split(';')
            .filter(|p| !p.is_empty())
            .map(|pair| pair.split_once('=').unwrap_or((pair, "")))
    }

    /// The nick in `nick!user@host`, or the whole prefix for a server.
    pub fn nick(&self) -> &'a str {
        self.prefix.split('!').next().unwrap_or("")
    }

    /// Params before the trailing one.
    pub fn middle(&self) -> impl Iterator<Item = &'a str> {
        self.middle.split(' ').filter(|p| !p.is_empty())
    }

    /// The text after ` :`, if any.
    pub fn trailing(&self) -> Option<&'a str> {
        self.trailing
    }

    /// First middle param without the `#`, e.g. the channel login.
    pub fn channel(&self) -> Option<&'a str> {
        self.middle()
            .next()
            .and_then(|c| c.strip_prefix('#'))
            .filter(|c| !c.is_empty())
    }
}

/// IRCv3 tag unescaping: `\s` space, `\:` `;`, `\\` `\`, `\r`, `\n`. A trailing lone
/// backslash is dropped; any other escaped char stands for itself.
pub fn unescape(value: &str) -> Cow<'_, str> {
    if !value.contains('\\') {
        return Cow::Borrowed(value);
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some(':') => out.push(';'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    Cow::Owned(out)
}

/// The reverse of [`unescape`], for tags we send.
pub fn escape(value: &str) -> Cow<'_, str> {
    if !value.contains([' ', ';', '\\', '\r', '\n']) {
        return Cow::Borrowed(value);
    }
    let mut out = String::with_capacity(value.len() + 8);
    for c in value.chars() {
        match c {
            ' ' => out.push_str("\\s"),
            ';' => out.push_str("\\:"),
            '\\' => out.push_str("\\\\"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privmsg_with_tags() {
        let l = Line::parse(
            "@badge-info=subscriber/8;color=#FF0000;display-name=Foo;emotes=25:0-4;id=abc;mod=0 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :Kappa hello :) there\r\n",
        )
        .unwrap();
        assert_eq!(l.command, "PRIVMSG");
        assert_eq!(l.nick(), "foo");
        assert_eq!(l.channel(), Some("bar"));
        assert_eq!(l.trailing(), Some("Kappa hello :) there"));
        assert_eq!(l.raw_tag("emotes"), Some("25:0-4"));
        assert_eq!(l.raw_tag("mod"), Some("0"));
        assert!(!l.tag_flag("mod"));
        assert_eq!(l.raw_tag("missing"), None);
        assert_eq!(l.tags().count(), 6);
    }

    #[test]
    fn server_lines() {
        let ping = Line::parse("PING :tmi.twitch.tv").unwrap();
        assert_eq!((ping.command, ping.trailing()), ("PING", Some("tmi.twitch.tv")));

        let welcome = Line::parse(":tmi.twitch.tv 001 justinfan1 :Welcome, GLHF!").unwrap();
        assert_eq!(welcome.command, "001");
        assert_eq!(welcome.middle().collect::<Vec<_>>(), ["justinfan1"]);
        assert_eq!(welcome.nick(), "tmi.twitch.tv");

        let cap = Line::parse(":tmi.twitch.tv CAP * ACK :twitch.tv/tags twitch.tv/commands").unwrap();
        assert_eq!(cap.middle().collect::<Vec<_>>(), ["*", "ACK"]);

        let join = Line::parse(":a!a@a.tmi.twitch.tv JOIN #chan").unwrap();
        assert_eq!((join.channel(), join.trailing()), (Some("chan"), None));

        let state = Line::parse("@emote-only=0;room-id=1;slow=10 :tmi.twitch.tv ROOMSTATE #chan").unwrap();
        assert_eq!(state.tag_num::<u32>("slow"), Some(10));
    }

    #[test]
    fn empty_and_odd_input() {
        assert!(Line::parse("").is_none());
        assert!(Line::parse("\r\n").is_none());
        assert!(Line::parse("@a=b").is_none());
        let l = Line::parse("@flag;x= :p CMD").unwrap();
        assert_eq!(l.raw_tag("flag"), Some(""));
        assert_eq!(l.raw_tag("x"), Some(""));
        assert_eq!(l.trailing(), None);
    }

    #[test]
    fn trailing_may_be_empty_or_contain_colons() {
        let l = Line::parse(":a PRIVMSG #c :").unwrap();
        assert_eq!(l.trailing(), Some(""));
        let l = Line::parse(":a PRIVMSG #c ::D :: x").unwrap();
        assert_eq!(l.trailing(), Some(":D :: x"));
    }

    #[test]
    fn tag_escapes_round_trip() {
        let l = Line::parse(r"@system-msg=5\sraiders\:\sfrom\\here\n :tmi.twitch.tv USERNOTICE #c").unwrap();
        assert_eq!(l.tag("system-msg").unwrap(), "5 raiders; from\\here\n");
        assert_eq!(unescape(r"end\"), "end");
        assert_eq!(unescape(r"\q"), "q");
        let text = "a b;c\\d";
        assert_eq!(unescape(&escape(text)), text);
        assert!(matches!(escape("plain"), Cow::Borrowed(_)));
    }
}
