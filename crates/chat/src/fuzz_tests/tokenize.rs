//! V110-F89 Tokenizer on generated text and generated emote/GIF range tags.

use crate::fuzz::{Rng, byte_prefixes};
use crate::model::{Fragment, Provider, mods};
use crate::tokenize::{Emotes, Found, Input, NoEmotes, detect_link, retokenize, tokenize, trusted_link, trusted_media};

/// Matches words by their first byte, so every kind of provider emote shows up.
struct Fake;

impl Emotes for Fake {
    fn find(&self, word: &str, _sender: &str) -> Option<Found> {
        let b = *word.as_bytes().first()?;
        let mut f = Found::plain(
            match b % 4 {
                0 => Provider::Seventv,
                1 => Provider::Bttv,
                2 => Provider::Ffz,
                _ => Provider::Twitch,
            },
            word,
        );
        match b % 11 {
            0 => f.zero_width = true,
            1 => f.modifier = Some(mods::HIDDEN),
            2 => f.modifier = Some(mods::SHAKE | mods::FLIP_X),
            3 => return None,
            _ => {}
        }
        Some(f)
    }
}

/// The text a fragment stands for, when it is a verbatim slice of the message.
fn verbatim(f: &Fragment) -> Option<&str> {
    match f {
        Fragment::Text { text } | Fragment::Mention { text, .. } | Fragment::Channel { text, .. } => Some(text),
        Fragment::Emote { name, .. } => Some(name),
        Fragment::Link { text, .. } => Some(text),
        _ => None,
    }
}

fn run(text: &str, emotes: &str, gifs: &str, bits: u32, me: &[String], dict: &dyn Emotes) -> Vec<Fragment> {
    tokenize(&Input { text, sender: "1", emotes, gifs, bits }, me, dict).body
}

/// A message made of fragments we can't read back (a GIF replaces its words, zero-width
/// emotes eat the spaces between) isn't expected to round-trip.
fn gen_emote_tag(r: &mut Rng, len: usize) -> String {
    let mut tag = String::new();
    for e in 0..r.below(4) {
        if e > 0 {
            tag.push('/');
        }
        tag.push_str(&format!("{}:", if r.chance(30) { r.text(6) } else { r.below(500).to_string() }));
        for s in 0..1 + r.below(4) {
            if s > 0 {
                tag.push(',');
            }
            let pos = |r: &mut Rng| match r.below(8) {
                0 => r.number_text(),
                1 => (len + r.below(3)).to_string(),
                2 => len.saturating_sub(1).to_string(),
                3 => r.text(2),
                _ => r.below(len + 2).to_string(),
            };
            tag.push_str(&format!("{}-{}", pos(r), pos(r)));
        }
    }
    tag
}

fn gen_gif_tag(r: &mut Rng, len: usize) -> String {
    let mut tag = String::new();
    for g in 0..r.below(3) {
        if g > 0 {
            tag.push(',');
        }
        let a = r.below(len + 2);
        let b = a + r.below(6);
        let url = ["https://a.b/c,d.gif", "https://x.y/z", "", "a|b|c", "\u{130},1-2|x|y"][r.below(5)];
        tag.push_str(&format!("{a}-{b}|{}|{url}", r.text(4)));
    }
    tag
}

#[test]
fn tokenize_never_panics_with_random_ranges() {
    let mut r = Rng::new(0x70C3);
    let me = ["cyex".to_owned(), "サイ".to_owned(), "\u{130}x".to_owned()];
    for i in 0..4000 {
        let text = match i % 3 {
            0 => r.text(40),
            1 => r.words(10),
            _ => r.lossy(60),
        };
        let len = text.chars().count();
        let emotes = if r.chance(70) { gen_emote_tag(&mut r, len) } else { r.text(20) };
        let gifs = if r.chance(30) { gen_gif_tag(&mut r, len) } else { String::new() };
        let bits = if r.chance(30) { r.below(2_000_000_000) as u32 } else { 0 };
        for dict in [&NoEmotes as &dyn Emotes, &Fake] {
            let body = run(&text, &emotes, &gifs, bits, &me, dict);
            // The result can be read again by the C9 pass.
            let _ = retokenize(&body, "1", &Fake);
        }
    }
}

#[test]
fn fragments_spell_the_message_back_without_gifs_bits_or_provider_emotes() {
    let mut r = Rng::new(0xF4A6);
    for i in 0..5000 {
        let text = match i % 4 {
            0 => r.text(40),
            1 => r.words(10),
            2 => r.mutate("Kappa hi https://twitch.tv/x.mp4, (a.com/b) @Foo, #chan! cheer100 w! x"),
            _ => r.lossy(60),
        };
        let len = text.chars().count();
        let emotes = if r.chance(60) { gen_emote_tag(&mut r, len) } else { String::new() };
        let body = run(&text, &emotes, "", 0, &[], &NoEmotes);
        let joined: Option<String> = body.iter().map(verbatim).collect::<Option<Vec<_>>>().map(|v| v.concat());
        // V110-F94 As shown: without control characters and bidi overrides.
        assert_eq!(joined.as_deref(), Some(&*crate::tokenize::shown(&text)), "text {text:?} emotes {emotes:?}");
        // No two text fragments in a row, none empty.
        for w in body.windows(2) {
            assert!(!matches!(w, [Fragment::Text { .. }, Fragment::Text { .. }]), "{text:?} {emotes:?}");
        }
        assert!(body.iter().all(|f| !matches!(f, Fragment::Text { text } if text.is_empty())), "{text:?} {emotes:?}");
    }
}

