//! V110-F89 Test-only JSON generators: random trees whose keys come from the names a crate's
//! own sources mention, and in-place tweaks of a valid sample. Included with
//! `#[path = "../../chat/src/fuzz_json.rs"]` next to `fuzz.rs`.
#![allow(dead_code)]

use serde_json::{Map, Value, json};

use crate::fuzz::Rng;

/// Every `"word"` in the gql sources that looks like a JSON key.
pub fn dictionary(manifest_dir: &str) -> Vec<String> {
    let mut words = std::collections::BTreeSet::new();
    let mut stack = vec![std::path::PathBuf::from(manifest_dir).join("src")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs")
                && let Ok(text) = std::fs::read_to_string(&p)
            {
                for part in text.split('"').skip(1).step_by(2) {
                    let ok = (2..=32).contains(&part.len())
                        && part.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                        && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                    if ok {
                        words.insert(part.to_owned());
                    }
                }
                // serde fields: snake_case names become camelCase keys.
                for line in text.lines() {
                    if let Some(rest) = line.trim().strip_prefix("pub ").or(Some(line.trim()))
                        && let Some((name, ty)) = rest.split_once(": ")
                        && ty.ends_with(',')
                        && name.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                        && !name.is_empty()
                    {
                        let mut camel = String::new();
                        let mut up = false;
                        for c in name.chars() {
                            if c == '_' {
                                up = true;
                            } else if up {
                                camel.push(c.to_ascii_uppercase());
                                up = false;
                            } else {
                                camel.push(c);
                            }
                        }
                        words.insert(camel);
                        words.insert(name.to_owned());
                    }
                }
            }
        }
    }
    words.into_iter().collect()
}

pub fn leaf(r: &mut Rng, dict: &[String]) -> Value {
    match r.below(16) {
        0 => Value::Null,
        1 => json!(true),
        2 => json!(false),
        3 => json!(r.below(100)),
        4 => json!(-(r.below(100) as i64) - 1),
        5 => json!(r.next()),
        6 => json!(r.next() as i64),
        7 => json!(f64::from_bits(r.next())).as_f64().filter(|f| f.is_finite()).map_or(Value::Null, |f| json!(f)),
        8 => json!(["0", "1", "42", "-1", "99999999999999999999", "1e9", "", " "][r.below(8)]),
        9 => json!(["2026-11-10T01:02:03Z", "2026-11-10T01:02:03.456Z", "0000-00-00T00:00:00Z", "9999-12-31T23:59:59Z", "2026-13-40T25:61:61Z", "x"][r.below(6)]),
        10 => json!(*r.pick(dict)),
        11 => json!(r.text(12)),
        12 => json!(["ACTIVE", "EXPIRED", "UPCOMING", "DIRECT_ENTITLEMENT", "BADGE", "LIVE", "VOD", "NONE"][r.below(8)]),
        13 => json!("x".repeat(r.below(5000))),
        14 => json!(1e308),
        _ => json!(i64::MIN),
    }
}

pub fn tree(r: &mut Rng, dict: &[String], depth: usize) -> Value {
    if depth >= 6 || r.chance(20 + 8 * depth as u64) {
        return leaf(r, dict);
    }
    if r.chance(22) {
        return Value::Array((0..r.below(3)).map(|_| tree(r, dict, depth + 1)).collect());
    }
    let mut m = Map::new();
    // Narrower with depth, so a tree stays small.
    for _ in 0..1 + r.below(if depth < 2 { 5 } else { 3 }) {
        let k = if r.chance(92) { r.pick(dict).clone() } else { r.text(6) };
        m.insert(k, tree(r, dict, depth + 1));
    }
    // The wrapper shapes most answers have.
    let mut v = Value::Object(m);
    if r.chance(30) {
        v = json!({ "data": v });
    }
    if r.chance(15) {
        v = json!({ "currentUser": v.clone(), "user": v });
    }
    v
}

/// Mutates a JSON value in place: swaps a node for a random tree or leaf.
pub fn tweak(r: &mut Rng, dict: &[String], v: &mut Value) {
    match v {
        Value::Object(m) if !m.is_empty() && r.chance(70) => {
            let keys: Vec<String> = m.keys().cloned().collect();
            let k = r.pick(&keys).clone();
            if r.chance(15) {
                m.remove(&k);
            } else if let Some(c) = m.get_mut(&k) {
                tweak(r, dict, c);
            }
        }
        Value::Array(a) if !a.is_empty() && r.chance(70) => {
            let i = r.below(a.len());
            if r.chance(15) {
                a.remove(i);
            } else if r.chance(10) {
                let e = a[i].clone();
                a.push(e);
            } else {
                tweak(r, dict, &mut a[i]);
            }
        }
        _ => *v = if r.chance(50) { leaf(r, dict) } else { tree(r, dict, 6) },
    }
}

