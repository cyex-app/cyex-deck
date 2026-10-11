//! S5 Plugins see a line before the user's rules do and may hide it, change its text or
//! highlight it. The app implements [`LineHook`]; rooms only apply what comes back. S7
//! Plugins can change lines that are in already, and swap their own for new ones.

use std::collections::HashMap;
use std::sync::Arc;

use super::Room;
use crate::model::{ChatMessage, Flag, Fragment, Highlight, HighlightKind, Provider, RoomEvent};
use crate::tokenize::{Emotes, Found, Input, tokenize};

/// What a hook wants done to a line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineChange {
    pub hide: bool,
    pub text: Option<String>,
    /// `#rrggbb(aa)`, or empty for the usual highlight color.
    pub highlight: Option<String>,
}

pub trait LineHook: Send + Sync {
    /// Called with the room's channel login. Must answer quickly: the room waits.
    fn check(&self, channel: &str, m: &ChatMessage) -> Option<LineChange>;
}

impl Room {
    /// `false`: a hook hides the line.
    pub(super) fn hooked(&self, m: &mut ChatMessage, emotes: &dyn Emotes) -> bool {
        let Some(change) = self.hook.as_ref().and_then(|h| h.check(&self.login, m)) else {
            return true;
        };
        if change.hide {
            return false;
        }
        apply(m, &change, emotes);
        true
    }

    /// S7 A plugin's change to a line that's in already: new text or a highlight in place;
    /// hiding it marks it deleted, in the user's look for deleted lines.
    pub fn change(&mut self, id: &str, change: &LineChange, emotes: &dyn Emotes) -> Vec<RoomEvent> {
        let Some(slot) = self.buffer.iter_mut().find(|m| m.id == id) else {
            return Vec::new();
        };
        let mut m = (**slot).clone();
        apply(&mut m, change, emotes);
        if change.hide {
            m.set(Flag::Deleted);
        }
        let m = Arc::new(m);
        *slot = m.clone();
        let mut events = vec![RoomEvent::Update { message: m }];
        if change.hide {
            events.push(RoomEvent::Delete { id: id.to_owned() });
        }
        events
    }

    /// S7 Swaps a line for one with the same id, where it was. `None`: it's gone.
    pub fn replace(&mut self, m: Arc<ChatMessage>) -> Option<RoomEvent> {
        let slot = self.buffer.iter_mut().find(|old| old.id == m.id)?;
        *slot = m.clone();
        Some(RoomEvent::Update { message: m })
    }
}

fn apply(m: &mut ChatMessage, change: &LineChange, emotes: &dyn Emotes) {
    if let Some(text) = &change.text {
        m.body = retext(&m.body, text, m.sender_id().unwrap_or(""), emotes);
    }
    if let Some(color) = &change.highlight {
        m.meta.highlight = Some(Highlight {
            kind: HighlightKind::Rule,
            color: (!color.is_empty()).then(|| color.clone()),
        });
    }
}

/// New text, tokenized again. Twitch emotes the line had keep working by name.
fn retext(old: &[Fragment], text: &str, sender: &str, emotes: &dyn Emotes) -> Vec<Fragment> {
    let twitch: HashMap<&str, &str> = old
        .iter()
        .filter_map(|f| match f {
            Fragment::Emote {
                provider: Provider::Twitch,
                id,
                name,
                ..
            } => Some((name.as_str(), id.as_str())),
            _ => None,
        })
        .collect();
    let known = Known { twitch, emotes };
    let input = Input {
        text,
        sender,
        emotes: "",
        gifs: "",
        bits: 0,
    };
    tokenize(&input, &[], &known).body
}

struct Known<'a> {
    twitch: HashMap<&'a str, &'a str>,
    emotes: &'a dyn Emotes,
}

impl Emotes for Known<'_> {
    fn find(&self, word: &str, sender: &str) -> Option<Found> {
        match self.twitch.get(word) {
            Some(id) => Some(Found::plain(Provider::Twitch, *id)),
            None => self.emotes.find(word, sender),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::irc::Line;
    use crate::model::{RoomEvent, plain};
    use crate::tokenize::NoEmotes;

    struct Upper;

    impl LineHook for Upper {
        fn check(&self, channel: &str, m: &ChatMessage) -> Option<LineChange> {
            assert_eq!(channel, "chan");
            let text = plain(&m.body);
            if text.contains("secret") {
                return Some(LineChange {
                    hide: true,
                    ..LineChange::default()
                });
            }
            Some(LineChange {
                text: Some(format!("{} Kappa", text.to_uppercase())),
                highlight: Some("#ff0000".into()),
                ..LineChange::default()
            })
        }
    }

    fn feed(r: &mut Room, id: &str, text: &str, emotes: &str) -> Vec<RoomEvent> {
        let raw =
            format!("@id={id};emotes={emotes};user-id=7;tmi-sent-ts=1000 :bob!bob@bob PRIVMSG #chan :{text}");
        r.handle_line(&Line::parse(&raw).unwrap(), 1_000, &NoEmotes)
    }

    #[test]
    fn hooks_hide_change_and_highlight() {
        let mut r = Room::new("chan", None);
        r.set_hook(Some(Arc::new(Upper)));
        // "Kappa" was a Twitch emote in the line, so it still is after the change.
        let events = feed(&mut r, "1", "hi Kappa", "25:3-7");
        let RoomEvent::Message { message } = &events[0] else {
            panic!("{events:?}")
        };
        assert_eq!(plain(&message.body), "HI KAPPA Kappa");
        assert!(
            matches!(&message.body[..], [Fragment::Text { .. }, Fragment::Emote { id, .. }] if id == "25")
        );
        let h = message.meta.highlight.as_ref().unwrap();
        assert_eq!(
            (h.kind, h.color.as_deref()),
            (HighlightKind::Rule, Some("#ff0000"))
        );
        assert!(feed(&mut r, "2", "a secret", "").is_empty());
        assert_eq!(r.snapshot().messages.len(), 1);
        // History goes through it too.
        let old = r.history(
            &["@id=0;user-id=7;tmi-sent-ts=500 :bob!bob@bob PRIVMSG #chan :old secret".into()],
            1_000,
            &NoEmotes,
        );
        assert!(old.is_empty());
    }

    #[test]
    fn lines_change_and_swap_in_place() {
        let mut r = Room::new("chan", None);
        feed(&mut r, "1", "first", "");
        feed(&mut r, "2", "second", "");
        let change = LineChange {
            text: Some("changed".into()),
            highlight: Some(String::new()),
            ..LineChange::default()
        };
        let events = r.change("1", &change, &NoEmotes);
        assert!(matches!(&events[..], [RoomEvent::Update { message }] if plain(&message.body) == "changed"));
        let hide = LineChange {
            hide: true,
            ..LineChange::default()
        };
        assert!(matches!(&r.change("2", &hide, &NoEmotes)[..], [RoomEvent::Update { .. }, RoomEvent::Delete { .. }]));
        assert!(r.change("gone", &hide, &NoEmotes).is_empty());
        let mut new = (*r.snapshot().messages[0]).clone();
        new.body = vec![Fragment::text("swapped")];
        assert!(r.replace(Arc::new(new)).is_some());
        let snap = r.snapshot();
        assert_eq!(plain(&snap.messages[0].body), "swapped");
        assert!(snap.messages[1].has(Flag::Deleted));
    }
}
