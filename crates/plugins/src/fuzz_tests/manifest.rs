//! V110-F89 info.json and the permission checks on generated input.

use serde_json::{Value, json};

use crate::fuzz::{Rng, byte_prefixes};
use crate::manifest::{Field, FieldKind, Manifest, Permission};
use crate::perm::{check_url, check_ws_url, data_name, valid_host_pattern};

const FULL: &str = r#"{
    "$schema": "https://example.com/schema.json",
    "name": "Counter", "description": "Counts words.", "authors": ["someone"], "version": "1.2.0",
    "license": "MIT", "homepage": "https://example.com", "tags": ["fun"],
    "permissions": [{"type": "ReadChat"}, {"type": "Network", "hosts": ["API.example.com", "*.example.org"]}, {"type": "Tabs"}],
    "settings": [
        {"key": "loud", "label": "Loud", "type": "toggle", "default": true},
        {"key": "limit", "label": "Limit", "type": "number", "default": 5, "min": 1, "max": 50},
        {"key": "name", "label": "Name", "type": "text", "default": "x"},
        {"key": "mode", "label": "Mode", "type": "choice", "options": ["a", {"value": "b", "label": "Bee"}], "default": "b"}
    ]
}"#;

fn exercise(m: &Manifest, r: &mut Rng) {
    let _ = m.keys();
    let _ = m.has(&Permission::ReadChat);
    for f in &m.settings {
        for stored in [
            None,
            Some(Value::Null),
            Some(json!(f64::MAX)),
            Some(json!(-1e308)),
            Some(json!(1e-320)),
            Some(json!(9e15)),
            Some(json!(9.1e15)),
            Some(json!(i64::MIN)),
            Some(json!(u64::MAX)),
            Some(json!("a".repeat(5000))),
            Some(json!([1, 2])),
            Some(json!({"a": 1})),
            Some(json!(r.text(10))),
        ] {
            let _ = f.value(stored.as_ref());
        }
    }
}

#[test]
fn manifests_never_panic() {
    let mut r = Rng::new(0x3A1F);
    let m = Manifest::parse(FULL).unwrap();
    exercise(&m, &mut r);
    for p in byte_prefixes(FULL) {
        if let Ok(m) = Manifest::parse(&p) {
            exercise(&m, &mut r);
        }
    }
    for _ in 0..4000 {
        let t = match r.below(3) {
            0 => r.mutate(FULL),
            1 => r.text(80),
            _ => r.lossy(80),
        };
        if let Ok(m) = Manifest::parse(&t) {
            exercise(&m, &mut r);
        }
    }
    // Value-level mutations keep the JSON valid, so the checks run.
    let base: Value = serde_json::from_str(FULL).unwrap();
    for _ in 0..3000 {
        let mut v = base.clone();
        mutate_value(&mut v, &mut r);
        if let Ok(m) = Manifest::parse(&v.to_string()) {
            exercise(&m, &mut r);
        }
    }
}

fn mutate_value(v: &mut Value, r: &mut Rng) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            let keys: Vec<String> = m.keys().cloned().collect();
            let k = r.pick(&keys).clone();
            match r.below(8) {
                0 => {
                    m.remove(&k);
                }
                1 => {
                    m.insert(r.text(4), json!(r.number_text()));
                }
                _ => {
                    if let Some(c) = m.get_mut(&k) {
                        mutate_value(c, r);
                    }
                }
            }
        }
        Value::Array(a) if !a.is_empty() => {
            let i = r.below(a.len());
            match r.below(6) {
                0 => {
                    a.remove(i);
                }
                1 => {
                    let e = a[i].clone();
                    a.push(e);
                }
                _ => mutate_value(&mut a[i], r),
            }
        }
        Value::String(s) => *s = if r.chance(50) { r.text(12) } else { r.mutate(s) },
        Value::Number(_) => *v = [json!(-1), json!(0), json!(1e308), json!(u64::MAX), json!(f64::MIN_POSITIVE), json!(0.5)][r.below(6)].clone(),
        Value::Bool(b) => *b = !*b,
        _ => *v = if r.chance(50) { Value::Null } else { json!([]) },
    }
}

