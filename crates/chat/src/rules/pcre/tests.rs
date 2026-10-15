use super::props::{H, V};
use super::to_rust;
use crate::rules::compile::HighlightMatcher;
use crate::rules::import_regex;

/// (sources, converted, texts it matches, texts it doesn't). `None`: unchanged.
type Row = (
    &'static [&'static str],
    Option<&'static str>,
    &'static [&'static str],
    &'static [&'static str],
);

const ROWS: &[Row] = &[
    (&[r"\Qa.b*\E+"], Some(r"a\.b\*+"), &["a.b**"], &["axb*"]),
    (&[r"\Qa.b"], Some(r"a\.b"), &["a.b"], &["axb"]),
    (&[r"x\Ey"], Some("xy"), &["xy"], &["x"]),
    (&[r"[\Q]-\E]"], Some(r"[\]\-]"), &["]", "-"], &["a"]),
    (&[r"\h+x"], Some("[{H}]+x"), &["\u{A0}x", "\u{180E}x"], &["\nx"]),
    (&[r"[\h\d]"], Some(r"[{H}\d]"), &["\u{3000}", "5"], &["a"]),
    (&[r"[^\H]"], Some("[^[^{H}]]"), &["a b"], &["ab"]),
    (&[r"a\vb"], Some("a[{V}]b"), &["a\u{2028}b"], &["axb"]),
    (&[r"a\Vb"], Some("a[^{V}]b"), &["axb"], &["a\nb"]),
    (&["a++b", "(?>a+)b", "a{1,3}+b"], None, &["aab"], &["b"]),
    (&["^a++a"], None, &[], &["aaa"]),
    (&[r"foo\Kbar"], None, &["foobar"], &["foo"]),
    (
        &[r"(?n)(a)(?<x>b)\k<x>"],
        Some(r"(?:a)(?<x>b)\k<x>"),
        &["abb"],
        &["aba"],
    ),
    (&["(?^i)a"], Some("(?i-msx)a"), &["A"], &["b"]),
    (&["(?xx)[a b]"], Some("(?x)[ab]"), &["b"], &[" "]),
    (&[r"(?aD)\d"], Some("[0-9]"), &["3"], &["\u{663}"]),
    (&[r"(?aW)\w"], Some("[0-9A-Za-z_]"), &["a"], &["\u{e9}"]),
    (&["(?s-i:a.b)"], None, &["a\nb"], &["ab"]),
    (
        &[r"(?<n>a)\k<n>", r"(?'n'a)\k'n'", "(?P<n>a)(?P=n)"],
        None,
        &["aa"],
        &["ab"],
    ),
    (
        &[r"(?<n>a)\k{n}", r"(?<n>a)\g{n}"],
        Some(r"(?<n>a)\k<n>"),
        &["aa"],
        &["ab"],
    ),
    (&[r"a\Rb"], None, &["a\r\nb"], &["ab"]),
    (&[r"\Acy\z"], None, &["cy"], &["xcy"]),
    (&[r"\p{L&}"], Some(r"\p{LC}"), &["a"], &["1"]),
    (&[r"\p{^Lu}"], Some(r"\P{Lu}"), &["1"], &["A"]),
    (&[r"\pL"], None, &["a"], &["1"]),
    (&[r"\p{Greek}"], None, &["\u{3b1}"], &["1"]),
    (&[r"\p{Xan}+"], Some(r"[\p{L}\p{N}]+"), &["a1"], &["!"]),
    (&[r"[\p{Xan}_]"], Some(r"[\p{L}\p{N}_]"), &["_"], &["!"]),
    (&[r"\P{Xan}"], Some(r"[^\p{L}\p{N}]"), &["!"], &["a"]),
    (&[r"\p{Xps}"], Some(r"[\p{Z}\t\n\x0B\f\r]"), &[" "], &["a"]),
    (&[r"\p{Xwd}"], Some(r"\w"), &["a"], &[" "]),
    (
        &[r"(a)\g1", r"(a)\g{1}", r"(a)\g-1"],
        Some(r"(a)\1"),
        &["aa"],
        &["ab"],
    ),
    (&[r"(a)(b)\g{-1}"], Some(r"(a)(b)\2"), &["abb"], &["aba"]),
    (
        &[r"\x41\x4\x{263A}\x"],
        Some(r"\x{41}\x{4}\x{263A}\x{0}"),
        &["A\u{4}\u{263A}\0"],
        &["A"],
    ),
    (
        &[r"\0\012\o{101}\101"],
        Some(r"\x{0}\x{A}\x{41}\x{41}"),
        &["\0\nAA"],
        &["AA"],
    ),
    (&[r"\18"], Some(r"\x{1}8"), &["\u{1}8"], &["18"]),
    (&[r"[\101\8]"], Some(r"[\x{41}8]"), &["A", "8"], &["1"]),
    (&[r"\cA\c["], Some(r"\x{1}\x{1B}"), &["\u{1}\u{1B}"], &["A"]),
    (
        &[r"\N{U+263A}\N"],
        Some(r"\x{263A}\N"),
        &["\u{263A}a"],
        &["\u{263A}\n"],
    ),
    (&["[[:alpha:]]+"], Some(r"[\p{L}]+"), &["\u{e9}"], &["1"]),
    (&["[[:^digit:]]"], Some(r"[[^\p{Nd}]]"), &["a"], &["\u{663}"]),
    (&["[[:xdigit:]]"], Some("[0-9A-Fa-f]"), &["f"], &["g"]),
    (
        &["[[:punct:]]"],
        Some(r"[\p{P}[\p{S}&&\x00-\xFF]]"),
        &["!", "$"],
        &["\u{20ac}", "a"],
    ),
    (&["(?aP)[[:alpha:]]"], Some("[[[:alpha:]]]"), &["a"], &["\u{e9}"]),
    (&["[[:<:]]cy"], Some(r"\b(?=\w)cy"), &["a cy"], &["acy"]),
    (&["cy[[:>:]]"], Some(r"cy\b(?<=\w)"), &["cy a"], &["cya"]),
    (&["[a[b]"], Some(r"[a\[b]"), &["["], &["c"]),
    (&["[a&&b]"], Some(r"[a\&\&b]"), &["&"], &["c"]),
    (&["[]a]"], Some(r"[\]a]"), &["]"], &["b"]),
    (&["[^]a]"], Some(r"[^\]a]"), &["b"], &["]"]),
    (&[r"\<a\>"], Some("<a>"), &["<a>"], &["a"]),
    (&[r"\/"], Some("/"), &["/"], &["a"]),
    (&["a(?#note)b"], Some("ab"), &["ab"], &["a"]),
    (&[r#"a(?C1)b(?C"x)y")c"#], Some("abc"), &["abc"], &["ab"]),
    (&[r"^(\((?1)*\))$"], Some(r"^(\(\g<1>*\))$"), &["(())"], &["(()"]),
    (&["(?<p>a)(?&p)"], Some(r"(?<p>a)\g<p>"), &["aa"], &["a"]),
    (&["(?+1)(a)"], Some(r"\g<+1>(a)"), &["aa"], &["a"]),
    (&["(a)?(?(1)b|c)"], None, &["ab", "c"], &["a"]),
    (
        &["(?(?=a)ab|cd)"],
        Some("(?((?=a))ab|cd)"),
        &["ab", "cd"],
        &["ad"],
    ),
    (&[r"(?(DEFINE)(?<w>\w+))\g<w>"], None, &["hi"], &[" "]),
    (&["a(*FAIL)|b"], None, &["b"], &["a"]),
    (&["(*MARK:x)a", "(*:x)a"], Some("a"), &["a"], &["b"]),
    (
        &["(*UTF)(*UCP)(*CRLF)(*LIMIT_MATCH=10)a"],
        Some("a"),
        &["a"],
        &["b"],
    ),
    (
        &["(*pla:a)a", "(*positive_lookahead:a)a"],
        Some("(?=a)a"),
        &["a"],
        &["b"],
    ),
    (&["(*nlb:x)y"], Some("(?<!x)y"), &["y"], &["xy"]),
    (&["(*atomic:a+)b"], Some("(?>a+)b"), &["ab"], &["b"]),
    (&["(?|a|b)c"], Some("(?:a|b)c"), &["bc"], &["c"]),
    (&["a{,2}b"], Some("a{0,2}b"), &["b"], &["c"]),
    (&["a{ 1 , 2 }"], Some("a{1,2}"), &["aa"], &["b"]),
    (&["a{x}"], Some(r"a\{x}"), &["a{x}"], &["ax"]),
    (&["a{,}"], Some(r"a\{,}"), &["a{,}"], &["a"]),
    (&["(?x) a b # c\\Q"], None, &["ab"], &["a b"]),
];

const ERRORS: &[(&str, &str)] = &[
    (r"\X", r"Uses \X, which isn’t supported here."),
    (r"\C", r"Uses \C, which isn’t supported here."),
    (
        "(*napla:a)",
        "Uses a non-atomic lookaround, which isn’t supported here.",
    ),
    (r"(*sr:\d+)", "Uses a script run, which isn’t supported here."),
    ("(*THEN)", "Uses (*THEN), which isn’t supported here."),
    ("(*SKIP:x)", "Uses (*SKIP:name), which isn’t supported here."),
    ("a(*PRUNE:x)b", "Uses (*PRUNE), which isn’t supported here."),
    ("(*COMMIT)a", "Uses (*COMMIT), which isn’t supported here."),
    ("(*ACCEPT)", "Uses (*ACCEPT), which isn’t supported here."),
    ("(*NOTEMPTY)a", "Uses (*NOTEMPTY), which isn’t supported here."),
    ("(?(R)a)", "Uses a recursion check, which isn’t supported here."),
    (
        "(?(VERSION>=10.4)a)",
        "Uses a version check, which isn’t supported here.",
    ),
    ("(?J)(?<n>a)", "Uses (?J), which isn’t supported here."),
    (
        r"(?|(a)|(b))\1",
        "Uses (?| with numbered groups, which isn’t supported here.",
    ),
    (r"\y", r"\y isn’t a PCRE2 escape."),
    (r"a\", r"Ends with a lone \."),
    (r"(a)\g{-2}", r"\g{-2} points before the first group."),
];

fn fill(s: &str) -> String {
    s.replace("{H}", H).replace("{V}", V)
}

#[test]
fn converts_every_row() {
    for (sources, out, yes, no) in ROWS {
        for src in *sources {
            let want = out.map_or_else(|| (*src).to_owned(), fill);
            assert_eq!(to_rust(src), Ok(want.clone()), "convert {src}");
            assert_eq!(import_regex(src), Ok(want.clone()), "import {src}");
            let re = HighlightMatcher::build(&want, true).unwrap_or_else(|e| panic!("{src} -> {want}: {e}"));
            for t in *yes {
                assert!(re.is_match(t), "{src} -> {want} should match {t:?}");
            }
            for t in *no {
                assert!(!re.is_match(t), "{src} -> {want} should not match {t:?}");
            }
        }
    }
}

#[test]
fn says_what_it_cannot_convert() {
    for (src, why) in ERRORS {
        assert_eq!(to_rust(src), Err((*why).to_owned()), "{src}");
    }
}

#[test]
fn import_checks_the_result_compiles() {
    assert!(import_regex("(abc").is_err());
    assert_eq!(
        import_regex(r"\X"),
        Err(r"Uses \X, which isn’t supported here.".to_owned())
    );
}

// V105-16 review: a pattern from someone's settings file never crashes the import.
#[test]
fn never_panics_on_odd_input() {
    const ALPHABET: &[char] = &[
        '\\', '(', ')', '[', ']', '{', '}', '?', '*', '+', ':', '^', '-', '<', '>', '\'', '&', '#',
        '|', ',', '=', '!', 'a', '1', '0', '8', 'Q', 'E', 'x', 'g', 'k', 'p', 'P', 'c', 'o', 'N',
        'h', 'R', 'C', 'F', 'n', 'U', ' ',
    ];
    let n = ALPHABET.len();
    for len in 1..=3u32 {
        for mut k in 0..n.pow(len) {
            let mut s = String::new();
            for _ in 0..len {
                s.push(ALPHABET[k % n]);
                k /= n;
            }
            let _ = to_rust(&s);
        }
    }
    // Longer ones from a fixed generator, so a failure repeats.
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..20_000 {
        let mut s = String::new();
        for _ in 0..(seed % 12 + 1) {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            s.push(ALPHABET[(seed >> 33) as usize % n]);
        }
        let _ = to_rust(&s);
    }
}
