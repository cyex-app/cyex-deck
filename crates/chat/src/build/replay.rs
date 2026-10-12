//! P8 A VOD chat comment as a chat line, built like a live one: the same tokenizer and
//! the same readable name colors.

use super::Ctx;
use super::sender::default_color;
use crate::model::{Badge, ChatMessage, Flag, Meta, Sender};
use crate::tokenize::{Input, tokenize};

pub struct Comment<'a> {
    pub id: &'a str,
    /// Unix ms it was sent at.
    pub ts: i64,
    pub user_id: &'a str,
    pub login: &'a str,
    pub display: &'a str,
    pub color: Option<&'a str>,
    /// `(set, version)`.
    pub badges: &'a [(String, String)],
    /// Text pieces, each with the Twitch emote id it shows, if any.
    pub pieces: &'a [(String, Option<String>)],
}

pub fn replay(c: &Comment<'_>, ctx: &mut Ctx<'_>) -> ChatMessage {
    // Rebuild the IRC emotes tag (code point ranges) and let the tokenizer do the rest.
    let mut text = String::new();
    let mut tag = Vec::new();
    let mut at = 0;
    for (piece, emote) in c.pieces {
        let len = piece.chars().count();
        if let Some(id) = emote.as_deref().filter(|_| len > 0) {
            tag.push(format!("{id}:{at}-{}", at + len - 1));
        }
        text.push_str(piece);
        at += len;
    }
    let tag = tag.join("/");
    let me: &[String] = ctx.me.map_or(&[], |m| m.names.as_slice());
    let input = Input {
        text: text.trim_end(),
        sender: c.user_id,
        emotes: &tag,
        gifs: "",
        bits: 0,
    };
    let tokens = tokenize(&input, me, ctx.emotes);
    let raw = c.color.unwrap_or_else(|| default_color(c.user_id, c.login));
    let mut msg = ChatMessage {
        id: c.id.to_owned(),
        ts: c.ts,
        sender: Some(Sender {
            id: c.user_id.to_owned(),
            login: c.login.to_owned(),
            display: c.display.to_owned(),
            color: ctx.colors.get(raw),
            badges: c
                .badges
                .iter()
                .map(|(set, version)| Badge {
                    set: set.clone(),
                    version: version.clone(),
                    info: None,
                })
                .collect(),
        }),
        body: tokens.body,
        notice: None,
        reply: None,
        reward: None,
        source: None,
        flags: Vec::new(),
        meta: Meta::default(),
    };
    if tokens.mentions_me {
        msg.set(Flag::Mention);
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::Colors;
    use crate::model::{Fragment, Provider};

    #[test]
    fn comments_become_chat_lines() {
        let pieces = [
            ("héllo ".to_owned(), None),
            ("Kappa".to_owned(), Some("25".to_owned())),
            (" see twitch.tv/x".to_owned(), None),
        ];
        let badges = [("subscriber".to_owned(), "12".to_owned())];
        let c = Comment {
            id: "c1",
            ts: 5,
            user_id: "7",
            login: "bob",
            display: "Bob",
            color: None,
            badges: &badges,
            pieces: &pieces,
        };
        let mut colors = Colors::default();
        let msg = replay(&c, &mut Ctx::new(None, &mut colors, 0));
        assert_eq!(
            msg.body[1],
            Fragment::emote(Provider::Twitch, "25", "Kappa")
        );
        assert!(matches!(&msg.body[3], Fragment::Link { url, .. } if url == "https://twitch.tv/x"));
        let s = msg.sender.unwrap();
        assert_eq!((s.display.as_str(), s.badges[0].version.as_str()), ("Bob", "12"));
        assert!(s.color.dark.starts_with('#'));
    }
}
