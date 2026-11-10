//! V110-F89 Rule texts × message texts: mentions, regex flavors, PCRE import, filters,
//! look-alike folding, conditions and the whole rule set.

use std::time::Instant;

use cyex_settings::schema::{
    Highlights, IgnoreRule, LiveWhen, PhraseRule, RegexFlavor, RoleWhen, UserRule, When,
};

use crate::build::{Ctx as BuildCtx, Incoming, Me, build};
use crate::filter::{Facts, Filter};
use crate::fuzz::{Rng, byte_prefixes};
use crate::irc::Line;
use crate::lookalike;
use crate::mentions::{self, MentionSet};
use crate::model::{ChatMessage, ModRole};
use crate::rules::pcre;
use crate::rules::when::{Cond, Ctx};
use crate::rules::{Rules, import_regex};

const PATTERNS: &[&str] = &[
    "cyex", "hello world", r"\bfoo\b", r"(?i)ca+t", r"cheer\d+", r"(a+)+b", r"(a|aa)+$", r"(?=.*x)(?!.*y).*", r"(\w)\1+",
    r"(?<!\w)@?name(?!\w)", r"^[a-z]{3,10}$", r"\p{L}+", r"[\p{Greek}\p{Cyrillic}]+", r"/foo/gi", r"/(?<n>a)\k<n>/u",
    r"(?P<n>a)(?P=n)", r"\Qa.b*\E+", r"a++b", r"(?>a+)b", r"foo\Kbar", r"(?n)(a)(?<x>b)\k<x>", r"[\h\d]", r"a\vb",
    r"(?(?=a)ab|cd)", r"(?|(a)|(b))", r"\X", r"\R", r"(*UTF8)abc", r"\G", r"(?#comment)x", r"(?x) a b # c",
    r"[[:alpha:]]+", r"\x{1F600}", r"\u{130}", r"\N{U+130}", r"[\d-z]", r"[a-\d]", r"x{2,1}", r"x{99999}", r"(((((a)))))",
    "İ", "ß", "K", "\u{1E9E}", "ǅ", "ŉ", "ﬃ", "ΣΑΣ", "\u{130}\u{130}\u{130}",
];

fn phrase(r: &mut Rng, pat: String) -> PhraseRule {
    PhraseRule {
        pattern: pat,
        regex: r.chance(70),
        flavor: *r.pick(&[RegexFlavor::Rust, RegexFlavor::JavaScript, RegexFlavor::Python]),
        case_sensitive: r.chance(30),
        whole_word: r.chance(50),
        lookalikes: r.chance(40),
        words_only: r.chance(50),
        ..PhraseRule::default()
    }
}

fn texts(r: &mut Rng) -> String {
    match r.below(4) {
        0 => r.text(50),
        1 => r.words(10),
        2 => format!("hello {} cyex İstanbul ß foo cheer100 {}", r.text(8), r.text(8)),
        _ => r.lossy(60),
    }
}

fn check_spans(text: &str, spans: &[(usize, usize)]) {
    let n = text.chars().count();
    assert!(spans.len() <= 32, "{text:?}");
    let mut last = 0;
    for &(s, e) in spans {
        assert!(s < e && e <= n && s >= last, "{spans:?} in {text:?} ({n} chars)");
        last = e;
    }
}

#[test]
fn mentions_never_panic_on_any_rule_and_text() {
    let mut r = Rng::new(0x3E47);
    let start = Instant::now();
    for i in 0..120 {
        let one = Instant::now();
        let pat = match i % 4 {
            0 => r.text(20),
            1 => (*r.pick(PATTERNS)).to_owned(),
            2 => {
                let p = *r.pick(PATTERNS);
                r.mutate(p)
            }
            _ => r.lossy(30),
        };
        let list = [phrase(&mut r, pat.clone()), phrase(&mut r, "cyex".into())];
        let _ = mentions::check_all(&list);
        let set = MentionSet::new(&list);
        let mut scratch = String::new();
        for _ in 0..3 {
            let text = texts(&mut r);
            set.each_match(&text, &mut scratch, |c| {
                let s = c.spans(&text);
                check_spans(&text, &s);
            });
            let _ = set.first(&text, &mut scratch);
        }
        // Spans of a words-only rule that matches.
        let mut p = phrase(&mut r, pat);
        p.words_only = true;
        let set = MentionSet::new(&[p]);
        let text = texts(&mut r);
        set.each_match(&text, &mut scratch, |c| check_spans(&text, &c.spans(&text)));
        assert!(one.elapsed().as_millis() < 1500, "slow rule {:?}: {:?}", list[0].pattern, one.elapsed());
    }
    assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
}

