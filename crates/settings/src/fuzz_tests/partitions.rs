//! V110-F89 Every partition's normalize (the entry the store uses for a loaded, set,
//! patched or imported value): never panics, and normalizing twice equals normalizing once.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Map, Number, Value, json};

use crate::Partition;
use crate::fuzz::Rng;

#[derive(Clone, Debug)]
enum Seg {
    Key(String),
    Idx(usize),
}

fn leaves(v: &Value, path: &mut Vec<Seg>, out: &mut Vec<Vec<Seg>>) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            for (k, c) in m {
                path.push(Seg::Key(k.clone()));
                leaves(c, path, out);
                path.pop();
            }
        }
        Value::Array(a) if !a.is_empty() => {
            for (i, c) in a.iter().enumerate() {
                path.push(Seg::Idx(i));
                leaves(c, path, out);
                path.pop();
            }
        }
        _ => out.push(path.clone()),
    }
}

fn at<'a>(v: &'a mut Value, path: &[Seg]) -> Option<&'a mut Value> {
    let mut cur = v;
    for s in path {
        cur = match (s, cur) {
            (Seg::Key(k), Value::Object(m)) => m.get_mut(k)?,
            (Seg::Idx(i), Value::Array(a)) => a.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn substitutes() -> Vec<Value> {
    vec![
        Value::Null,
        json!(-1),
        json!(0),
        json!(1e308),
        json!(-1e308),
        json!(1.5e-320),
        json!(0.5),
        json!(i64::MAX),
        json!(i64::MIN),
        json!(u64::MAX),
        json!(255),
        json!(256),
        json!(65535),
        json!(65536),
        json!(4294967296u64),
        json!(""),
        json!("x".repeat(100_000)),
        json!("\u{130}\u{212A}\u{1E9E}ß\u{202E}\u{0}\u{200D}👨‍👩‍👧"),
        json!("#zzzzzz"),
        json!("https://example.com/a b"),
        json!("javascript:alert(1)"),
        json!("(?<n>(a+)+)\\1["),
        json!("%s%n{0}${x}"),
        json!([]),
        json!({}),
        json!(true),
        json!(false),
        json!([{}]),
        json!([[]]),
        json!([null]),
        json!([""]),
        json!([1, 2, 3]),
        json!({"a": {}}),
        json!({"": 1}),
        json!({"id": "", "name": "", "enabled": true}),
    ]
}

fn random_json(r: &mut Rng, depth: usize) -> Value {
    match r.below(if depth >= 3 { 7 } else { 9 }) {
        0 => Value::Null,
        1 => Value::Bool(r.chance(50)),
        2 => json!(r.next() as i64),
        3 => json!(r.next()),
        4 => Number::from_f64(f64::from_bits(r.next())).map_or(Value::Null, Value::Number),
        5 => json!(r.text(30)),
        6 => json!(r.number_text()),
        7 => Value::Array((0..r.below(4)).map(|_| random_json(r, depth + 1)).collect()),
        _ => {
            let mut m = Map::new();
            for _ in 0..r.below(4) {
                m.insert(r.text(6), random_json(r, depth + 1));
            }
            Value::Object(m)
        }
    }
}

/// The default with every empty list given one default element, twice over.
fn rich(p: Partition) -> Value {
    let mut v = p.default_value();
    for _ in 0..2 {
        let mut paths = Vec::new();
        leaves(&v, &mut Vec::new(), &mut paths);
        for path in paths {
            let mut trial = v.clone();
            if let Some(slot) = at(&mut trial, &path)
                && slot.as_array().is_some_and(Vec::is_empty)
            {
                *slot = json!([{}]);
                if let Ok(n) = p.normalize(trial) {
                    v = n;
                }
            }
        }
    }
    v
}

/// "panic" or "not idempotent" or `None`.
fn problem(p: Partition, v: Value) -> Option<String> {
    let run = catch_unwind(AssertUnwindSafe(|| {
        let once = p.normalize(v.clone()).ok()?;
        let twice = p.normalize(once.clone());
        match twice {
            Err(e) => Some(format!("normalized value no longer reads: {e}")),
            Ok(t) if t != once => Some("not idempotent".to_owned()),
            Ok(_) => None,
        }
    }));
    match run {
        Err(_) => Some("panic".to_owned()),
        Ok(r) => r,
    }
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 60 { format!("{}...({} bytes)", &s[..s.floor_char_boundary(60)], s.len()) } else { s }
}

#[test]
fn every_leaf_of_every_default_survives_odd_values() {
    let subs = substitutes();
    let mut bad = Vec::new();
    let (mut cases, mut accepted) = (0usize, 0usize);
    for p in Partition::ALL {
        for base in [p.default_value(), rich(p)] {
            let mut paths = Vec::new();
            leaves(&base, &mut Vec::new(), &mut paths);
            for path in &paths {
                for s in &subs {
                    let mut v = base.clone();
                    let Some(slot) = at(&mut v, path) else { continue };
                    *slot = s.clone();
                    cases += 1;
                    accepted += usize::from(p.normalize(v.clone()).is_ok());
                    if let Some(what) = problem(p, v) {
                        bad.push(format!("{p:?} {path:?} <- {}: {what}", short(s)));
                    }
                }
            }
        }
    }
    assert!(cases > 20_000 && accepted > cases / 20, "{cases} cases, {accepted} accepted: the generator is not reaching the schemas");
    assert!(bad.is_empty(), "{} problems, first ones:\n{}", bad.len(), bad.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn randomly_mutated_partitions_survive() {
    let mut r = Rng::new(0x5E77);
    let mut bad = Vec::new();
    for p in Partition::ALL {
        let base = rich(p);
        let mut paths = Vec::new();
        leaves(&base, &mut Vec::new(), &mut paths);
        for _ in 0..250 {
            let mut v = base.clone();
            for _ in 0..1 + r.below(4) {
                let path = r.pick(&paths).clone();
                match r.below(5) {
                    // Replace with random JSON.
                    0..=2 => {
                        if let Some(slot) = at(&mut v, &path) {
                            *slot = random_json(&mut r, 0);
                        }
                    }
                    // Remove the key.
                    3 => {
                        if let Some((Seg::Key(k), parent)) = path.split_last().and_then(|(l, rest)| Some((l, at(&mut v, rest)?)))
                            && let Some(m) = parent.as_object_mut()
                        {
                            m.remove(k);
                        }
                    }
                    // Add unknown keys next to it, and repeat an element.
                    _ => {
                        if let Some(parent) = at(&mut v, &path[..path.len().saturating_sub(1)]) {
                            match parent {
                                Value::Object(m) => {
                                    m.insert(r.text(5), random_json(&mut r, 1));
                                }
                                Value::Array(a) if !a.is_empty() => {
                                    let e = a[r.below(a.len())].clone();
                                    a.push(e);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if let Some(what) = problem(p, v.clone()) {
                bad.push(format!("{p:?}: {what}: {}", short(&v)));
            }
        }
        // Whole-value garbage.
        for _ in 0..50 {
            if let Some(what) = problem(p, random_json(&mut r, 0)) {
                bad.push(format!("{p:?} garbage: {what}"));
            }
        }
    }
    assert!(bad.is_empty(), "{} problems, first ones:\n{}", bad.len(), bad.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn a_wrong_shaped_whole_file_is_an_error_not_a_panic() {
    for p in Partition::ALL {
        for v in [Value::Null, json!(1), json!("x"), json!([]), json!([{}]), json!(true), json!({"a": [[[[[]]]]]})] {
            assert!(catch_unwind(AssertUnwindSafe(|| p.normalize(v.clone()))).is_ok(), "{p:?} {v}");
        }
    }
}
