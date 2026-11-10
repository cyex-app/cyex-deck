//! V110-F89 Settings read from text: share codes, bundles, theme codes, Chatterino files,
//! and settings files on disk.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::fuzz::{Rng, byte_prefixes};
use crate::schema::Highlights;
use crate::testutil::{FakeClock, TempDir, day};
use crate::transfer::Bundle;
use crate::{Partition, Store, chatterino, theme_code};

fn bundle_json(r: &mut Rng) -> String {
    let mut parts = std::collections::BTreeMap::new();
    for p in Partition::ALL {
        if p.is_portable() && r.chance(30) {
            parts.insert(p.key().to_owned(), p.default_value());
        }
    }
    Bundle { format: "cyex-settings".into(), version: 1, exported: "2026/11/10 01:00".into(), partitions: parts }.to_json()
}

#[test]
fn share_codes_and_bundles_never_panic() {
    let mut r = Rng::new(0x5CA);
    let good = {
        let b = Bundle::from_json(&bundle_json(&mut r)).unwrap();
        b.to_share_code()
    };
    for p in byte_prefixes(&good) {
        let _ = Bundle::from_share_code(&p);
    }
    for _ in 0..1500 {
        let _ = Bundle::from_share_code(&r.mutate(&good));
        let _ = Bundle::from_share_code(&format!("cyex1.{}", r.text(40)));
        let _ = Bundle::from_share_code(&r.lossy(60));
        let b = bundle_json(&mut r);
        let _ = Bundle::from_json(&r.mutate(&b));
        let _ = Bundle::from_json(&r.text(60));
    }
    // Huge and degenerate codes.
    for code in [
        String::new(),
        "cyex1.".into(),
        format!("cyex1.{}", "A".repeat(30_000_000)),
        format!("cyex1.{}", "A".repeat(1_000_001)),
        format!("cyex1.{}", "=".repeat(1000)),
        format!("cyex1.{}", "\u{130}".repeat(10_000)),
        " ".repeat(5_000_000),
        format!("cyex1.{}", "_-".repeat(100_000)),
    ] {
        let _ = Bundle::from_share_code(&code);
    }
    let _ = Bundle::from_json(&"[".repeat(100_000));
    let _ = Bundle::from_json(&format!("{}1{}", "[".repeat(100_000), "]".repeat(100_000)));
    let _ = Bundle::from_json("{\"format\":\"cyex-settings\",\"version\":99999999999999999999,\"exported\":\"\",\"partitions\":{}}");
}