#[test]
fn every_prefix_and_mutation_of_known_patterns_compiles_or_errors() {
    let mut r = Rng::new(0x44);
    for pat in PATTERNS {
        for p in byte_prefixes(pat).into_iter().step_by(2) {
            for flavor in [RegexFlavor::Rust, RegexFlavor::JavaScript, RegexFlavor::Python] {
                let _ = mentions::flavor::translate(&p, flavor);
                let m = PhraseRule { pattern: p.clone(), regex: true, flavor, ..PhraseRule::default() };
                let _ = mentions::check(&m);
                let _ = MentionSet::new(&[m]);
            }
            let _ = pcre::to_rust(&p);
            let _ = import_regex(&p);
            let _ = Filter::parse(&p);
        }
        for _ in 0..8 {
            let m = r.mutate(pat);
            let _ = pcre::to_rust(&m);
            let _ = import_regex(&m);
            for flavor in [RegexFlavor::JavaScript, RegexFlavor::Python] {
                let _ = mentions::flavor::translate(&m, flavor);
            }
        }
    }
}

#[test]
fn pathological_patterns_are_bounded_in_time() {
    let start = Instant::now();
    let long_a = "a".repeat(400);
    let patterns = [
        r"(a+)+b", r"(a*)*b", r"(a|a)*b", r"(?=(a+)+b)", r"(a*)*\1b", r"((a*)*)*\2c", r"(?:(?=a)a)*b",
        r"(a?){40}a{40}", r"(?<=(a+)+)b", r"^(a+)+$", r"(x+x+)+y", r"(.*a){20}", r"(?:a|b|ab|ba)*c", r"(\w+\s?)+$",
    ];
    for flavor in [RegexFlavor::Rust, RegexFlavor::JavaScript, RegexFlavor::Python] {
        for p in patterns {
            let m = PhraseRule { pattern: p.into(), regex: true, flavor, whole_word: false, words_only: true, ..PhraseRule::default() };
            let set = MentionSet::new(&[m]);
            let one = Instant::now();
            for text in [&long_a, &format!("{long_a}!"), &"ab".repeat(200), &"a b ".repeat(100)] {
                let _ = set.first(text, &mut String::new()).map(|c| c.spans(text));
            }
            assert!(one.elapsed().as_millis() < 3000, "{p:?} {flavor:?} took {:?}", one.elapsed());
        }
    }
    // The same through the PCRE import and ignore rules (the `regex` crate: linear time).
    for p in patterns {
        if let Ok(rust) = import_regex(p) {
            let m = PhraseRule { pattern: rust, regex: true, ..PhraseRule::default() };
            let set = MentionSet::new(&[m]);
            let one = Instant::now();
            let _ = set.first(&long_a, &mut String::new());
            assert!(one.elapsed().as_millis() < 3000, "import {p:?} took {:?}", one.elapsed());
        }
        let h = Highlights {
            ignores: vec![IgnoreRule { pattern: p.into(), regex: true, ..IgnoreRule::default() }],
            ..Highlights::default()
        };
        let rules = Rules::new(&h);
        let mut m = message("hi");
        m.body = vec![crate::model::Fragment::text(long_a.clone())];
        let one = Instant::now();
        rules.replace(&mut m, &Ctx::default());
        assert!(one.elapsed().as_millis() < 3000, "ignore {p:?} took {:?}", one.elapsed());
    }
    // Huge patterns are refused or compile quickly.
    let huge = [
        "a?".repeat(5000),
        format!("(?:{}){{1000}}", "[a-z]{100}"),
        "x{1000}{1000}".to_owned(),
        "(".repeat(5000) + &")".repeat(5000),
        "\\p{L}".repeat(300),
        "[a-z&&[^b]]".repeat(500),
        "(?i)ß".repeat(5000),
    ];
    for p in &huge {
        let one = Instant::now();
        for flavor in [RegexFlavor::Rust, RegexFlavor::JavaScript, RegexFlavor::Python] {
            let m = PhraseRule { pattern: p.clone(), regex: true, flavor, ..PhraseRule::default() };
            let set = MentionSet::new(&[m]);
            let _ = set.first("hello ".repeat(80).as_str(), &mut String::new());
        }
        let _ = import_regex(p);
        let h = Highlights {
            ignores: vec![IgnoreRule { pattern: p.clone(), regex: true, ..IgnoreRule::default() }],
            users: vec![UserRule { login: p.clone(), regex: true, ..UserRule::default() }],
            ..Highlights::default()
        };
        let _ = Rules::new(&h);
        assert!(one.elapsed().as_secs() < 10, "huge pattern {:.20} took {:?}", p, one.elapsed());
    }
    assert!(start.elapsed().as_secs() < 60, "{:?}", start.elapsed());
}

