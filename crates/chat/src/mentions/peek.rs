//! V104-02 The firehose's lines are raw IRC. Most never match, so only the channel and
//! the text are read, without parsing tags or allocating; a match is parsed in full.

/// A chat message's channel and text.
#[derive(Debug, PartialEq, Eq)]
pub struct Peek<'a> {
    /// Login, without `#`.
    pub channel: &'a str,
    /// `/me` lines without their wrapper.
    pub text: &'a str,
}

/// `@tags :nick!nick@host PRIVMSG #channel :text`. Anything else is `None`.
pub fn peek(line: &str) -> Option<Peek<'_>> {
    let line = line.trim_end_matches(['\r', '\n']);
    // Tag values never hold a space (it's `\s`), so the first one ends them.
    let rest = match line.strip_prefix('@') {
        Some(tagged) => tagged.split_once(' ')?.1,
        None => line,
    };
    let rest = match rest.strip_prefix(':') {
        Some(prefixed) => prefixed.split_once(' ')?.1,
        None => rest,
    };
    let (channel, text) = rest.strip_prefix("PRIVMSG #")?.split_once(" :")?;
    let text = match text.strip_prefix("\u{1}ACTION ") {
        Some(me) => me.strip_suffix('\u{1}').unwrap_or(me),
        None => text,
    };
    Some(Peek { channel, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_privmsg() {
        let line =
            "@badge-info=;display-name=A\\sB;id=x :ab!ab@ab.tmi.twitch.tv PRIVMSG #forsen :hi :) there\r\n";
        assert_eq!(
            peek(line),
            Some(Peek {
                channel: "forsen",
                text: "hi :) there"
            })
        );
        let me = ":ab!ab@ab.tmi.twitch.tv PRIVMSG #x :\u{1}ACTION waves\u{1}";
        assert_eq!(peek(me).map(|p| p.text), Some("waves"));
        assert_eq!(peek("@a=b :tmi.twitch.tv CLEARCHAT #x :someone"), None);
        assert_eq!(peek("garbage"), None);
    }
}
