//! V107-Bn The checks Twitch's web chat runs before a cheer leaves (research "Cheering"):
//! the cheer words, their total, the channel's minimums and the balance. Pure.

use cyex_core::number::grouped;

use super::CheerRules;

/// The largest single cheer emote.
pub const MAX_EMOTE: u64 = 100_000;
/// A cheer's text, cheer words included.
pub const MAX_CHARS: usize = 500;
/// The user chose no anonymous cheering (V107-B).
const ANON: &str = "anon";

/// A cheer word: `Cheer100` → (`cheer`, 100).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// Lowercase.
    pub prefix: String,
    pub amount: u64,
}

/// What the checks found: the total to spend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cheer {
    pub bits: u32,
    pub words: Vec<Word>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Command,
    TooLong(usize),
    NoCheer,
    Anonymous,
    DisplayOnly(String),
    TooLarge,
    Off,
    NoBits,
    Short(u64),
    BelowMin(u32),
    EmoteMin(u32),
    UnknownBalance,
}

impl Refusal {
    pub fn text(&self) -> String {
        match self {
            Self::Command => "Commands can’t carry a cheer.".into(),
            Self::TooLong(n) => format!("Too long: {}/{MAX_CHARS}.", grouped(*n as u64)),
            Self::NoCheer => "No cheer in that message.".into(),
            Self::Anonymous => "Anonymous cheers are off.".into(),
            Self::DisplayOnly(p) => format!("{p} can’t be cheered."),
            Self::TooLarge => format!("{} is the largest cheer emote.", grouped(MAX_EMOTE)),
            Self::Off => "Cheering is off in this channel.".into(),
            Self::NoBits => "You have 0 Bits.".into(),
            Self::Short(n) => format!("You need {} more Bits.", grouped(*n)),
            Self::BelowMin(n) => format!("This channel’s minimum cheer is {} Bits.", grouped(u64::from(*n))),
            Self::EmoteMin(n) => format!("Each cheer emote needs at least {} Bits here.", grouped(u64::from(*n))),
            Self::UnknownBalance => "Your Bits balance didn’t load. Try again.".into(),
        }
    }
}

/// Twitch's rule: a whole word `^([A-Za-z0-9]+[A-Za-z])(\d+)$`.
fn split(word: &str) -> Option<(&str, &str)> {
    let cut = word.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (prefix, digits) = word.split_at(cut);
    let ok = !digits.is_empty()
        && prefix.len() >= 2
        && prefix.chars().all(|c| c.is_ascii_alphanumeric())
        && prefix.ends_with(|c: char| c.is_ascii_alphabetic());
    ok.then_some((prefix, digits))
}

/// The cheer words in `text` whose prefix is one of `known` (lowercase). Amounts too big
/// to read count as over the limit.
pub fn words(text: &str, known: impl Fn(&str) -> bool) -> Vec<Word> {
    text.split_whitespace()
        .filter_map(split)
        .filter_map(|(p, d)| {
            let prefix = p.to_ascii_lowercase();
            known(&prefix).then(|| Word {
                prefix,
                amount: d.parse().unwrap_or(u64::MAX),
            })
        })
        .collect()
}

/// A slash or dot command, which is never a cheer (`/me` and `.me` are).
pub fn is_command(text: &str) -> bool {
    let t = text.trim_start();
    let Some(rest) = t.strip_prefix(['/', '.']) else {
        return false;
    };
    // `/me` then any space, like the web's `/^[/.]me\s/i`.
    let me = rest.get(..2).is_some_and(|h| h.eq_ignore_ascii_case("me"))
        && rest[2..].starts_with(char::is_whitespace);
    !me
}

/// The total of a message's cheer, or why it can't go out. Same order as the web client.
pub fn check(text: &str, rules: &CheerRules) -> Result<Cheer, Refusal> {
    let text = text.trim();
    if is_command(text) {
        return Err(Refusal::Command);
    }
    let chars = text.chars().count();
    if chars > MAX_CHARS {
        return Err(Refusal::TooLong(chars));
    }
    let kind = |p: &str| rules.prefixes.iter().find(|c| c.prefix == p).map(|c| c.kind.as_str());
    let words = words(text, |p| kind(p).is_some());
    let total: u64 = words.iter().map(|w| w.amount).fold(0, u64::saturating_add);
    if total == 0 {
        return Err(Refusal::NoCheer);
    }
    if words.iter().any(|w| w.prefix == ANON || kind(&w.prefix) == Some("ANONYMOUS")) {
        return Err(Refusal::Anonymous);
    }
    if let Some(w) = words.iter().find(|w| kind(&w.prefix) == Some("DISPLAY_ONLY")) {
        let shown = text
            .split_whitespace()
            .find_map(|t| split(t).filter(|(p, _)| p.eq_ignore_ascii_case(&w.prefix)).map(|(p, _)| p))
            .unwrap_or(&w.prefix);
        return Err(Refusal::DisplayOnly(shown.to_owned()));
    }
    if words.iter().any(|w| w.amount > MAX_EMOTE) {
        return Err(Refusal::TooLarge);
    }
    if !rules.enabled {
        return Err(Refusal::Off);
    }
    let balance = rules.balance.ok_or(Refusal::UnknownBalance)?;
    if total > balance {
        return Err(if balance == 0 {
            Refusal::NoBits
        } else {
            Refusal::Short(total - balance)
        });
    }
    if total < u64::from(rules.cheer_min) {
        return Err(Refusal::BelowMin(rules.cheer_min));
    }
    let emote_min = rules.emote_min.max(1);
    if words.iter().any(|w| w.amount < u64::from(emote_min)) {
        return Err(Refusal::EmoteMin(emote_min));
    }
    let bits = u32::try_from(total).map_err(|_| Refusal::TooLarge)?;
    Ok(Cheer { bits, words })
}

