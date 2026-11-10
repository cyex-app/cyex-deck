//! V110-F89 7TV / BTTV / FFZ frames and answers on mutated fixtures and generated JSON.
//! Paints become CSS: whatever 7TV sends, the CSS may point only at 7TV's own CDN.

use serde_json::Value;

use crate::badges::art::Prints;
use crate::bttv::socket as bttv_socket;
use crate::fuzz::{Rng, byte_prefixes};
use crate::fuzz_json::{dictionary, tree, tweak};
use crate::seventv::account::{model::parse_user, wearing};
use crate::seventv::events::{self, Event, Frame};
use crate::seventv::paint::Paint;

fn fixtures(sub: &str) -> Vec<(String, String)> {
    let dir = format!("{}/tests/fixtures/{sub}", env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if let Ok(text) = std::fs::read_to_string(e.path()) {
            out.push((e.file_name().to_string_lossy().into_owned(), text));
        }
    }
    out.sort();
    out
}

/// Every url( in the CSS must be the CDN's, in quotes, with nothing that closes the string.
fn check_css(css: &str, what: &str) {
    let mut rest = css;
    while let Some(i) = rest.find("url(") {
        let after = &rest[i + 4..];
        assert!(after.starts_with("\"https://cdn.7tv.app/"), "foreign url in css: {css:?} ({what})");
        let end = after[1..].find('"').unwrap_or_else(|| panic!("unclosed url in {css:?} ({what})"));
        let inner = &after[1..1 + end];
        assert!(!inner.contains(['\\', '(', ')', ';', '<', '>', '\n', '\r', '{', '}']), "{inner:?} ({what})");
        rest = &after[1 + end..];
    }
    let lower = css.to_ascii_lowercase();
    for bad in ["@import", "expression(", "javascript:", "</style", "<script", "behavior:", "-moz-binding"] {
        assert!(!lower.contains(bad), "{bad} in css {css:?} ({what})");
    }
    assert!(!css.contains(['{', '}', '<', '>']), "markup in css {css:?} ({what})");
}

fn check_paint(p: &Paint, what: &str) {
    let s = p.style();
    check_css(&s.css, what);
    if let Some(still) = &s.still {
        check_css(still, what);
    }
}

fn check_text(text: &str, what: &str) {
    if let Frame::Event(e) = events::parse(text)
        && let Event::Paint(p) = *e
    {
        check_paint(&p, what);
    }
    let _ = bttv_socket::parse(text);
    let _ = Prints::from_json(text);
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        check_value(&v, what);
    }
}

/// A value fed to every Value-taking reader, and to the paint reader.
fn check_value(v: &Value, what: &str) {
    if let Ok(p) = serde_json::from_value::<Paint>(v.clone()) {
        check_paint(&p, what);
    }
    let _ = parse_user(v.clone());
    if let Ok(Some(w)) = wearing::parse(v.clone())
        && let Some(p) = &w.paint
    {
        check_css(&p.css, what);
        if let Some(still) = &p.still {
            check_css(still, what);
        }
    }
    for status in [200u16, 401, 403, 429, 500, 0, 65535] {
        let _ = wearing::read(status, Some(u64::MAX), v.to_string().as_bytes());
    }
}

/// Every nested object / array of a value down to a depth, so a fixture's inner user or
/// paint object is fed on its own too.
fn parts(v: &Value, depth: usize, out: &mut Vec<Value>) {
    out.push(v.clone());
    if depth == 0 {
        return;
    }
    match v {
        Value::Object(m) => m.values().for_each(|c| parts(c, depth - 1, out)),
        Value::Array(a) => a.iter().take(4).for_each(|c| parts(c, depth - 1, out)),
        _ => {}
    }
}

#[test]
fn fixtures_and_their_parts_are_fine() {
    for sub in ["7tv", "bttv", "ffz", "badges"] {
        let all = fixtures(sub);
        assert!(!all.is_empty(), "no fixtures in {sub}");
        for (name, text) in all {
            check_text(&text, &name);
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                let mut ps = Vec::new();
                parts(&v, 6, &mut ps);
                for p in ps {
                    check_value(&p, &name);
                }
            }
        }
    }
}

