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

    /// V107-Mq Frames as the firehose sends them on 2026/10/22: one IRC line per text
    /// frame, no line ending, `tmi-sent-ts` first, `source-*` tags. Synthetic text.
    const FRAMES: [&str; 3] = [
        "@tmi-sent-ts=1792694804267;subscriber=1;id=4759c679-0000-415c-bd2d-a915785f2968;room-id=1;user-id=2;client-nonce=72e9f45c;display-name=Ann_B;badges=founder/0,premium/1;badge-info=founder/2;color=#B22222;flags=;user-type=;emotes= :ann_b!ann_b@ann_b.tmi.twitch.tv PRIVMSG #somechan :hey @Seryxx look",
        "@tmi-sent-ts=1792694804217;id=07d72428-0000-4750-85be-eba61048cd83;room-id=3;user-id=4;display-name=bob;badges=;badge-info=;flags=;user-type=;emotes=;source-only=0;source-room-id=3;source-id=07d72428-0000-4750-85be-eba61048cd83 :bob!bob@bob.tmi.twitch.tv PRIVMSG #other :\u{1}ACTION waves at seryxx\u{1}",
        "@tmi-sent-ts=1792694804300;room-id=3;target-user-id=4 :tmi.twitch.tv CLEARCHAT #other :bob",
    ];

    #[test]
    fn current_firehose_frames_match_and_build() {
        use crate::build::{Ctx, Incoming, build};
        use crate::irc::Line;
        use crate::mentions::MentionSet;
        use cyex_settings::schema::PhraseRule;

        let set = MentionSet::new(&[PhraseRule {
            pattern: "seryxx".into(),
            ..PhraseRule::default()
        }]);
        let mut scratch = String::new();
        let mut hits = Vec::new();
        for raw in FRAMES {
            let Some(p) = peek(raw) else { continue };
            if set.first(p.text, &mut scratch).is_none() {
                continue;
            }
            let mut colors = Default::default();
            let mut ctx = Ctx::new(None, &mut colors, 0);
            let Some(Incoming::Message(m)) = build(&Line::parse(raw).unwrap(), &mut ctx) else {
                panic!("not a message: {raw}")
            };
            let s = m.sender.unwrap();
            hits.push((p.channel.to_owned(), m.id, s.login, s.id));
        }
        assert_eq!(hits.len(), 2);
        let first = ("somechan".to_owned(), "4759c679-0000-415c-bd2d-a915785f2968".to_owned(), "ann_b".to_owned(), "2".to_owned());
        assert_eq!(hits[0], first);
        assert_eq!((hits[1].0.as_str(), hits[1].2.as_str()), ("other", "bob"));
    }
}