#[test]
fn a_number_field_always_returns_a_number_inside_its_range() {
    let mut r = Rng::new(0x9F);
    for _ in 0..3000 {
        let (min, max) = (r.chance(70).then(|| (r.next() as i64 >> r.below(60)) as f64), r.chance(70).then(|| (r.next() as i64 >> r.below(60)) as f64));
        let f = Field {
            key: "k".into(),
            label: "K".into(),
            hint: None,
            kind: FieldKind::Number { default: (r.next() as i64 >> r.below(60)) as f64, min, max },
        };
        let stored = json!((r.next() as i64 >> r.below(64)) as f64 * if r.chance(50) { 1e-3 } else { 1.0 });
        let out = f.value(Some(&stored));
        let n = out.as_f64().expect("a number");
        // A reversed range (min over max) is the plugin's own mistake: max wins.
        if let (Some(lo), Some(hi)) = (min, max)
            && lo <= hi
        {
            assert!(n >= lo && n <= hi, "{n} not in {lo}..{hi}");
        }
    }
}

#[test]
fn host_patterns_and_data_names_never_panic() {
    let mut r = Rng::new(0x77);
    for _ in 0..6000 {
        let t = if r.chance(50) { r.text(30) } else { r.lossy(30) };
        let _ = valid_host_pattern(&t);
        let _ = data_name(&t);
    }
    let _ = valid_host_pattern(&"a.".repeat(5000));
    let _ = data_name(&"a".repeat(100_000));
    for bad in [".", "..", "a.", "con", "CON", "con.txt", "nul", "COM1", "lpt9.x", "a b", "a/b", "a\\b", "a:b", "a\u{0}", "a.txt.", "..a", "CON ", "com\u{b9}", "aux.", "prn.log", "x:stream"] {
        // Windows device names and traversal must be refused.
        let r = data_name(bad);
        assert!(r.is_err(), "{bad:?} accepted");
    }
}

#[test]
fn urls_never_panic_and_never_reach_this_computer() {
    let mut r = Rng::new(0x555);
    let seeds = [
        "https://api.example.com/x?y=1#z", "http://example.org:8080/", "wss://api.example.com/socket", "ws://[::1]:9/",
        "https://user:pw@example.com/", "http://127.0.0.1/", "http://[::ffff:127.0.0.1]/", "http://2130706433/", "http://0x7f.1/",
        "http://169.254.169.254/latest/meta-data", "http://localhost/", "https://xn--e1afmkfd.xn--p1ai/", "http://\u{FF4C}ocalhost/",
    ];
    for s in seeds {
        for p in byte_prefixes(s) {
            for hosts in [vec![], vec!["*.example.com".to_owned()], vec!["example.org".to_owned()]] {
                let _ = check_url(&p, &hosts);
                let _ = check_ws_url(&p, &hosts);
            }
        }
        for _ in 0..300 {
            let m = r.mutate(s);
            let _ = check_url(&m, &[]);
            let _ = check_ws_url(&m, &[]);
        }
    }
    for _ in 0..3000 {
        let t = r.text(40);
        let _ = check_url(&t, &[]);
        let _ = check_ws_url(&t, &["*.a.com".into()]);
    }
}

#[test]
fn loopback_and_private_forms_are_refused() {
    for url in [
        "http://127.0.0.1/", "http://127.1/", "http://0x7f.0.0.1/", "http://2130706433/", "http://017700000001/", "http://0/",
        "http://[::1]/", "http://[::]/", "http://[::ffff:127.0.0.1]/", "http://[::ffff:7f00:1]/", "http://[0:0:0:0:0:ffff:7f00:1]/",
        "http://[64:ff9b::7f00:1]/", "http://[2002:7f00:1::]/", "http://[fe80::1]/", "http://[fc00::1]/", "http://10.0.0.1/",
        "http://192.168.1.1/", "http://172.16.0.1/", "http://169.254.169.254/", "http://100.64.0.1/", "http://localhost/",
        "http://LOCALHOST/", "http://a.localhost/", "http://127.0.0.1:8080/", "http://user@127.0.0.1/", "http://127.0.0.1./",
        "http://intranet/", "http://\u{FF10}\u{FF58}7f.1/", "http://127%2e0%2e0%2e1/", "http://%31%32%37.0.0.1/",
    ] {
        assert!(check_url(url, &[]).is_err(), "{url} accepted");
        let ws = url.replacen("http", "ws", 1);
        assert!(check_ws_url(&ws, &[]).is_err(), "{ws} accepted");
    }
}

#[test]
fn a_trailing_dot_does_not_slip_past_localhost() {
    for url in ["http://localhost./", "http://LOCALHOST./", "http://a.localhost./", "https://localhost../x"] {
        assert!(check_url(url, &[]).is_err(), "{url} accepted");
    }
}
