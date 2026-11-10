//! V110-F89 Markdown, colors, numbers and versions on generated input.

use std::collections::BTreeMap;

use crate::color::{Rgb, Theme, accent_tokens, contrast, normalize_accent, readable_name};
use crate::fuzz::{Rng, byte_prefixes};
use crate::markdown::{Block, Span, inline, parse};
use crate::number::grouped;
use crate::palette::{KEYS, palette};
use crate::version;

const SEEDS: &[&str] = &[
    "# Title\n\nSome **bold** and *italic* and `code` text.\n> quote\n> more\n- a\n- b\n  cont\n1. x\n2) y\n---\n",
    "[link](https://twitch.tv/x \"title\") ![img](https://static-cdn.jtvnw.net/a.png) ![alt](https://evil.example/a.png)",
    "<br>line<br/>two<br />three\r\nfour",
    "[x](javascript:alert(1)) [y](JAVASCRIPT:1) [z](data:text/html,x) [w](<https://a.b/c>) [v](  https://a.b ) [u](java\tscript:1)",
    "![x](javascript:1) ![](data:image/svg+xml,<svg onload=alert(1)>) ![a](https://twitch.tv@evil.com/x.png) ![b](https://twitch.tv.evil.com/x.png)",
    "https://a.b/c_d_e *a **b ***c*** b** a* _x_ __y__ ___z___ snake_case_word \\* \\_ \\[ \\]",
    "[[nested](https://a.b)](https://c.d) [a](b](https://c.d) [](https://a.b) [a]() [a](()) [a](((((https://a.b))))",
    "1234567890. x\n1234567890) y\n9. z\n    deep\n\t\ttab\n* star\n+ plus\n   - sub",
];

fn all_urls(blocks: &[Block]) -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |spans: &[Span]| {
        for s in spans {
            if let Span::Link { url, .. } | Span::Image { url, .. } = s {
                out.push(url.clone());
            }
        }
    };
    for b in blocks {
        match b {
            Block::Heading { spans, .. } | Block::Paragraph { spans } | Block::Quote { spans } => add(spans),
            Block::List { items, .. } => items.iter().for_each(|i| add(i)),
            Block::Rule => {}
        }
    }
    out
}

fn check_urls(src: &str) {
    let blocks = parse(src);
    for url in all_urls(&blocks) {
        let lower = url.to_ascii_lowercase();
        assert!(
            lower.starts_with("https://") || lower.starts_with("http://"),
            "non-web url {url:?} from {src:?}"
        );
        assert!(!url.chars().any(|c| c.is_whitespace()), "odd url {url:?} from {src:?}");
    }
}

#[test]
fn markdown_never_panics_and_only_links_to_the_web() {
    let mut r = Rng::new(0x3D);
    for s in SEEDS {
        check_urls(s);
        for p in byte_prefixes(s) {
            check_urls(&p);
        }
        for _ in 0..150 {
            check_urls(&r.mutate(s));
        }
    }
    for _ in 0..1000 {
        check_urls(&r.text(80));
        check_urls(&r.words(20));
        check_urls(&r.lossy(100));
    }
    // Deep and long.
    check_urls(&"*".repeat(50_000));
    check_urls(&"[".repeat(20_000));
    check_urls(&format!("{}x{}", "[".repeat(5_000), "](https://a.b)".repeat(5_000)));
    check_urls(&"_a".repeat(20_000));
    check_urls(&"> ".repeat(20_000));
    check_urls(&"- ".repeat(20_000));
    check_urls(&"http://".repeat(10_000));
    check_urls(&"![](".repeat(10_000));
    check_urls(&"\u{130}".repeat(20_000));
    check_urls(&"a\n".repeat(50_000));
    let _ = inline(&"`".repeat(50_000));
}

fn long_lines(n: usize) -> Vec<String> {
    vec![
        "*a ".repeat(n),
        "[a](".repeat(n / 4),
        "**a _b ".repeat(n / 2),
        "`a".repeat(n),
        "![a](https://a.b ".repeat(n / 6),
        "(".repeat(n * 2) + "[a](" + &"(".repeat(n * 2),
    ]
}

#[test]
fn markdown_on_moderately_long_lines_is_fast() {
    let start = std::time::Instant::now();
    for s in long_lines(2_000) {
        let _ = parse(&s);
    }
    assert!(start.elapsed().as_secs() < 5, "{:?}", start.elapsed());
}

#[test]
fn markdown_on_very_long_lines_is_not_quadratic() {
    for s in long_lines(20_000) {
        let one = std::time::Instant::now();
        let _ = parse(&s);
        assert!(one.elapsed().as_secs() < 2, "{} bytes took {:?}", s.len(), one.elapsed());
    }
}

#[test]
fn image_hosts_are_only_twitchs_own() {
    for url in [
        "https://twitch.tv@evil.com/x.png",
        "https://evil.com\\@twitch.tv/x.png",
        "https://evil.com#.twitch.tv/x.png",
        "https://evil.com?.twitch.tv/x.png",
        "https://twitch.tv.evil.com/x.png",
        "https://evil.com/.twitch.tv/x.png",
        "https://eviltwitch.tv/x.png",
        "https://jtvnw.net:443@evil.com/x.png",
        "http://static-cdn.jtvnw.net/x.png",
        "https://\u{FF54}witch.tv/x.png",
        "https://static-cdn.jtvnw.net.evil.com/x.png",
        "HTTPS://evil.com/x.png",
    ] {
        let blocks = parse(&format!("![a]({url})"));
        let imgs = all_images(&blocks);
        let is_twitch = |u: &str| {
            let h = u.trim_start_matches("https://");
            let host = h.split(['/', '\\', '?', '#']).next().unwrap_or("");
            let host = host.rsplit_once(':').map_or(host, |(h, _)| h);
            host == "twitch.tv" || host.ends_with(".twitch.tv") || host.ends_with("jtvnw.net") || host.ends_with("ttvnw.net")
        };
        for i in imgs {
            assert!(is_twitch(&i) && !i.contains('@'), "image kept for {url:?}: {i:?}");
        }
    }
}

