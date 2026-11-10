//! V110-F89 IRC line parsing and tag escaping on generated input.

use crate::fuzz::{Rng, byte_prefixes};
use crate::irc::write::{Outgoing, privmsg};
use crate::irc::{Line, escape, unescape};

const SEEDS: &[&str] = &[
    "@badge-info=subscriber/8;color=#FF0000;display-name=Foo;emotes=25:0-4;id=abc;mod=0 :foo!foo@foo.tmi.twitch.tv PRIVMSG #bar :Kappa hello :) there\r\n",
    "PING :tmi.twitch.tv",
    ":tmi.twitch.tv 001 justinfan1 :Welcome, GLHF!",
    ":tmi.twitch.tv CAP * ACK :twitch.tv/tags twitch.tv/commands",
    ":a!a@a.tmi.twitch.tv JOIN #chan",
    "@emote-only=0;room-id=1;slow=10 :tmi.twitch.tv ROOMSTATE #chan",
    "@id=a;historical=1 :regressz!regressz@regressz.tmi.twitch.tv PRIVMSG #satxris plaksa",
    ":a!a@a PRIVMSG #chan \u{1F68C} ",
    "@flag;x= :p CMD",
    r"@system-msg=5\sraiders\:\sfrom\\here\n :tmi.twitch.tv USERNOTICE #c",
    "@msg-id=raid;msg-param-viewerCount=18446744073709551616;tmi-sent-ts=-1 :tmi.twitch.tv USERNOTICE #c :hi",
];

/// Calls every accessor so a lazy slice that would panic actually runs.
fn touch(raw: &str) {
    let Some(l) = Line::parse(raw) else { return };
    let _ = (l.command, l.nick(), l.channel(), l.trailing());
    let _ = l.middle().count();
    for (k, v) in l.tags() {
        let _ = (l.tag(k), l.raw_tag(k), l.tag_nonempty(k), l.tag_flag(k), unescape(v));
        let _ = (l.tag_num::<u64>(k), l.tag_num::<i64>(k), l.tag_num::<u32>(k), l.tag_num::<f64>(k));
    }
    let _ = l.tag("emotes");
    let _ = l.tag("");
}

#[test]
fn line_parse_never_panics_on_random_text() {
    let mut r = Rng::new(0x1CC1);
    for _ in 0..4000 {
        touch(&r.text(60));
        touch(&r.words(12));
        touch(&r.lossy(80));
    }
    touch(&"@".repeat(10_000));
    touch(&" ".repeat(10_000));
    touch(&format!("@{} :{} {} {} :{}", "a=b;".repeat(5000), "n!u@h", "PRIVMSG", "#c", "x".repeat(10_000)));
    touch(&format!("{}{}", ":".repeat(5000), "\u{130}".repeat(5000)));
}

#[test]
fn line_parse_never_panics_on_every_prefix_and_mutation_of_valid_lines() {
    let mut r = Rng::new(0xBEEF);
    let mut cases = 0;
    for s in SEEDS {
        for p in byte_prefixes(s) {
            touch(&p);
            cases += 1;
        }
        for _ in 0..400 {
            touch(&r.mutate(s));
            cases += 1;
        }
    }
    assert!(cases > 4000);
}

#[test]
fn line_parts_are_slices_of_the_input() {
    // The parser borrows: every returned piece must lie inside the raw line.
    let mut r = Rng::new(7);
    for _ in 0..3000 {
        let seed = *r.pick(SEEDS);
        let raw = r.mutate(seed);
        let Some(l) = Line::parse(&raw) else { continue };
        let inside = |s: &str| {
            let (a, b) = (s.as_ptr() as usize, s.as_ptr() as usize + s.len());
            let (ra, rb) = (raw.as_ptr() as usize, raw.as_ptr() as usize + raw.len());
            s.is_empty() || (a >= ra && b <= rb)
        };
        assert!(inside(l.command), "{raw:?}");
        assert!(!l.command.contains(' '), "{raw:?}");
        assert!(l.trailing().is_none_or(inside), "{raw:?}");
        assert!(l.middle().all(inside), "{raw:?}");
        assert!(l.nick().is_empty() || inside(l.nick()), "{raw:?}");
    }
}

#[test]
fn tag_escape_round_trips_for_any_value() {
    let mut r = Rng::new(0xE5C);
    for _ in 0..5000 {
        let v = if r.chance(50) { r.text(40) } else { r.lossy(40) };
        let e = escape(&v);
        assert!(!e.contains([' ', ';', '\r', '\n']), "escaped value still has a delimiter: {v:?}");
        assert_eq!(unescape(&e), v, "{v:?}");
    }
    // Unescape of any text never panics and never grows.
    for _ in 0..5000 {
        let v = r.text(40);
        let u = unescape(&v);
        assert!(u.len() <= v.len());
    }
    assert_eq!(unescape(&"\\".repeat(10_001)).len(), 5000);
}

#[test]
fn an_escaped_tag_value_survives_a_whole_line() {
    let mut r = Rng::new(0x71A6);
    for _ in 0..3000 {
        let value = r.text(30);
        let line = format!("@k={};z=1 :n!n@n PRIVMSG #c :hi", escape(&value));
        let l = Line::parse(&line).unwrap();
        assert_eq!(l.tag("k").unwrap(), value, "{value:?}");
        assert_eq!(l.raw_tag("z"), Some("1"));
        assert_eq!(l.trailing(), Some("hi"));
    }
}

#[test]
fn outgoing_privmsg_parses_back_to_the_cleaned_text() {
    let mut r = Rng::new(0x5E4D);
    for _ in 0..3000 {
        let text = if r.chance(70) { r.text(60) } else { r.words(10) };
        let o = Outgoing {
            text: text.clone(),
            reply_to: r.chance(50).then(|| r.text(12)),
            nonce: r.chance(50).then(|| r.text(12)),
            action: r.chance(20),
        };
        let line = privmsg("chan", &o);
        assert!(!line.contains(['\r', '\n']), "{line:?}");
        let l = Line::parse(&line).unwrap_or_else(|| panic!("unparsable {line:?}"));
        assert_eq!(l.command, "PRIVMSG", "{line:?}");
        assert_eq!(l.channel(), Some("chan"), "{line:?}");
        let want: String = text
            .chars()
            .filter_map(|c| match c {
                '\r' | '\n' | '\t' => Some(' '),
                c if c.is_control() => None,
                c => Some(c),
            })
            .collect();
        let want = if o.action { format!("\u{1}ACTION {want}\u{1}") } else { want };
        assert_eq!(l.trailing(), Some(want.as_str()), "{line:?}");
        if let Some(id) = &o.reply_to {
            assert_eq!(l.tag("reply-parent-msg-id").unwrap(), *id, "{line:?}");
        }
        if let Some(n) = &o.nonce {
            assert_eq!(l.tag("client-nonce").unwrap(), *n, "{line:?}");
        }
    }
}
