//! Message text to fragments (ARCHITECTURE §8): tag ranges first (Twitch emotes, GIFs),
//! then words: provider emotes (with zero-width stacking and modifiers, C7), mentions,
//! cheers, links.

mod links;
mod ranges;
mod targets;

pub use links::detect as detect_link;

use ranges::RangeKind;

use crate::model::{EmoteLook, Fragment, Layer, Provider, mods};

pub struct Input<'a> {
    pub text: &'a str,
    /// Sender's user id, for personal emotes (C9). Empty when unknown.
    pub sender: &'a str,
    /// Raw `emotes` tag, or "".
    pub emotes: &'a str,
    /// Raw `gifs` tag, or "".
    pub gifs: &'a str,
    pub bits: u32,
}

/// Emotes found by name in the word pass: provider emotes, personal ones (C9), and
/// Twitch emotes in our own messages (Twitch doesn't tag those).
pub trait Emotes {
    /// `sender`: the sender's user id, empty when unknown.
    fn find(&self, word: &str, sender: &str) -> Option<Found>;
}

pub struct NoEmotes;

impl Emotes for NoEmotes {
    fn find(&self, _: &str, _: &str) -> Option<Found> {
        None
    }
}

/// An emote matched by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub provider: Provider,
    pub id: String,
    /// C7 Drawn on top of the emote before it.
    pub zero_width: bool,
    /// C7 FFZ modifier emote: its flags go to the emote before it.
    pub modifier: Option<u32>,
    /// 2.4
    pub unlisted: bool,
    pub look: EmoteLook,
}