#[test]
fn prefixes_of_fixtures_never_panic() {
    let mut r = Rng::new(0xE07);
    for sub in ["7tv", "bttv", "ffz", "badges"] {
        for (name, text) in fixtures(sub) {
            // Every offset of the small ones, a sample of the large ones.
            if text.len() <= 3000 {
                for p in byte_prefixes(&text) {
                    check_text(&p, &name);
                }
            } else {
                for _ in 0..150 {
                    let cut = r.below(text.len() + 1);
                    check_text(&String::from_utf8_lossy(&text.as_bytes()[..cut]), &name);
                }
            }
        }
    }
}

#[test]
fn mutated_fixtures_never_panic_and_paints_stay_on_the_cdn() {
    let mut r = Rng::new(0xE08);
    let dict = dictionary(env!("CARGO_MANIFEST_DIR"));
    for sub in ["7tv", "bttv", "ffz", "badges"] {
        for (name, text) in fixtures(sub) {
            let Ok(base) = serde_json::from_str::<Value>(&text) else { continue };
            for _ in 0..120 {
                // Value-level tweaks (keeps the JSON valid, so decoding goes deep).
                let mut v = base.clone();
                for _ in 0..1 + r.below(3) {
                    tweak(&mut r, &dict, &mut v);
                }
                check_text(&v.to_string(), &name);
                // Text-level mutation.
                check_text(&r.mutate(&text), &name);
            }
        }
    }
}

#[test]
fn hostile_paint_urls_never_reach_the_css() {
    let urls = [
        "https://evil.example/a.webp",
        "https://cdn.7tv.app.evil.example/a.webp",
        "http://cdn.7tv.app/a.webp",
        "https://cdn.7tv.app/a\"); background:url(https://evil.example/x.png",
        "https://cdn.7tv.app/a\\22 ",
        "https://cdn.7tv.app/a)b",
        "https://cdn.7tv.app/a;b",
        "https://cdn.7tv.app/a\nb",
        "https://cdn.7tv.app/a<b>",
        "javascript:alert(1)",
        "data:image/svg+xml,<svg onload=alert(1)>",
        "//cdn.7tv.app/a.webp",
        "https://cdn.7tv.app@evil.example/a.webp",
        "https://cdn.7tv.app/ok.webp",
    ];
    for u in urls {
        for key in ["image_url", "still_url"] {
            let json = format!(
                r#"{{"id":"x","name":"x","function":"URL","{key}":{},"stops":[],"shadows":[]}}"#,
                serde_json::to_string(u).unwrap()
            );
            let p: Paint = serde_json::from_str(&json).unwrap();
            check_paint(&p, u);
            // The check above is not vacuous: the good address does come through.
            if u.ends_with("/ok.webp") && key == "image_url" {
                assert!(p.style().css.contains("url(\"https://cdn.7tv.app/ok.webp\")"), "{}", p.style().css);
            }
        }
    }
    // Gradient stops and shadows with absurd numbers.
    for body in [
        r#"{"id":"x","name":"x","function":"LINEAR_GRADIENT","angle":1e308,"repeat":true,"stops":[{"at":1e308,"color":-1},{"at":-1e308,"color":9223372036854775807},{"at":0.5,"color":-9223372036854775808}],"shadows":[{"x_offset":1e308,"y_offset":-1e308,"radius":1e-320,"color":-1}],"color":-9223372036854775808}"#,
        r#"{"id":"x","name":"x","function":"RADIAL_GRADIENT","shape":"circle\"; }","stops":[],"at":[1e308,-1e308],"size":[0,0],"canvas_repeat":"no-repeat; background:url(https://evil.example)"}"#,
        r#"{"id":"x","name":"x","gradients":[{"function":"URL","image_url":"https://cdn.7tv.app/a.webp","at":[0.5],"size":[1,2,3]},{"function":"LINEAR_GRADIENT","stops":[{"at":0,"color":0}]}],"shadows":[]}"#,
    ] {
        let p: Paint = serde_json::from_str(body).unwrap();
        check_paint(&p, body);
    }
}

#[test]
fn generated_json_never_panics_the_value_readers() {
    let dict = dictionary(env!("CARGO_MANIFEST_DIR"));
    let mut r = Rng::new(0xE09);
    for _ in 0..3000 {
        let v = tree(&mut r, &dict, 0);
        check_value(&v, "tree");
        check_text(&v.to_string(), "tree");
    }
    for _ in 0..2000 {
        check_text(&r.text(80), "text");
        check_text(&r.lossy(80), "lossy");
    }
}