#[test]
fn emote_ranges_count_code_points_not_bytes_or_utf16_units() {
    // Every position of a multi-byte message: a range that fits must cut exactly that slice.
    let text = "\u{1F468}\u{200D}\u{1F469} \u{130}Kappa\u{212A}\u{1E9E}ß\u{301}e \u{1F3FD}x";
    let chars: Vec<char> = text.chars().collect();
    for a in 0..chars.len() {
        for b in a..chars.len() + 2 {
            let body = run(text, &format!("9:{a}-{b}"), "", 0, &[], &NoEmotes);
            let emote = body.iter().find_map(|f| match f {
                Fragment::Emote { name, .. } => Some(name.clone()),
                _ => None,
            });
            if b < chars.len() {
                let want: String = chars[a..=b].iter().collect();
                assert_eq!(emote.as_deref(), Some(want.as_str()), "{a}-{b}");
            } else {
                assert_eq!(emote, None, "{a}-{b} runs past the end and is dropped");
            }
        }
    }
}

#[test]
fn huge_and_odd_tags_do_not_panic() {
    let t = "hello wörld \u{1F600}";
    for tag in [
        "25:18446744073709551615-18446744073709551615",
        "25:0-18446744073709551615",
        "25:-1-3",
        "25:3--1",
        "25:::0-1",
        ":0-1",
        "/",
        "//:/:",
        "25:0-1,",
        ",,,",
        "25:0-1/25:0-1/25:0-1",
        "25:1-0",
        "25:00000000000000000000000000-1",
        "25:\u{FF11}-\u{FF12}",
    ] {
        let _ = run(t, tag, "", 0, &[], &NoEmotes);
    }
    for gif in ["", ",", "|", "||", "0-1||", "0-1|a|", "0-1|a|b,", ",0-1|a|b", "0-1|a|b,0-1|c|d", "18446744073709551615-18446744073709551615|a|b"] {
        let _ = run(t, "", gif, 0, &[], &NoEmotes);
    }
    // Very large tags and texts.
    let big = "Kappa ".repeat(20_000);
    let tag = (0..20_000).map(|i| format!("25:{}-{}", i * 6, i * 6 + 4)).collect::<Vec<_>>().join("/");
    let start = std::time::Instant::now();
    let body = run(&big, &tag, "", 0, &[], &NoEmotes);
    assert_eq!(body.iter().filter(|f| matches!(f, Fragment::Emote { .. })).count(), 20_000);
    assert!(start.elapsed().as_secs() < 5, "big tag took {:?}", start.elapsed());
}

#[test]
fn many_modifier_and_zero_width_words_do_not_panic() {
    let mut r = Rng::new(0x20E0);
    let toks = ["w!", "h!", "v!", "z!", "c!", "l!", "r!", "p!", "s!", "aaa", "bbb", "ccc", "kappa", "  ", " ", "@x", "#abc", "cheer5", "x.com"];
    for _ in 0..4000 {
        let n = r.below(14);
        let text = (0..n).map(|_| *r.pick(&toks)).collect::<Vec<_>>().join(if r.chance(70) { " " } else { "  " });
        let body = run(&text, "", "", 1000, &[], &Fake);
        let _ = retokenize(&body, "1", &Fake);
    }
}

#[test]
fn prefixes_and_mutations_of_a_busy_message_do_not_panic() {
    let seed = "Kappa @Foo, https://www.twitch.tv/videos/123?t=1h2m3s (twitch.tv/bar.) #chan cheer100 w! LUL \u{1F468}\u{200D}\u{1F469} x";
    for p in byte_prefixes(seed) {
        let _ = run(&p, "25:0-4,60-62", "", 100, &["foo".into()], &Fake);
    }
    let mut r = Rng::new(9);
    for _ in 0..3000 {
        let m = r.mutate(seed);
        let _ = run(&m, "25:0-4", "0-1|a|https://x.y", 5000, &["foo".into()], &Fake);
    }
}

#[test]
fn link_detection_never_panics_and_keeps_the_word_whole() {
    let mut r = Rng::new(0x11A4);
    let seeds = [
        "https://twitch.tv/clip/abc?x=1", "(twitch.tv/bar.)", "http://\u{FF4C}ocalhost", "[a.com]", "\"example.org/x\",",
        "xn--e.com", "a@b.com", "user:pw@host.com/x", "http://[::1]/", "http://[::1", "https://a.b:99999/",
        "http://1.2.3.4.5", "https://\u{130}.tv/", "bit.ly/x", "www.twitch.tv/videos/1", "https://clips.twitch.tv/Abc-Def",
        "twitch.tv/videos/99999999999999999999", "https://twitch.tv/x/clip/y?t=zzz", "https://%00.com/",
    ];
    let mut all: Vec<String> = Vec::new();
    for s in seeds {
        all.extend(byte_prefixes(s));
        for _ in 0..200 {
            all.push(r.mutate(s));
        }
    }
    for _ in 0..3000 {
        all.push(r.text(40));
    }
    all.push(format!("https://{}.com", "a".repeat(20_000)));
    all.push(format!("{}.com/{}", "\u{130}".repeat(500), "%".repeat(5000)));
    all.push("(".repeat(10_000) + "a.com" + &")".repeat(10_000));
    for w in all {
        if let Some(l) = detect_link(&w) {
            assert_eq!(format!("{}{}{}", l.lead, l.text, l.tail), w, "{w:?}");
            assert!(l.url.starts_with("http://") || l.url.starts_with("https://"), "{w:?} -> {}", l.url);
            // These must not panic either.
            let _ = (trusted_link(&l.url), trusted_media(&l.url));
        }
        let _ = trusted_link(&w);
        let _ = trusted_media(&w);
    }
}