impl Found {
    pub fn plain(provider: Provider, id: impl Into<String>) -> Self {
        Self {
            provider,
            id: id.into(),
            zero_width: false,
            modifier: None,
            unlisted: false,
            look: EmoteLook::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Tokens {
    pub body: Vec<Fragment>,
    /// The text names the signed-in account.
    pub mentions_me: bool,
}

/// `me`: the account's login and display name, lowercase. Empty when anonymous.
pub fn tokenize(input: &Input<'_>, me: &[String], emotes: &dyn Emotes) -> Tokens {
    let text = input.text;
    let mut found = ranges::emotes(input.emotes);
    found.extend(ranges::gifs(input.gifs));

    let mut b = Builder {
        out: Vec::new(),
        pending: String::new(),
        me,
        emotes,
        sender: input.sender,
        bits_left: input.bits,
        mentions_me: false,
        base: None,
        held: None,
    };
    if found.is_empty() {
        b.words(text);
        return b.finish();
    }

    // Code point index → byte offset. ASCII text needs no table.
    let offsets: Option<Vec<usize>> = (!text.is_ascii())
        .then(|| text.char_indices().map(|(i, _)| i).chain([text.len()]).collect());
    let len = offsets.as_ref().map_or(text.len(), |o| o.len() - 1);
    let byte = |cp: usize| offsets.as_ref().map_or(cp, |o| o[cp]);

    let mut pos = 0;
    for r in ranges::clean(found, len) {
        let (start, end) = (byte(r.start), byte(r.end));
        b.words(&text[pos..start]);
        b.push(match r.kind {
            RangeKind::Emote { id } => Fragment::emote(Provider::Twitch, id, &text[start..end]),
            RangeKind::Gif { id, url } => Fragment::Gif {
                id: id.to_owned(),
                url: url.to_owned(),
            },
        });
        pos = end;
    }
    b.words(&text[pos..]);
    b.finish()
}

struct Builder<'a> {
    out: Vec<Fragment>,
    /// Plain text waiting to become one Text fragment.
    pending: String,
    me: &'a [String],
    emotes: &'a dyn Emotes,
    sender: &'a str,
    bits_left: u32,
    mentions_me: bool,
    /// C7 The emote zero-width emotes and modifiers land on: the last one, as long as
    /// only spaces followed it.
    base: Option<usize>,
    /// C7 BTTV modifier words, held until we know an emote follows.
    held: Option<Held>,
}

struct Held {
    flags: u32,
    /// The words as typed, to give back if no emote follows.
    text: String,
    /// Where in `pending` they were.
    at: usize,
}

/// C7 BTTV's modifier words, written before a third-party emote.
fn bttv_modifier(word: &str) -> Option<u32> {
    Some(match word {
        "w!" => mods::WIDE,
        "h!" => mods::FLIP_X,
        "v!" => mods::FLIP_Y,
        "z!" => mods::NO_SPACE,
        "c!" => mods::CURSED,
        "l!" => mods::ROTATE_LEFT,
        "r!" => mods::ROTATE_90,
        "p!" => mods::PARTY,
        "s!" => mods::SHAKE,
        _ => return None,
    })
}

impl Builder<'_> {
    fn words(&mut self, s: &str) {
        for (i, word) in s.split(' ').enumerate() {
            if i > 0 {
                self.pending.push(' ');
            }
            if !word.is_empty() {
                self.word(word);
            }
        }
    }

    fn word(&mut self, word: &str) {
        if let Some(flag) = bttv_modifier(word) {
            match &mut self.held {
                Some(h) => {
                    h.text.push_str(&self.pending[h.at..]);
                    self.pending.truncate(h.at);
                    h.text.push_str(word);
                    h.flags |= flag;
                }
                None => {
                    self.held = Some(Held {
                        flags: flag,
                        text: word.to_owned(),
                        at: self.pending.len(),
                    });
                }
            }
            return;
        }
        if let Some(found) = self.emotes.find(word, self.sender) {
            let plain = found.provider != Provider::Twitch && !found.zero_width && found.modifier.is_none();
            match self.held.take() {
                Some(h) if plain => {
                    self.pending.truncate(h.at);
                    if h.flags & mods::NO_SPACE != 0 && self.pending.ends_with(' ') {
                        self.pending.pop();
                    }
                    self.base = None;
                    self.emote(found, word);
                    if let Some(Fragment::Emote { mods: m, .. }) = self.out.last_mut() {
                        *m = Some(h.flags);
                    }
                }
                held => {
                    if let Some(h) = held {
                        self.release(h);
                    }
                    self.emote(found, word);
                }
            }
            return;
        }
        if let Some(h) = self.held.take() {
            self.release(h);
        }
        self.base = None;
        if let Some((login, tail)) = mention(word) {
            self.check_me(login);
            self.push(Fragment::Mention {
                text: word[..word.len() - tail.len()].to_owned(),
                login: login.to_ascii_lowercase(),
            });
            self.pending.push_str(tail);
            return;
        }
        if let Some((login, tail)) = channel(word) {
            self.push(Fragment::Channel {
                text: word[..word.len() - tail.len()].to_owned(),
                login: login.to_ascii_lowercase(),
            });
            self.pending.push_str(tail);
            return;
        }
        if let Some((prefix, amount)) = cheer(word, self.bits_left) {
            self.bits_left -= amount;
            self.push(Fragment::Cheer {
                prefix: prefix.to_owned(),
                amount,
            });
            return;
        }
        if let Some(link) = links::detect(word) {
            self.pending.push_str(link.lead);
            self.push(Fragment::Link {
                text: link.text.to_owned(),
                url: link.url,
                host: link.host,
                target: link.target,
            });
            self.pending.push_str(link.tail);
            return;
        }
        self.check_me(word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'));
        self.pending.push_str(word);
    }

    fn emote(&mut self, found: Found, name: &str) {
        if let Some(i) = self.base {
            let on_top = found.zero_width || found.modifier.is_some();
            if on_top && let Some(Fragment::Emote { stack, mods: base_mods, .. }) = self.out.get_mut(i) {
                // The spaces between belong to the stack now.
                self.pending.clear();
                let flags = found.modifier.unwrap_or(0);
                if flags & !mods::HIDDEN != 0 {
                    *base_mods = Some(base_mods.unwrap_or(0) | (flags & !mods::HIDDEN));
                }
                if flags & mods::HIDDEN == 0 {
                    stack.get_or_insert_with(Vec::new).push(Layer {
                        provider: found.provider,
                        id: found.id,
                        name: name.to_owned(),
                        look: found.look,
                    });
                }
                return;
            }
        }
        self.push(Fragment::Emote {
            provider: found.provider,
            id: found.id,
            name: name.to_owned(),
            stack: None,
            mods: None,
            unlisted: found.unlisted.then_some(true),
            look: found.look,
        });
    }

    fn check_me(&mut self, name: &str) {
        if !self.mentions_me && !name.is_empty() {
            self.mentions_me = self.me.iter().any(|m| {
                if name.is_ascii() {
                    m.eq_ignore_ascii_case(name)
                } else {
                    *m == name.to_lowercase()
                }
            });
        }
    }

    /// No emote followed the modifier words: they're plain text after all.
    fn release(&mut self, h: Held) {
        self.pending.insert_str(h.at, &h.text);
    }

    fn push(&mut self, f: Fragment) {
        if let Some(h) = self.held.take() {
            self.release(h);
        }
        self.flush();
        let emote = matches!(f, Fragment::Emote { .. });
        self.out.push(f);
        self.base = emote.then(|| self.out.len() - 1);
    }

    fn flush(&mut self) {
        if !self.pending.is_empty() {
            self.out.push(Fragment::text(std::mem::take(&mut self.pending)));
        }
    }

    fn finish(mut self) -> Tokens {
        if let Some(h) = self.held.take() {
            self.release(h);
        }
        self.flush();
        Tokens {
            body: self.out,
            mentions_me: self.mentions_me,
        }
    }
}

/// C9 Reads a message's plain text again, e.g. once its sender's personal emotes arrive
/// right after it. Everything already found stays. `None` if nothing new turned up.
pub fn retokenize(body: &[Fragment], sender: &str, emotes: &dyn Emotes) -> Option<Vec<Fragment>> {
    let mut out = Vec::with_capacity(body.len());
    let mut found = false;
    for f in body {
        let Fragment::Text { text } = f else {
            out.push(f.clone());
            continue;
        };
        let input = Input {
            text,
            sender,
            emotes: "",
            gifs: "",
            bits: 0,
        };
        let pieces = tokenize(&input, &[], emotes).body;
        found |= pieces.iter().any(|p| matches!(p, Fragment::Emote { .. }));
        out.extend(pieces);
    }
    found.then_some(out)
}

/// `@name,` → (`name`, `,`).
fn mention(word: &str) -> Option<(&str, &str)> {
    let rest = word.strip_prefix('@')?;
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    (1..=25).contains(&end).then(|| rest.split_at(end))
}

/// V102-05 `#name,` → (`name`, `,`). Three to 25 name characters, not starting with `_`
/// and not only digits, so `#1` and `#2026` stay text.
fn channel(word: &str) -> Option<(&str, &str)> {
    let rest = word.strip_prefix('#')?;
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let name = &rest[..end];
    let ok = (3..=25).contains(&end) && !name.starts_with('_') && !name.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| rest.split_at(end))
}

/// `cheer100` → (`cheer`, 100), only while the message's bits aren't used up.
fn cheer(word: &str, bits_left: u32) -> Option<(&str, u32)> {
    if bits_left == 0 {
        return None;
    }
    let split = word.find(|c: char| c.is_ascii_digit())?;
    let (prefix, digits) = word.split_at(split);
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_alphabetic()) || digits.len() > 7 {
        return None;
    }
    let amount: u32 = digits.parse().ok()?;
    (amount > 0 && amount <= bits_left).then_some((prefix, amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, emotes: &str) -> Vec<Fragment> {
        tokenize(
            &Input {
                text,
                sender: "",
                emotes,
                gifs: "",
                bits: 0,
            },
            &[],
            &NoEmotes,
        )
        .body
    }

    fn emote(id: &str, name: &str) -> Fragment {
        Fragment::emote(Provider::Twitch, id, name)
    }

    #[test]
    fn plain_text_is_one_fragment() {
        assert_eq!(run("hello  there ", ""), [Fragment::text("hello  there ")]);
        assert!(run("", "").is_empty());
    }

    #[test]
    fn twitch_emotes_by_code_point() {
        assert_eq!(
            run("Kappa hi Kappa", "25:0-4,9-13"),
            [emote("25", "Kappa"), Fragment::text(" hi "), emote("25", "Kappa")]
        );
        // Emoji take one code point each but several bytes (and two UTF-16 units).
        assert_eq!(
            run("👋🏽 Kappa ü", "25:3-7"),
            [Fragment::text("👋🏽 "), emote("25", "Kappa"), Fragment::text(" ü")]
        );
    }

    #[test]
    fn bad_emote_ranges_are_ignored() {
        assert_eq!(run("hi", "25:0-9"), [Fragment::text("hi")]);
    }

    #[test]
    fn channel_names() {
        assert_eq!(
            run("go to #Forsen, or #xqc!", ""),
            [
                Fragment::text("go to "),
                Fragment::Channel {
                    text: "#Forsen".into(),
                    login: "forsen".into()
                },
                Fragment::text(", or "),
                Fragment::Channel {
                    text: "#xqc".into(),
                    login: "xqc".into()
                },
                Fragment::text("!"),
            ]
        );
        // Too short, only digits, a leading underscore, a lone `#`: plain text.
        for text in ["#1", "#gg", "#2026", "#_abc", "#", "##", "a#bcd"] {
            assert_eq!(run(text, ""), [Fragment::text(text)], "{text}");
        }
    }

    #[test]
    fn mentions_and_links() {
        let body = run("@Foo, look: twitch.tv/bar.", "");
        assert_eq!(
            body,
            [
                Fragment::Mention {
                    text: "@Foo".into(),
                    login: "foo".into()
                },
                Fragment::text(", look: "),
                Fragment::Link {
                    text: "twitch.tv/bar".into(),
                    url: "https://twitch.tv/bar".into(),
                    host: "twitch.tv".into(),
                    target: None,
                },
                Fragment::text("."),
            ]
        );
    }

    #[test]
    fn detects_mentions_of_me() {
        let me = ["cyex".to_owned(), "サイ".to_owned()];
        let t = |text: &str| {
            tokenize(
                &Input {
                    text,
                    sender: "",
                    emotes: "",
                    gifs: "",
                    bits: 0,
                },
                &me,
                &NoEmotes,
            )
            .mentions_me
        };
        assert!(t("hey CYEX!"));
        assert!(t("@cyex hi"));
        assert!(t("サイ さん"));
        assert!(!t("cyexfan hi"));
        assert!(!t("hello"));
    }

    #[test]
    fn cheers_only_up_to_the_bits_sent() {
        let body = tokenize(
            &Input {
                sender: "",
                text: "Cheer100 Cheer50 abc123",
                emotes: "",
                gifs: "",
                bits: 100,
            },
            &[],
            &NoEmotes,
        )
        .body;
        assert_eq!(
            body,
            [
                Fragment::Cheer {
                    prefix: "Cheer".into(),
                    amount: 100
                },
                Fragment::text(" Cheer50 abc123"),
            ]
        );
    }

    #[test]
    fn emotes_by_name() {
        struct Kappa;
        impl Emotes for Kappa {
            fn find(&self, word: &str, _: &str) -> Option<Found> {
                (word == "Kappa").then(|| Found::plain(Provider::Twitch, "25"))
            }
        }
        let body = tokenize(
            &Input {
                sender: "",
                text: "hi Kappa",
                emotes: "",
                gifs: "",
                bits: 0,
            },
            &[],
            &Kappa,
        )
        .body;
        assert_eq!(body, [Fragment::text("hi "), emote("25", "Kappa")]);
    }

    #[test]
    fn gifs_replace_their_range() {
        let body = tokenize(
            &Input {
                sender: "",
                text: "look [gif] nice",
                emotes: "",
                gifs: "5-9|g1|https://media.example/g1.gif",
                bits: 0,
            },
            &[],
            &NoEmotes,
        )
        .body;
        assert_eq!(body.len(), 3);
        assert!(matches!(&body[1], Fragment::Gif { id, .. } if id == "g1"));
    }

    /// 7TV-style: `zw` is zero-width, `mod` an FFZ modifier that flips, `hid` a hidden one.
    struct Stack;
    impl Emotes for Stack {
        fn find(&self, word: &str, sender: &str) -> Option<Found> {
            let mut f = Found::plain(Provider::Seventv, word);
            match word {
                "base" | "other" => {}
                "zw" => f.zero_width = true,
                "mod" => f.modifier = Some(mods::FLIP_X),
                "hid" => f.modifier = Some(mods::HIDDEN | mods::GROW_X),
                "mine" if sender == "7" => {}
                _ => return None,
            }
            Some(f)
        }
    }

    fn stacked(text: &str, sender: &str) -> Vec<Fragment> {
        let input = Input {
            text,
            sender,
            emotes: "",
            gifs: "",
            bits: 0,
        };
        tokenize(&input, &[], &Stack).body
    }

    #[test]
    fn zero_width_emotes_stack_on_the_last_emote() {
        let body = stacked("base zw  zw hi zw", "");
        assert_eq!(body.len(), 3, "{body:?}");
        let Fragment::Emote { stack: Some(stack), .. } = &body[0] else { panic!("{body:?}") };
        assert_eq!(stack.len(), 2);
        assert_eq!(body[1], Fragment::text(" hi "));
        // Nothing to sit on after a word: shown on its own.
        assert!(matches!(&body[2], Fragment::Emote { stack: None, id, .. } if id == "zw"));
    }

    #[test]
    fn zero_width_stacks_on_twitch_emotes_too() {
        let input = Input {
            text: "Kappa zw",
            sender: "",
            emotes: "25:0-4",
            gifs: "",
            bits: 0,
        };
        let body = tokenize(&input, &[], &Stack).body;
        assert_eq!(body.len(), 1);
        assert!(matches!(&body[0], Fragment::Emote { stack: Some(s), .. } if s[0].id == "zw"));
    }

    #[test]
    fn modifiers_change_the_emote_before() {
        let body = stacked("base mod hid other", "");
        assert_eq!(body.len(), 3, "{body:?}");
        let Fragment::Emote { mods: Some(m), stack, .. } = &body[0] else { panic!("{body:?}") };
        assert_eq!(*m, mods::FLIP_X | mods::GROW_X);
        // The visible modifier sits on top; the hidden one is gone.
        assert_eq!(stack.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn personal_emotes_need_their_sender() {
        assert!(matches!(&stacked("mine", "7")[0], Fragment::Emote { .. }));
        assert_eq!(stacked("mine", "8"), [Fragment::text("mine")]);
    }

    #[test]
    fn bttv_modifier_words_change_the_next_emote() {
        let body = stacked("hi h! v! base", "");
        assert_eq!(body[0], Fragment::text("hi "));
        let Fragment::Emote { mods: Some(m), .. } = &body[1] else { panic!("{body:?}") };
        assert_eq!(*m, mods::FLIP_X | mods::FLIP_Y);
        // No emote after them: they stay as typed.
        assert_eq!(stacked("h!  v! hi", ""), [Fragment::text("h!  v! hi")]);
        assert_eq!(stacked("w! hi base", ""), [Fragment::text("w! hi "), stacked("base", "")[0].clone()]);
        // Not for Twitch emotes.
        let input = Input {
            text: "w! Kappa",
            sender: "",
            emotes: "25:3-7",
            gifs: "",
            bits: 0,
        };
        assert_eq!(tokenize(&input, &[], &Stack).body[0], Fragment::text("w! "));
        // z! eats the space before the emote.
        assert_eq!(stacked("a z! other", "")[0], Fragment::text("a"));
    }

    #[test]
    fn retokenizing_finds_late_emotes() {
        let body = vec![emote("25", "Kappa"), Fragment::text(" mine and more")];
        let again = retokenize(&body, "7", &Stack).unwrap();
        assert_eq!(again.len(), 4);
        assert!(matches!(&again[2], Fragment::Emote { id, .. } if id == "mine"));
        assert!(retokenize(&body, "8", &Stack).is_none());
    }
}