#[cfg(test)]
mod tests {
    use super::super::CheerPrefix;
    use super::*;

    fn rules(balance: u64) -> CheerRules {
        let p = |prefix: &str, kind: &str| CheerPrefix {
            prefix: prefix.into(),
            kind: kind.into(),
        };
        CheerRules {
            channel_id: "1".into(),
            enabled: true,
            cheer_min: 1,
            emote_min: 1,
            prefixes: vec![
                p("cheer", "DEFAULT"),
                p("4head", "FIRST_PARTY"),
                p("anon", "DISPLAY_ONLY"),
                p("charity", "DISPLAY_ONLY"),
                p("mycheer", "CUSTOM"),
            ],
            balance: Some(balance),
        }
    }

    #[test]
    fn v107_bn_cheer_words_follow_twitchs_regex() {
        let all = |_: &str| true;
        let w = words("Cheer100 4Head5 a1 Cheer abc 100 Cheer100x gg2ez ShowLove1000", all);
        let got: Vec<_> = w.iter().map(|w| (w.prefix.as_str(), w.amount)).collect();
        assert_eq!(got, [("cheer", 100), ("4head", 5), ("showlove", 1000)]);
        let known = |p: &str| p == "cheer";
        assert_eq!(words("cheer1 Kappa1 CHEER2", known).len(), 2);
        assert_eq!(words("Cheer99999999999999999999", all)[0].amount, u64::MAX);
    }

    #[test]
    fn v107_h1_cheer_lookalikes_are_not_cheers() {
        let known = |p: &str| p == "cheer";
        for t in ["cheers100", "Cheer100Kappa", "Cheer１００", "Chéer100", "Cheer100!", "Cheer\u{200b}100", "xCheer100"] {
            assert!(words(t, known).is_empty(), "{t}");
        }
        assert_eq!(words("gg\u{a0}Cheer5", known).len(), 1, "a no-break space splits like the box");
        assert_eq!(check("Cheer0100", &rules(500)).unwrap().bits, 100);
    }

    #[test]
    fn v107_bn_commands_are_never_cheers() {
        assert!(is_command("/ban x Cheer100"));
        assert!(is_command(".color red"));
        assert!(!is_command("/me Cheer100"));
        assert!(!is_command(".ME Cheer100"));
        assert!(!is_command("Cheer100 /lol"));
        assert!(!is_command("/me\tCheer100"));
        assert!(is_command("/me"));
        assert!(is_command("/meCheer100"));
        assert!(is_command(" .gg Cheer100"));
        assert_eq!(check("/w x Cheer100", &rules(500)), Err(Refusal::Command));
    }

    #[test]
    fn v107_bn_total_and_ok() {
        let c = check("Cheer100 gg 4Head50 mycheer1", &rules(500)).unwrap();
        assert_eq!(c.bits, 151);
        assert_eq!(check("/me Cheer10 hype", &rules(500)).unwrap().bits, 10);
    }

    #[test]
    fn v107_bn_refusals_in_web_order() {
        assert_eq!(check("hello Kappa1", &rules(500)), Err(Refusal::NoCheer));
        assert_eq!(check("Cheer0", &rules(500)), Err(Refusal::NoCheer));
        assert_eq!(check("Anon100 hi", &rules(500)), Err(Refusal::Anonymous));
        assert_eq!(check("Charity5", &rules(500)), Err(Refusal::DisplayOnly("Charity".into())));
        assert_eq!(check("Cheer100001", &rules(u64::MAX)), Err(Refusal::TooLarge));
        assert_eq!(check(&format!("Cheer1 {}", "a".repeat(500)), &rules(5)), Err(Refusal::TooLong(507)));
        let mut off = rules(500);
        off.enabled = false;
        assert_eq!(check("Cheer1", &off), Err(Refusal::Off));
        assert_eq!(check("Cheer1", &rules(0)), Err(Refusal::NoBits));
        assert_eq!(check("Cheer100 Cheer50", &rules(120)), Err(Refusal::Short(30)));
        let mut min = rules(5000);
        min.cheer_min = 200;
        assert_eq!(check("Cheer100", &min), Err(Refusal::BelowMin(200)));
        min.emote_min = 50;
        assert_eq!(check("Cheer190 Cheer10", &min), Err(Refusal::EmoteMin(50)));
        assert_eq!(check("Cheer190 Cheer10 Cheer0", &rules(500)), Err(Refusal::EmoteMin(1)));
        let mut unknown = rules(0);
        unknown.balance = None;
        assert_eq!(check("Cheer1", &unknown), Err(Refusal::UnknownBalance));
    }

    #[test]
    fn v107_bn_refusal_texts_group_numbers() {
        assert_eq!(Refusal::Short(1500).text(), "You need 1.500 more Bits.");
        assert_eq!(Refusal::TooLarge.text(), "100.000 is the largest cheer emote.");
    }
}
