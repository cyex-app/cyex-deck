//! Docs source (V1051-04 registry, settings section): every partition's defaults and the
//! number limits its schema enforces, in `docs/commands/settings-schema.json`. The limits
//! come from running values through the schema, so a new clamp shows up by itself.
//! `tools/commands/registry.ps1` rewrites the file (`CYEX_REGISTRY_WRITE=1`); the settings
//! section of `docs/commands/registry.json` is built from it.

use serde_json::{Map, Value, json};

use crate::Partition;
use crate::schema::FONTS;

/// Integer probes from low to high; the first one a field's type accepts counts.
const LOW: [i64; 2] = [-1_000_000_000, 0];
const HIGH: [i64; 5] = [1_000_000_000, 4_294_967_295, 65_535, 255, 127];
/// Ranges up to this wide are walked one by one to find a fixed set of steps.
const STEP_SCAN: i64 = 1_000;

fn at<'a>(v: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(v, |v, k| v.get(k))
}

/// What the schema keeps of `value` at `path`, or None when the type refuses it.
fn probe(p: Partition, path: &[String], value: i64) -> Option<f64> {
    let mut doc = p.default_value();
    let mut slot = &mut doc;
    for k in path {
        slot = slot.get_mut(k)?;
    }
    *slot = json!(value);
    at(&p.normalize(doc).ok()?, path)?.as_f64()
}

fn limits(p: Partition, path: &[String]) -> Map<String, Value> {
    let mut out = Map::new();
    let low = LOW.iter().find_map(|&x| probe(p, path, x).map(|r| (x, r)));
    let high = HIGH.iter().find_map(|&x| probe(p, path, x).map(|r| (x, r)));
    let min = low.filter(|(x, r)| *r != *x as f64).map(|(_, r)| r);
    let max = high.filter(|(x, r)| *r < *x as f64).map(|(_, r)| r);
    // f32 fields read back as f64 (0.15 → 0.1500000059…); six places are plenty.
    let num = |f: f64| {
        let f = (f * 1e6).round() / 1e6;
        if f.fract() == 0.0 {
            json!(f as i64)
        } else {
            json!(f)
        }
    };
    if let Some(m) = min {
        out.insert("min".into(), num(m));
    }
    if let Some(m) = max {
        out.insert("max".into(), num(m));
    }
    if let (Some(lo), Some(hi)) = (min, max) {
        let (lo, hi) = (lo.ceil() as i64, hi.floor() as i64);
        if hi > lo && hi - lo <= STEP_SCAN {
            let mut steps: Vec<i64> = (lo..=hi)
                .filter_map(|x| probe(p, path, x))
                .map(|r| r as i64)
                .collect();
            steps.dedup();
            if (steps.len() as i64) < hi - lo + 1 && steps.windows(2).all(|w| w[0] < w[1]) {
                out.insert("values".into(), json!(steps));
            }
        }
    }
    out
}

fn walk(p: Partition, v: &Value, path: &mut Vec<String>, out: &mut Map<String, Value>) {
    match v {
        Value::Object(m) => {
            for (k, inner) in m {
                path.push(k.clone());
                walk(p, inner, path, out);
                path.pop();
            }
        }
        Value::Number(_) => {
            let found = limits(p, path);
            if !found.is_empty() {
                out.insert(format!("{}.{}", p.key(), path.join(".")), Value::Object(found));
            }
        }
        _ => {}
    }
}

/// Rounds every float to six places, as `limits` does.
fn tidy(v: &mut Value) {
    match v {
        Value::Object(m) => m.values_mut().for_each(tidy),
        Value::Array(a) => a.iter_mut().for_each(tidy),
        Value::Number(n) if n.is_f64() => {
            let f = (n.as_f64().unwrap_or_default() * 1e6).round() / 1e6;
            *v = json!(f);
        }
        _ => {}
    }
}

fn render() -> String {
    let mut defaults = Map::new();
    let mut limits = Map::new();
    for p in Partition::ALL {
        let mut d = p.default_value();
        walk(p, &d, &mut Vec::new(), &mut limits);
        tidy(&mut d);
        defaults.insert(p.key().into(), d);
    }
    // Fonts-fixed: strings the schema maps onto its list; empty is the app font.
    let mut fonts: Vec<&str> = vec![""];
    fonts.extend(FONTS);
    limits.insert("general.chatFontFamily".into(), json!({ "values": fonts }));
    let doc = json!({
        "regenerate": "tools/commands/registry.ps1",
        "defaults": defaults,
        "limits": limits,
    });
    let mut out = serde_json::to_string_pretty(&doc).expect("the defaults serialize");
    out.push('\n');
    out
}

#[test]
fn settings_schema_file_is_current() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/commands/settings-schema.json"
    );
    let now = render();
    if std::env::var("CYEX_REGISTRY_WRITE").is_ok_and(|v| v == "1") {
        std::fs::write(path, &now).expect("write docs/commands/settings-schema.json");
        return;
    }
    let file = std::fs::read_to_string(path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        file == now,
        "docs/commands/settings-schema.json is stale: run tools/commands/registry.ps1"
    );
}

#[test]
fn probes_find_clamps_and_steps() {
    let path = |s: &str| s.split('.').map(String::from).collect::<Vec<_>>();
    let zoom = limits(Partition::Appearance, &path("zoom"));
    assert_eq!(zoom.get("values"), Some(&json!([100, 125, 150, 175, 200])));
    let size = limits(Partition::General, &path("chatFontSize"));
    assert_eq!(
        (size.get("min"), size.get("max")),
        (Some(&json!(9)), Some(&json!(32)))
    );
    assert!(!size.contains_key("values"));
}
