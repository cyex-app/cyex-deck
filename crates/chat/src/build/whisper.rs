//! 1.26 A whisper as a chat line, so whispers draw like chat: Twitch emotes from their
//! ranges, provider emotes by name, readable name colors.

use std::collections::BTreeMap;

use crate::model::{ChatMessage, Flag, Meta, Sender};
use crate::tokenize::{Emotes, Input, tokenize};

use super::Colors;
use super::sender::default_color;

/// Who sent it.
pub struct From<'a> {
    pub id: &'a str,
    pub login: &'a str,
    pub display: &'a str,
    /// `#rrggbb`, or empty when never set.
    pub color: &'a str,
}

pub struct Whisper<'a> {
    pub id: &'a str,
    pub ts: i64,
    pub from: From<'a>,
    pub text: &'a str,
    /// Twitch emotes: id and code point range, both ends inclusive.
    pub emotes: &'a [(String, usize, usize)],
    /// Sent by the reading account.
    pub own: bool,
    pub deleted: bool,
}

/// `me`: the reading account's lowercase names, for mentions.
pub fn message(w: &Whisper<'_>, me: &[String], colors: &mut Colors, emotes: &dyn Emotes) -> ChatMessage {
    let tag = emote_tag(w.emotes);
    let tokens = tokenize(
        &Input {
            text: w.text,
            sender: w.from.id,
            emotes: &tag,
            gifs: "",
            bits: 0,
        },
        if w.own { &[] } else { me },
        emotes,
    );
    let raw = if w.from.color.is_empty() {
        default_color(w.from.id, w.from.login)
    } else {
        w.from.color
    };
    let display = if w.from.display.trim().is_empty() {
        w.from.login
    } else {
        w.from.display.trim()
    };
    let mut flags = vec![Flag::Whisper];
    if w.own {
        flags.push(Flag::Own);
    }
    if w.deleted {
        flags.push(Flag::Deleted);
    }
    ChatMessage {
        id: w.id.to_owned(),
        ts: w.ts,
        sender: Some(Sender {
            id: w.from.id.to_owned(),
            login: w.from.login.to_owned(),
            display: display.to_owned(),
            color: colors.get(raw),
            badges: Vec::new(),
        }),
        body: tokens.body,
        notice: None,
        reply: None,
        reward: None,
        source: None,
        flags,
        meta: Meta::default(),
    }
}

/// Ranges in IRC's `emotes` tag form: `25:0-4,6-10/354:12-16`.
fn emote_tag(ranges: &[(String, usize, usize)]) -> String {
    let mut by_id: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (id, from, to) in ranges {
        if from <= to {
            by_id.entry(id).or_default().push(format!("{from}-{to}"));
        }
    }
    by_id
        .into_iter()
        .map(|(id, r)| format!("{id}:{}", r.join(",")))
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Fragment;
    use crate::tokenize::NoEmotes;

    #[test]
    fn whispers_become_lines() {
        let ranges = [("25".to_owned(), 3, 7)];
        let w = Whisper {
            id: "u1",
            ts: 5,
            from: From {
                id: "2",
                login: "bob",
                display: "",
                color: "",
            },
            text: "hi Kappa @me",
            emotes: &ranges,
            own: false,
            deleted: false,
        };
        let m = message(&w, &["me".into()], &mut Colors::default(), &NoEmotes);
        assert_eq!(m.sender.as_ref().unwrap().display, "bob");
        assert!(m.flags.contains(&Flag::Whisper) && !m.flags.contains(&Flag::Own));
        assert!(m.body.iter().any(|f| matches!(f, Fragment::Emote { .. })));
    }

    #[test]
    fn tags() {
        let r = [
            ("25".to_owned(), 0, 4),
            ("354".to_owned(), 12, 16),
            ("25".to_owned(), 6, 10),
        ];
        assert_eq!(emote_tag(&r), "25:0-4,6-10/354:12-16");
        assert_eq!(emote_tag(&[]), "");
    }
}