#[test]
fn imported_bundles_with_odd_partitions_are_refused_or_normalised() {
    let mut r = Rng::new(0x1B0);
    let dir = TempDir::new("f89-import");
    let clock = Arc::new(FakeClock::at(day(1)));
    let (mut store, _) = Store::open(dir.path(), clock).unwrap();
    for i in 0..300 {
        let mut parts = std::collections::BTreeMap::new();
        for _ in 0..1 + r.below(3) {
            let key = if r.chance(80) { r.pick(&Partition::ALL).key().to_owned() } else { r.text(6) };
            let v = match r.below(4) {
                0 => json!({}),
                1 => json!({"general": {"language": r.text(5)}, "x": [1, 2]}),
                2 => serde_json::from_str(&r.mutate(r#"{"a":[1,2,{"b":null}],"c":"d"}"#)).unwrap_or(json!(null)),
                _ => store.get(Partition::Chat).clone(),
            };
            parts.insert(key, v);
        }
        let b = Bundle { format: "cyex-settings".into(), version: 1, exported: String::new(), partitions: parts };
        let _ = store.import(b);
        // A patch with odd values.
        let p = *r.pick(&Partition::ALL);
        let patch = json!({ r.text(4): r.number_text(), "enabled": null, "nested": {"a": [null]} });
        let _ = store.patch(p, &patch);
        if i % 50 == 0 {
            let _ = store.flush();
        }
    }
}

#[test]
fn theme_codes_and_links_never_panic() {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut r = Rng::new(0x7E3E);
    let theme = json!({"id":"x","name":"T","base":"dark","colors":{"bg":"#101010","text":"#eeeeee","accent":"#145aff"},"wallpaper":null});
    let code = format!("cyextheme1.{}", URL_SAFE_NO_PAD.encode(theme.to_string()));
    for p in byte_prefixes(&code) {
        let _ = theme_code::from_code(&p);
    }
    for _ in 0..1500 {
        let _ = theme_code::from_code(&r.mutate(&code));
        let body = r.mutate(&theme.to_string());
        let _ = theme_code::from_code(&format!("cyextheme1.{}", URL_SAFE_NO_PAD.encode(body)));
        let _ = theme_code::from_code(&format!("cyextheme1.{}", URL_SAFE_NO_PAD.encode(r.lossy(60))));
        let _ = theme_code::parse_link(&r.mutate("https://www.cyex.app/t/k3f9x2ab/?x=1#y"));
        let _ = theme_code::parse_link(&r.text(40));
    }
    for s in [
        "\u{130}".repeat(10_000),
        format!("cyextheme1.{}", "A".repeat(100_000)),
        format!("cyextheme1.{}", URL_SAFE_NO_PAD.encode("{".repeat(7_000))),
        format!("cyextheme1.{}", URL_SAFE_NO_PAD.encode(json!({"colors": {"bg": "x".repeat(5000)}, "wallpaper": {"link": "javascript:1"}}).to_string())),
    ] {
        let _ = theme_code::from_code(&s);
        let _ = theme_code::parse_link(&s);
    }
}

const HIGHLIGHT_FILE: &str = r##"{"accounts":{"u":{"oauthToken":"t"}},"highlighting":{"highlights":[{"pattern":"twitch.tv/","showInMentions":false,"alert":false,"sound":true,"regex":true,"case":false,"soundUrl":"file:///C:/a/b.wav","color":"#ff0f398c"},{"pattern":"a+","regex":true,"soundUrl":"file:///C:/..\\..\\x.exe"}],"users":[{"pattern":"SomeUser"},{"pattern":"^b","regex":true}],"badges":[{"name":"moderator,subscriber/3000","displayName":"x"}]}}"##;

const LAYOUT_FILE: &str = r#"{"windows":[{"type":"main","tabs":[{"title":"t","selected":true,"splits2":{"type":"vertical","flexh":1,"items":[{"type":"split","flexv":-2,"data":{"type":"twitch","name":"P"}},{"type":"horizontal","items":[{"type":"split","data":{"type":"twitch","name":"q"}}]}]}},{"splits":[{"data":{"type":"twitch","name":"old"}}]}]}]}"#;

#[test]
fn chatterino_files_never_panic() {
    let mut r = Rng::new(0xC4A7);
    let conv = |p: &str| -> Result<String, String> {
        if p.len().is_multiple_of(3) { Err("no".into()) } else { Ok(p.to_owned()) }
    };
    for file in [HIGHLIGHT_FILE, LAYOUT_FILE] {
        for p in byte_prefixes(file) {
            let _ = chatterino::parse_highlights(&p, conv);
            let _ = chatterino::parse(&p);
        }
        for _ in 0..800 {
            let m = r.mutate(file);
            if let Ok(found) = chatterino::parse_highlights(&m, conv) {
                let cur = Highlights::default();
                let _ = chatterino::plan_highlights(&found, &cur);
                let mut cur = Highlights::default();
                let _ = chatterino::apply_highlights(found, &mut cur, r.chance(50));
            }
            if let Ok(layout) = chatterino::parse(&m) {
                let _ = layout.channel_count();
                let _ = chatterino::plan(&layout, &HashMap::new());
                let _ = chatterino::names(&layout);
            }
        }
    }
    for _ in 0..1500 {
        let t = r.text(60);
        let _ = chatterino::parse_highlights(&t, conv);
        let _ = chatterino::parse(&t);
    }
    // A very wide / long file.
    let many: Vec<String> = (0..20_000).map(|i| format!(r#"{{"pattern":"w{i}","regex":{}}}"#, i % 2 == 0)).collect();
    let big = format!(r#"{{"highlighting":{{"highlights":[{}]}}}}"#, many.join(","));
    let found = chatterino::parse_highlights(&big, conv);
    if let Ok(f) = found {
        let mut cur = Highlights::default();
        let _ = chatterino::apply_highlights(f, &mut cur, false);
    }
    let deep = format!(r#"{{"windows":[{{"tabs":[{{"splits2":{}"x"{}}}]}}]}}"#, r#"{"type":"vertical","items":["#.repeat(60), "]}".repeat(60));
    let _ = chatterino::parse(&deep);
    let deeper = format!(r#"{{"windows":[{{"tabs":[{{"splits2":{}1{}}}]}}]}}"#, r#"{"type":"vertical","items":["#.repeat(200), "]}".repeat(200));
    let _ = chatterino::parse(&deeper);
}

#[test]
fn settings_files_on_disk_never_stop_the_store_opening() {
    let mut r = Rng::new(0xD15C);
    let dir = TempDir::new("f89-open");
    let clock = Arc::new(FakeClock::at(day(1)));
    // One clean start writes every default file; mutate copies of them.
    {
        let (mut s, _) = Store::open(dir.path(), clock.clone()).unwrap();
        let _ = s.flush();
    }
    let clean: Vec<(Partition, String)> = Partition::ALL
        .iter()
        .map(|p| (*p, std::fs::read_to_string(dir.path().join(p.file_name())).unwrap_or_else(|_| p.default_value().to_string())))
        .collect();
    for round in 0..40 {
        let d = TempDir::new("f89-round");
        for (p, text) in &clean {
            let body: Vec<u8> = match r.below(8) {
                0 => text.as_bytes().to_vec(),
                1 => r.mutate(text).into_bytes(),
                2 => r.mutate(text).into_bytes(),
                3 => text.as_bytes()[..r.below(text.len() + 1)].to_vec(),
                4 => (0..r.below(200)).map(|_| r.next() as u8).collect(),
                5 => format!("\u{feff}{text}").into_bytes(),
                6 => Vec::new(),
                _ => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            };
            std::fs::write(d.path().join(p.file_name()), body).unwrap();
        }
        if round % 3 == 0 {
            std::fs::write(d.path().join("mentions.json"), r.mutate(r#"{"list":[{"pattern":"a","regex":true}],"me":{"enabled":true}}"#)).unwrap();
        }
        let (mut s, _) = Store::open(d.path(), clock.clone()).unwrap();
        let _ = s.flush();
        for p in Partition::ALL {
            let _ = s.get(p);
        }
    }
}
