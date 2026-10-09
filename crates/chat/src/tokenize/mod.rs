//! Message text to fragments (ARCHITECTURE §8): tag ranges first (Twitch emotes, GIFs),
//! then words: mentions, cheers, links. Phase 7 adds provider emotes to the word pass.

mod links;
mod ranges;

pub use links::detect as detect_link;

use ranges::RangeKind;

use crate::model::{Fragment, Provider};

pub struct Input<'a> {
    pub text: &'a str,
    /// Raw `emotes` tag, or "".
    pub emotes: &'a str,
    /// Raw `gifs` tag, or "".
    pub gifs: &'a str,
    pub bits: u32,
}

/// Emotes found by name in the word pass: Twitch emotes in our own messages (Twitch
/// doesn't tag those), provider emotes from phase 7.
pub trait Emotes {
    fn find(&self, word: &str) -> Option<(Provider, String)>;
}

pub struct NoEmotes;

impl Emotes for NoEmotes {
    fn find(&self, _: &str) -> Option<(Provider, String)> {
        None
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
        bits_left: input.bits,
        mentions_me: false,
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
            RangeKind::Emote { id } => Fragment::Emote {
                provider: Provider::Twitch,
                id: id.to_owned(),
                name: text[start..end].to_owned(),
            },
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
    bits_left: u32,
    mentions_me: bool,
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
        if let Some((provider, id)) = self.emotes.find(word) {
            self.push(Fragment::Emote {
                provider,
                id,
                name: word.to_owned(),
            });
            return;
        }
        if let Some((login, tail)) = mention(word) {
            self.check_me(login);
            self.push(Fragment::Mention {
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
            });
            self.pending.push_str(link.tail);
            return;
        }
        self.check_me(word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'));
        self.pending.push_str(word);
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

    fn push(&mut self, f: Fragment) {
        self.flush();
        self.out.push(f);
    }

    fn flush(&mut self) {
        if !self.pending.is_empty() {
            self.out.push(Fragment::text(std::mem::take(&mut self.pending)));
        }
    }

    fn finish(mut self) -> Tokens {
        self.flush();
        Tokens {
            body: self.out,
            mentions_me: self.mentions_me,
        }
    }
}

/// `@name,` → (`name`, `,`).
fn mention(word: &str) -> Option<(&str, &str)> {
    let rest = word.strip_prefix('@')?;
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    (1..=25).contains(&end).then(|| rest.split_at(end))
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
        Fragment::Emote {
            provider: Provider::Twitch,
            id: id.into(),
            name: name.into(),
        }
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
                    host: "twitch.tv".into()
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
            fn find(&self, word: &str) -> Option<(Provider, String)> {
                (word == "Kappa").then(|| (Provider::Twitch, "25".to_owned()))
            }
        }
        let body = tokenize(
            &Input {
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
}