fn message(text: &str) -> ChatMessage {
    let raw = format!("@badges=subscriber/12;display-name=Bob;id=1;user-id=7;first-msg=1 :bob!bob@bob PRIVMSG #forsen :{text}");
    let mut colors = Default::default();
    let me = Me::new("42", "cyex", "Cyex");
    let mut ctx = BuildCtx::new(Some(&me), &mut colors, 0);
    match build(&Line::parse(&raw).unwrap(), &mut ctx) {
        Some(Incoming::Message(m)) => m,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_whole_rule_set_never_panics() {
    let mut r = Rng::new(0x8081);
    let start = Instant::now();
    for _ in 0..600 {
        let mut h = Highlights::default();
        for _ in 0..1 + r.below(3) {
            let pat = if r.chance(50) { (*r.pick(PATTERNS)).to_owned() } else { r.text(15) };
            let mut p = phrase(&mut r, pat);
            p.when = r.chance(40).then(|| when(&mut r));
            h.phrases.push(p);
        }
        for _ in 0..r.below(3) {
            let pat = if r.chance(50) { (*r.pick(PATTERNS)).to_owned() } else { r.text(15) };
            h.ignores.push(IgnoreRule {
                pattern: pat,
                regex: r.chance(70),
                case_sensitive: r.chance(30),
                block: r.chance(40),
                replace: r.text(10) + if r.chance(50) { r"\1\9\99\" } else { "$1${2}$" },
                when: r.chance(40).then(|| when(&mut r)),
                ..IgnoreRule::default()
            });
        }
        for _ in 0..r.below(2) {
            h.users.push(UserRule { login: r.text(8), regex: r.chance(50), ..UserRule::default() });
        }
        h.lookalikes = r.chance(50);
        let rules = Rules::new(&h);
        for _ in 0..4 {
            let text = texts(&mut r);
            if text.ends_with(['\r', '\n']) || text.starts_with('\u{1}') {
                continue;
            }
            let mut m = message(&text);
            let recent: std::collections::VecDeque<i64> = (0..r.below(5)).map(|_| r.next() as i64 >> 20).collect();
            let ctx = Ctx {
                channel: "forsen",
                live: r.chance(50).then(|| r.chance(50)),
                category: r.chance(50).then_some("just chatting"),
                role: r.chance(30).then_some(ModRole::Moderator),
                vip: r.chance(30),
                first: r.chance(50),
                ts: [0, 1, i64::MAX, i64::MIN / 2][r.below(4)],
                recent: r.chance(70).then_some(&recent),
            };
            let mut routes = Vec::new();
            let _ = rules.hidden(&m, &ctx, &mut routes);
            rules.replace(&mut m, &ctx);
            let _ = rules.highlight(&mut m, &ctx);
        }
    }
    assert!(start.elapsed().as_secs() < 60, "{:?}", start.elapsed());
}

fn when(r: &mut Rng) -> When {
    When {
        channels: (0..r.below(3)).map(|_| r.text(5)).collect(),
        not_channels: r.chance(50),
        categories: (0..r.below(3)).map(|_| r.text(5)).collect(),
        live: r.chance(50).then(|| *r.pick(&[LiveWhen::Live, LiveWhen::Offline])),
        role: r.chance(50).then(|| *r.pick(&[RoleWhen::Vip, RoleWhen::Mod, RoleWhen::Broadcaster])),
        burst: [0, 1, 2, 255][r.below(4)],
        burst_secs: [0, 1, 65535][r.below(3)],
        first_only: r.chance(50),
    }
}

fn conditions_at(stamps: &[i64]) {
    let mut r = Rng::new(0xC0);
    for _ in 0..3000 {
        let w = when(&mut r);
        let Some(c) = Cond::new(Some(&w)) else { continue };
        let recent: std::collections::VecDeque<i64> = [-1, 0, 1, i64::MAX / 4].into_iter().collect();
        for &ts in stamps {
            let ctx = Ctx { ts, recent: Some(&recent), channel: "x", ..Ctx::default() };
            let _ = c.holds(&ctx);
        }
    }
}

#[test]
fn conditions_never_panic_on_odd_message_times() {
    conditions_at(&[-1, 0, 1, i64::MAX / 4, i64::MIN / 4]);
}

#[test]
fn conditions_with_the_smallest_message_time() {
    conditions_at(&[i64::MIN, i64::MIN + 1000]);
}

#[test]
fn lookalike_folding_keeps_length_and_ascii() {
    let mut r = Rng::new(0xF01D);
    for _ in 0..5000 {
        let t = texts(&mut r);
        let f = lookalike::fold(&t);
        assert_eq!(f.chars().count(), t.chars().count(), "{t:?}");
        if t.is_ascii() {
            assert_eq!(f, t);
        }
        // Folding a folded text changes nothing more.
        assert_eq!(lookalike::fold(&f), f, "{t:?}");
        for c in t.chars() {
            if c.is_ascii() {
                assert_eq!(lookalike::fold_char(c), c);
            }
        }
    }
    for c in ['\0', '\u{7F}', '\u{80}', '\u{10FFFF}', '\u{D7FF}', '\u{E000}'] {
        let _ = lookalike::fold_char(c);
    }
    // Every scalar value folds without a panic.
    for cp in 0..=0x10FFFFu32 {
        if let Some(c) = char::from_u32(cp) {
            let _ = lookalike::fold_char(c);
        }
    }
}

const FILTERS: &[&str] = &[
    r#"author.subbed && author.sub_length >= 12"#,
    r#"message.content contains "hello""#,
    r#"author.badges contains "VIP" && channel.name == "FORSEN""#,
    r#"message.content match r"cheer\d+""#,
    r#"(message.content match {ri"(world)", 1}) == "world""#,
    "bits.amount == 100 && flags.cheer_message && !flags.reply",
    r#"author.name startswith "x""#,
    r#""a" + 1 == "a1""#,
    "1 + 2 * 3 == 9",
    "1 / 0 == 0 && 9223372036854775807 * 2 > 0 && -9223372036854775808 - 1 < 0 && 5 % 0 == 0",
    r#"message.content match r"(a+)+b" || message.content match r"(?=a)(?<!b)\w+""#,
    r#"message.content match {r"(", 1}"#,
    "!(!(!(flags.reply)))",
    "channel.live && (author.mod || author.vip) && !author.subbed",
];

#[test]
fn filters_parse_and_match_without_panicking() {
    let mut r = Rng::new(0xF117);
    let m = message("Hello cheer100 World aab");
    let facts = Facts { msg: &m, channel: "forsen", live: true };
    for f in FILTERS {
        for p in byte_prefixes(f) {
            if let Ok(flt) = Filter::parse(&p) {
                let _ = flt.matches(&facts);
            }
        }
        for _ in 0..300 {
            let t = r.mutate(f);
            if let Ok(flt) = Filter::parse(&t) {
                let _ = flt.matches(&facts);
            }
        }
    }
    for _ in 0..3000 {
        let t = if r.chance(50) { r.text(40) } else { r.lossy(40) };
        if let Ok(flt) = Filter::parse(&t) {
            let _ = flt.matches(&facts);
        }
    }
    // Every name is accepted on its own with a comparison.
    for (name, _) in crate::filter::names() {
        for tail in ["", " == 1", " == \"x\"", " contains \"a\"", " + 1 > 0", " && true"] {
            if let Ok(flt) = Filter::parse(&format!("{name}{tail}")) {
                let _ = flt.matches(&facts);
            }
        }
    }
}

/// `true` when the child process survived the input.
fn survives(test: &str, kind: &str, n: usize) -> bool {
    survives_stage(test, kind, n, "")
}

fn survives_stage(test: &str, kind: &str, n: usize, stage: &str) -> bool {
    std::process::Command::new(std::env::current_exe().unwrap())
        .env("CYEX_DEEP_STAGE", stage)
        .args(["--exact", test, "--nocapture"])
        .env("CYEX_DEEP_CHILD", kind)
        .env("CYEX_DEEP_N", n.to_string())
        .output()
        .unwrap()
        .status
        .success()
}

fn deep_text(kind: &str, n: usize) -> String {
    match kind {
        "filter_paren" => format!("{}flags.reply{}", "(".repeat(n), ")".repeat(n)),
        "filter_not" => format!("{}flags.reply", "!".repeat(n)),
        "filter_and" => format!("{}flags.reply", "flags.reply && ".repeat(n)),
        "filter_plus" => format!("{}1 == 1", "1 + ".repeat(n)),
        "filter_brace" => format!("message.content match {}", "{".repeat(n)),
        "regex_group" => format!("{}a{}", "(".repeat(n), ")".repeat(n)),
        "regex_look" => format!("{}a{}", "(?=".repeat(n), ")".repeat(n)),
        "regex_class" => format!("{}a{}", "[".repeat(n), "]".repeat(n)),
        "regex_alt" => "a|".repeat(n),
        "pcre_cond" => format!("{}a{}", "(?(?=a)".repeat(n), ")".repeat(n)),
        "pcre_atomic" => format!("{}a{}", "(?>".repeat(n), ")".repeat(n)),
        _ => unreachable!(),
    }
}

const DEEP_KINDS: &[&str] = &[
    "filter_paren", "filter_not", "filter_and", "filter_plus", "filter_brace", "regex_group", "regex_look",
    "regex_class", "regex_alt", "pcre_cond", "pcre_atomic",
];

const DEEP_TEST: &str = "fuzz_tests::rules::deep_nesting_child";

/// Runs in a child process (a stack overflow aborts, so it can't be caught). The thread has
/// a tokio worker's 2 MiB stack.
#[test]
fn deep_nesting_child() {
    let (Ok(kind), Ok(n)) = (std::env::var("CYEX_DEEP_CHILD"), std::env::var("CYEX_DEEP_N")) else { return };
    let t = deep_text(&kind, n.parse().unwrap());
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let stage = std::env::var("CYEX_DEEP_STAGE").unwrap_or_default();
            let on = |s: &str| stage.is_empty() || stage == s;
            if on("filter") {
                let _ = Filter::parse(&t);
            }
            if on("pcre") {
                let _ = pcre::to_rust(&t);
            }
            if on("import") {
                let _ = import_regex(&t);
            }
            for (name, flavor) in [("rust", RegexFlavor::Rust), ("js", RegexFlavor::JavaScript), ("py", RegexFlavor::Python)] {
                if on(name) {
                    let m = PhraseRule { pattern: t.clone(), regex: true, flavor, ..PhraseRule::default() };
                    let _ = mentions::check(&m);
                    let _ = MentionSet::new(&[m]).first("aaa", &mut String::new());
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn moderately_deep_nesting_is_fine() {
    let bad: Vec<_> = DEEP_KINDS.iter().filter(|k| !survives(DEEP_TEST, k, 50)).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn deep_nesting_must_error_not_overflow() {
    let mut bad = Vec::new();
    for k in DEEP_KINDS {
        // V110-F89 One depth far past every limit, every stage in one child.
        if !survives_stage(DEEP_TEST, k, 5000, "") {
            bad.push(format!("{k} x 5000"));
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn peek_never_panics() {
    let mut r = Rng::new(0x9EE);
    for s in [
        "@a=b :n!n@n PRIVMSG #c :hi",
        ":n PRIVMSG #c :\u{1}ACTION x\u{1}",
        "PRIVMSG #c :",
        "@x",
        ":",
    ] {
        for p in byte_prefixes(s) {
            let _ = mentions::peek(&p);
        }
        for _ in 0..300 {
            let m = r.mutate(s);
            let _ = mentions::peek(&m);
        }
    }
}
