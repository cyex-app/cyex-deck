//! V110-F89 Mod commands, durations, /nuke and refusal texts on generated input.

use crate::command::{self, NAMES};
use crate::duration;
use crate::fuzz::{Rng, byte_prefixes};
use crate::mass;
use crate::nuke::{Line, Nuke};
use crate::said;

const ARGS: &[&str] = &[
    "bob", "@Bob_1, 10m spam", "bob 1h30m reason here", "bob 99999999999999999999", "bob 10min", "bob 0", "bob 2w", "bob 1mo",
    "12345678-1234-1234-1234-123456789abc", "30", "10 5", "off", "red", "blue", "#chan", "bob 4294967296", "bob -5", "bob 18446744073709551615s",
    "hello world", "\"quoted phrase\" 60s 10m", "/regex+/ 5m ban", "", " ", "bob  \t spam",
];

fn all_names() -> Vec<&'static str> {
    NAMES.to_vec()
}

#[test]
fn mod_commands_never_panic() {
    let mut r = Rng::new(0xAD0);
    let names = all_names();
    for n in &names {
        for a in ARGS {
            let _ = command::parse(n, a);
            for p in byte_prefixes(a) {
                let _ = command::parse(n, &p);
            }
        }
        let _ = command::need(n);
    }
    for _ in 0..6000 {
        let n = if r.chance(70) { *r.pick(&names) } else { "x" };
        let a = match r.below(3) {
            0 => r.text(40),
            1 => {
                let s = *r.pick(ARGS);
                r.mutate(s)
            }
            _ => r.lossy(40),
        };
        let _ = command::parse(n, &a);
        let _ = command::login(&a);
        let _ = duration::parse(&a, r.next() % 100_000);
        let _ = duration::looks_like(&a);
        let _ = said::code(&a);
        let _ = Nuke::parse(&a);
    }
}

#[test]
fn durations_are_checked_not_wrapped() {
    for t in ["18446744073709551615", "18446744073709551615s", "18446744073709551615mo", "307445734561825861m", "1w1w1w", "0s", "0", "00000001", "1h1h", "1 h", "-1", "+1", "１０m", "10M", "10ms", "1mo1mo"] {
        for bare in [0u64, 1, 60, u64::MAX] {
            let _ = duration::parse(t, bare);
        }
    }
    assert_eq!(duration::parse("1h30m", 1), Some(5400));
    assert_eq!(duration::parse("18446744073709551615", 60), None, "bare number times unit overflows");
    assert_eq!(duration::parse("18446744073709551615mo", 1), None);
    let mut r = Rng::new(5);
    for _ in 0..5000 {
        let t = format!("{}{}", r.number_text(), ["s", "m", "h", "d", "w", "mo", ""][r.below(7)]);
        let _ = duration::parse(&t, 1);
    }
}

#[test]
fn nuke_scans_do_not_wrap_on_huge_windows_or_times() {
    let lines = [
        Line { user_id: "1", login: "a", display: "A", text: "buy followers", ts_ms: 1_000 },
        Line { user_id: "2", login: "b", display: "", text: "BUY followers", ts_ms: i64::MIN },
        Line { user_id: "3", login: "c", display: "C", text: "hello", ts_ms: i64::MAX },
        Line { user_id: "", login: "d", display: "D", text: "buy", ts_ms: 0 },
    ];
    for args in ["buy", "buy 1", "buy 60s 10m", "\"buy f\" 1h", "/buy.*/ 5m ban"] {
        if let Ok(n) = Nuke::parse(args) {
            for now in [0, 1_000, 1_000_000, i64::MAX / 2000] {
                let _ = n.scan(&lines, now);
            }
        }
    }
}

#[test]
fn nuke_with_a_huge_window_still_finds_old_messages() {
    let lines = [Line { user_id: "1", login: "a", display: "A", text: "buy followers", ts_ms: 1_000 }];
    for window in ["18446744073709551615", "9223372036854775807", "9223372036854775", "999999999999999m"] {
        let n = Nuke::parse(&format!("buy {window}")).unwrap();
        assert_eq!(n.scan(&lines, 5_000).len(), 1, "window {window}");
    }
}

#[test]
fn nuke_phrases_and_regexes_never_panic_or_hang() {
    let mut r = Rng::new(0x4B3);
    let start = std::time::Instant::now();
    let texts = ["buy now", "BUY NOW", "\u{130}\u{130}", &"a".repeat(2000), "ß", "\u{212A}"];
    for _ in 0..3000 {
        let phrase = match r.below(4) {
            0 => format!("/{}/", r.text(10)),
            1 => format!("\"{}\"", r.text(10)),
            2 => r.text(10),
            _ => format!("/{}/ {}", ["(a+)+$", "(?i)ß+", "\\p{L}{100}", "(.*)*x", "[a-z&&[^b]]+", "\\b\\w+\\b"][r.below(6)], r.text(4)),
        };
        if let Ok(n) = Nuke::parse(&phrase) {
            let lines: Vec<Line<'_>> = texts
                .iter()
                .enumerate()
                .map(|(i, t)| Line { user_id: ["1", "2", "3", "4", "5", "6"][i], login: "x", display: "", text: t, ts_ms: i as i64 })
                .collect();
            let _ = n.scan(&lines, 10);
        }
    }
    let big = format!("/{}/", "a?".repeat(20_000));
    let _ = Nuke::parse(&big);
    let _ = Nuke::parse(&format!("/{}/", "(".repeat(20_000)));
    assert!(start.elapsed().as_secs() < 20, "{:?}", start.elapsed());
}

#[test]
fn mass_helpers_never_panic() {
    let mut r = Rng::new(0x3A55);
    for n in 0..70u32 {
        for hint in [None, Some(i64::MIN), Some(-1), Some(0), Some(i64::MAX)] {
            let _ = mass::backoff_ms(n, hint);
        }
    }
    let _ = mass::backoff_ms(u32::MAX, Some(i64::MAX));
    for _ in 0..2000 {
        let list: Vec<String> = (0..r.below(10)).map(|_| r.text(12)).collect();
        let _ = mass::unique(&list);
        let _ = mass::helix_step(r.below(700) as u16, &r.text(40), r.chance(50));
        let _ = mass::gql_step(r.chance(50).then(|| r.text(10)).as_deref(), r.chance(50), r.chance(50));
    }
    let _ = mass::check_size(usize::MAX);
}