fn all_images(blocks: &[Block]) -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |spans: &[Span]| {
        for s in spans {
            if let Span::Image { url, .. } = s {
                out.push(url.clone());
            }
        }
    };
    for b in blocks {
        match b {
            Block::Heading { spans, .. } | Block::Paragraph { spans } | Block::Quote { spans } => add(spans),
            Block::List { items, .. } => items.iter().for_each(|i| add(i)),
            Block::Rule => {}
        }
    }
    out
}

fn is_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[test]
fn colors_never_panic_and_tokens_are_well_formed() {
    let mut r = Rng::new(0xC01);
    for _ in 0..4000 {
        let input = match r.below(4) {
            0 => r.text(10),
            1 => format!("#{}", r.text(7)),
            2 => format!("#{:06x}", r.next() as u32 & 0xFFFFFF),
            _ => r.lossy(12),
        };
        let _ = Rgb::parse(&input);
        let _ = normalize_accent(&input);
        for theme in [Theme::Dark, Theme::Light] {
            let t = accent_tokens(&input, theme);
            for hex in [&t.accent, &t.accent_hover, &t.accent_ink, &t.accent_text, &t.accent_line] {
                assert!(is_hex(hex), "{hex:?} for {input:?}");
            }
            for rgba in [&t.accent_dim, &t.accent_wash, &t.accent_border] {
                assert!(rgba.starts_with("rgba(") && rgba.ends_with(')'), "{rgba:?}");
            }
        }
    }
    // Every 3-digit and a sample of 6-digit colors, with the edge channels.
    for n in 0..4096u32 {
        let c = Rgb::parse(&format!("{:03x}", n)).unwrap();
        for theme in [Theme::Dark, Theme::Light] {
            let n = readable_name(c, theme);
            let [page, _] = theme.backgrounds();
            assert!(contrast(n, page).is_finite());
        }
    }
    for c in [Rgb::new(0, 0, 0), Rgb::new(255, 255, 255), Rgb::new(255, 0, 0)] {
        for t in [0.0, 1.0, -1.0, 2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e308] {
            let _ = c.mix(Rgb::new(9, 9, 9), t);
            let _ = c.rgba(t);
        }
        let _ = contrast(c, c);
    }
    assert_eq!(Rgb::parse("#fff"), Some(Rgb::new(255, 255, 255)));
}

#[test]
fn palettes_with_odd_color_maps_never_panic() {
    let mut r = Rng::new(0x9A1);
    for _ in 0..1500 {
        let mut m = BTreeMap::new();
        for _ in 0..r.below(14) {
            let key = if r.chance(70) { (*r.pick(&KEYS)).to_owned() } else { r.text(5) };
            let v = if r.chance(50) { format!("#{:06x}", r.next() as u32 & 0xFFFFFF) } else { r.text(8) };
            m.insert(key, v);
        }
        for theme in [Theme::Dark, Theme::Light] {
            let p = palette(theme, &m);
            let _ = p.backgrounds();
        }
    }
}

#[test]
fn numbers_group_by_dots_for_every_edge() {
    let mut r = Rng::new(7);
    let mut cases: Vec<u64> = vec![0, 1, 9, 10, 99, 100, 999, 1000, 1001, 999_999, 1_000_000, u64::MAX, u64::MAX - 1, 1 << 63];
    cases.extend((0..5000).map(|_| r.next() >> r.below(64)));
    for n in cases {
        let g = grouped(n);
        assert_eq!(g.replace('.', ""), n.to_string());
        assert!(!g.starts_with('.') && !g.ends_with('.'));
        let groups: Vec<&str> = g.split('.').collect();
        assert!(groups[0].len() <= 3 && groups[1..].iter().all(|s| s.len() == 3), "{g}");
    }
}

#[test]
fn versions_parse_or_refuse_and_round_trip() {
    let mut r = Rng::new(0x7E6);
    let seeds = ["v0.1.5", "0.1.04", "0.2.0-beta.2", "1.0.0+build", "1.2.3-rc.1.x-y", "99999999999999999999.0.0", "-1.0.0", "1.0.0-", "1.0.0-.", "1..0"];
    for s in seeds {
        for p in byte_prefixes(s) {
            let _ = version::parse(&p);
        }
        for _ in 0..300 {
            let m = r.mutate(s);
            if let Some(v) = version::parse(&m) {
                let shown = v.to_string();
                let again = version::parse(&shown).unwrap_or_else(|| panic!("{m:?} shows as {shown:?}, which does not parse"));
                assert_eq!(again, v, "{m:?} -> {shown:?}");
                assert_eq!(v.cmp(&again), std::cmp::Ordering::Equal);
            }
        }
    }
    for _ in 0..3000 {
        let a = version::parse(&r.text(12));
        let b = version::parse(&r.text(12));
        if let (Some(a), Some(b)) = (a, b) {
            assert_eq!(a.cmp(&b), b.cmp(&a).reverse());
        }
    }
    let _ = version::current();
}
