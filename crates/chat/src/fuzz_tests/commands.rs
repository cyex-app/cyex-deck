//! V110-F89 Custom command templates: parse and run on generated templates and arguments.

use std::collections::HashMap;
use std::sync::Arc;

use crate::commands::chain::{parse_timer, split_chain, split_foreach};
use crate::commands::menu::parse_menu;
use crate::commands::taint;
use crate::commands::{CommandList, Host, Items, Params, Var, parse, run};
use crate::fuzz::{Rng, byte_prefixes};

const SEEDS: &[&str] = &[
    "/me slaps $$1 around with a $if(2,$2-,trout)",
    "in $(chan): $(nope)$3",
    "$join(1-,/)",
    "$ifeq(1,cake,yummy,meh)",
    "$switch(1,25:50,50:18,25)",
    "$upper(1) $lower($1-)",
    "{paint}-{badge}",
    "$ifeq(1,$(chan),,$$1: )",
    "$(_m,forsen)FAQ",
    "$(_loop)",
    "$quote(a\\\"b)",
    "$urlencode($1-)",
    "$trim( x )",
    "$rand(a,b,c)",
    "$randnum(5,5)",
    "$randnum(-9223372036854775808,9223372036854775807)",
    "$randnum(9223372036854775807,-9223372036854775808)",
    "$request(https://x,trim)",
    "$request(https://x,k: {\"k\":1})",
    "$cs(a|b) $fs(a>b)",
    "/chain /echo $1 | /echo b",
    "$calc(2^3) $calc( (3+4\\) / 2) $calc(sqrt(-1)) $calc(1/0) $calc(2^99999) $calc(((((1)))))",
    "$calc(9223372036854775807*9223372036854775807) $calc(1%0) $calc(--1) $calc(tan(1e308))",
    "$datetime(yyyy/MM/dd HH:mm:ss,UTC,en,1552489908000)",
    "$datetime(EEEE d MMMM yyyy G a hh zzzz 'at' ZZZ XXX,Europe/Berlin,de,-62167219200000)",
    "$datetime(long,America/New_York,,9223372036854775807)",
    "$datetime(short,+99:99,xx,-9223372036854775808)",
    "$replace(A b and another B,b,bee) $replace(aaa,a,b,regFirst) $replace(abc,(?<x>b),[$1],reg)",
    "$replace($1-,(a+)+$,x,regFunc) $replace(a,(,b,reg)",
    "$round(3.14159,2) $round(2.5,0,up,3) $round(1e308,99) $round(NaN) $round(-0.0,5,half,99)",
    "$sort(b a C,cs, ) $sort($1-)",
    "$json($get(var,j),$j(a->b[0][join:'/']))",
    "$j(books[filter:author=a][collect:title][unique][size]) $j(x,,each:$(key): $j(y),$j([join]))",
    "$is(subs) $get(var,key) $input(Name?,x)",
    "$if($(chan),yes,no) $if(1) $if() $if(,,)",
    "$<$(((((((",
    "$\"literal $1\" \\$1 \\\\ \\",
    "$$$$1 $$(chan) $(_a,$(_b,$(_c)))",
    "$(10-) $(0) $(99999999999999999999) $1- $0",
    "{paint.id} {combo:a b} {badge} {cosmetic:} {",
];

#[derive(Default)]
struct H {
    vars: HashMap<String, String>,
    customs: HashMap<String, Arc<Items>>,
    rnd: u64,
    now: i64,
    off: i32,
    requests: usize,
}

impl Host for H {
    fn custom(&self, name: &str) -> Option<Arc<Items>> {
        self.customs.get(name).cloned()
    }
    fn get(&self, name: &str, key: Option<&str>) -> Option<String> {
        (name == "var").then(|| self.vars.get(key?).cloned()).flatten()
    }
    fn request(&mut self, url: &str, headers: &[(String, String)]) -> Result<String, String> {
        self.requests += 1;
        if self.requests.is_multiple_of(3) {
            return Err(url.to_owned());
        }
        Ok(format!("{{\"a\":{{\"b\":[1,2,{}]}},\"h\":{}}}", headers.len(), "9".repeat(self.requests % 40)))
    }
    fn is(&self, m: &str) -> bool {
        m.len().is_multiple_of(2)
    }
    fn input(&mut self, m: &str, i: &str) -> Option<String> {
        (!m.len().is_multiple_of(3)).then(|| i.to_owned())
    }
    fn cosmetic(&mut self, v: &Var) -> String {
        format!("{v:?}")
    }
    fn random(&mut self, below: u64) -> u64 {
        self.rnd = self.rnd.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rnd % below.max(1)
    }
    fn now_ms(&self) -> i64 {
        self.now
    }
    fn utc_offset_secs(&self) -> i32 {
        self.off
    }
}

fn host(r: &mut Rng) -> H {
    let mut h = H { rnd: r.next(), ..Default::default() };
    h.now = match r.below(6) {
        0 => i64::MAX,
        1 => i64::MIN,
        2 => 0,
        3 => -1,
        4 => r.next() as i64,
        _ => 1_552_489_908_000,
    };
    h.off = match r.below(5) {
        0 => i32::MAX,
        1 => i32::MIN,
        2 => 0,
        3 => -43_200,
        _ => 3_600,
    };
    h.vars.insert("j".into(), "{\"a\":{\"b\":[1,2,3]},\"books\":[{\"author\":\"a\",\"title\":\"t\"}]}".into());
    h.vars.insert("key".into(), r.text(10));
    for (n, body) in [("_a", "$(_b) $1"), ("_b", "$(_a)"), ("_loop", "$(_loop)"), ("_c", "x $2-")] {
        h.customs.insert(n.into(), Arc::new(parse(body).unwrap()));
    }
    h
}

fn go(template: &str, args: &str, r: &mut Rng) {
    let Ok(items) = parse(template) else { return };
    let mut p = Params::new(args);
    p.set("chan", "forsen");
    p.set("msg-id", r.text(5));
    let mut h = host(r);
    let _ = run(&items, &p, &mut h);
    // The same body as a rule's action (marked values).
    let _ = taint::clean(&taint::mark(&run(&items, &p, &mut h).unwrap_or_default()));
}

#[test]
fn parse_and_run_never_panic_on_random_templates_and_arguments() {
    let mut r = Rng::new(0xC0DE);
    let start = std::time::Instant::now();
    for i in 0..4000 {
        let t = match i % 3 {
            0 => r.text(60),
            1 => format!("${}", r.text(30)),
            _ => r.lossy(80),
        };
        let a = r.words(8);
        go(&t, &a, &mut r);
    }
    assert!(start.elapsed().as_secs() < 20, "{:?}", start.elapsed());
}

#[test]
fn every_prefix_of_every_seed_parses_or_errors() {
    let mut r = Rng::new(0xFACE);
    let mut cases = 0;
    for s in SEEDS {
        for p in byte_prefixes(s) {
            go(&p, "a b c", &mut r);
            cases += 1;
        }
    }
    assert!(cases > 1000);
}

#[test]
fn mutated_seeds_never_panic() {
    let mut r = Rng::new(0xD00D);
    let start = std::time::Instant::now();
    for _ in 0..40 {
        for s in SEEDS {
            let m = r.mutate(s);
            let args = if r.chance(50) { r.words(6) } else { "x ".repeat(r.below(20)) };
            go(&m, &args, &mut r);
        }
    }
    assert!(start.elapsed().as_secs() < 20, "{:?}", start.elapsed());
}

#[test]
fn unterminated_and_stray_syntax() {
    let mut r = Rng::new(1);
    for t in [
        "$", "$(", "$((", "$(chan", "$if(", "$if(1,", "$if(1,2,3", "$$", "$$(", "$1-", "$\"", "$\"abc", "\\", "$(_", "$(_a,",
        "{", "{paint", "{paint:", "}", "$j(", "$j([", "$j(a[", "$j(a[filter:", "$calc(", "$calc(1+", "$calc((", "$calc()",
        "$datetime(", "$datetime(,,,", "$datetime(a,b,c,d,e", "$replace(", "$replace(a,", "$replace(a,b,c,d,e,f,g)",
        "$request(", "$request(,", "$json(", "$json(,", "$switch(", "$switch(1,", "$switch(1,a:", "$ifeq(1,", "$rand()", "$randnum()",
        "$randnum(1)", "$randnum(a,b)", "$round()", "$round(1,", "$round(1,-1)", "$round(1,99999999999999999999)", "$sort()",
    ] {
        go(t, "", &mut r);
        go(t, "a b", &mut r);
    }
}

#[test]
fn deep_nesting_errors_or_finishes_without_overflowing_the_stack() {
    // A stack overflow aborts the process, so ask a child process; the thread has the size of
    // a tokio worker's stack (2 MiB).
    if let Ok(kind) = std::env::var("CYEX_DEEP_CHILD") {
        let n: usize = std::env::var("CYEX_DEEP_N").unwrap().parse().unwrap();
        let t = match kind.as_str() {
            "paren" => format!("{}x{}", "$(".repeat(n), ")".repeat(n)),
            "if" => format!("{}x{}", "$if(1,".repeat(n), ")".repeat(n)),
            "ifnoclose" => "$if(1,".repeat(n),
            "upper" => format!("{}x{}", "$upper(".repeat(n), ")".repeat(n)),
            "calc" => format!("$calc({}1{})", "(".repeat(n), ")".repeat(n)),
            "calcneg" => format!("$calc({}1)", "-".repeat(n)),
            "calcfn" => format!("$calc({}1{})", "sqrt(".repeat(n), ")".repeat(n)),
            "custom" => format!("{}x{}", "$(_a,".repeat(n), ")".repeat(n)),
            "json" => format!("$j(a{})", "[".repeat(n)),
            "brace" => "{".repeat(n),
            _ => unreachable!(),
        };
        std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || {
                let mut r = Rng::new(3);
                go(&t, "a", &mut r);
            })
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let bad = deep_failures(&[100]);
    assert!(bad.is_empty(), "{bad:?}");
}

/// Kinds and depths that kill the process, as "kind x n".
fn deep_failures(depths: &[usize]) -> Vec<String> {
    let exe = std::env::current_exe().unwrap();
    let mut bad = Vec::new();
    for kind in ["paren", "if", "ifnoclose", "upper", "calc", "calcneg", "calcfn", "custom", "json", "brace"] {
        for &n in depths {
            let out = std::process::Command::new(&exe)
                .args(["--exact", "fuzz_tests::commands::deep_nesting_errors_or_finishes_without_overflowing_the_stack", "--nocapture"])
                .env("CYEX_DEEP_CHILD", kind)
                .env("CYEX_DEEP_N", n.to_string())
                .output()
                .unwrap();
            if !out.status.success() {
                bad.push(format!("{kind} x {n}"));
            }
        }
    }
    bad
}

#[test]
fn deep_nesting_must_error_not_overflow() {
    let bad = deep_failures(&[300, 1000, 5000]);
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn huge_repeat_counts_and_fields_stay_fast() {
    let mut r = Rng::new(5);
    let big = "a".repeat(10_000);
    let start = std::time::Instant::now();
    for t in [
        format!("$replace({big},a,bb) $replace({big},a*,x,reg) $replace({big},(a|aa)+$,x,reg)"),
        format!("$sort({big}) $upper({big}) $lower({big}) $urlencode({big}) $trim({big})"),
        format!("$join(1-,{big})"),
        "$round(1,4294967295) $round(1,18446744073709551615) $round(1e308,1000000)".to_owned(),
        "$randnum(0,18446744073709551615)".to_owned(),
        format!("$calc({})", "9*".repeat(5000) + "9"),
        format!("$datetime({},UTC)", "y".repeat(10_000)),
        format!("$datetime({},UTC)", "'".repeat(10_001)),
        format!("$datetime({big},{big},{big},{big})"),
        format!("$replace(x,{},y,reg)", "a?".repeat(2000) + &"a".repeat(2000)),
        format!("$replace({big},{},y,reg)", "(a*)*b"),
        format!("$replace({big},{},y,regFunc)", "((a*)*)*b"),
        format!("$replace({big},{{1000}}{{1000}},y,reg)"),
        format!("$replace({big},[a-z]{{1000}}[a-z]{{1000}},y,reg)"),
        "$replace(x,(((((((((((((((((((((((((((((((((a))))))))))))))))))))))))))))))),y,reg)".to_owned(),
    ] {
        go(&t, &format!("{big} {big}"), &mut r);
    }
    assert!(start.elapsed().as_secs() < 10, "{:?}", start.elapsed());
}

#[test]
fn chain_foreach_timer_menu_parsers_never_panic() {
    let mut r = Rng::new(0xA11);
    let seeds = [
        "/echo a | /echo b || c |||",
        "a>b>>c>",
        "10s /echo hi",
        "id=x keep silent overwrite every 5m /say x",
        "at 25:61 /say",
        "at 23:59:59 /say",
        "in 99999999999999999999d /say",
        "every 1h30m /say",
        "2026/12/31 23:59 /say",
        "-5s x",
        "0 x",
        "1e9 x",
        "10 /say x",
    ];
    for s in seeds {
        for p in byte_prefixes(s) {
            let _ = (split_chain(&p), split_foreach(&p), parse_timer(&p));
        }
    }
    for _ in 0..5000 {
        let t = if r.chance(50) { r.text(40) } else { let s = *r.pick(&seeds); r.mutate(s) };
        let _ = (split_chain(&t), split_foreach(&t), parse_timer(&t));
        let n = r.below(5);
        let lines: Vec<String> = (0..n).map(|_| r.mutate("name|label|/cmd $1|#c")).collect();
        let _ = parse_menu(&lines);
        let n = r.below(6);
        let l: Vec<String> = (0..n).map(|_| { let s = *r.pick(&["/hi $1", "_x $(chan)", "# c", "/a#chan $1", "bad", "/dup a"]); r.mutate(s) }).collect();
        let list = CommandList::new(&l);
        let _ = (list.command("hi", "x"), list.names("x"), list.replacement("_x", "c"));
    }
    let _ = parse_timer(&format!("{} /x", "9".repeat(10_000)));
    let _ = split_chain(&"|".repeat(10_000));
}

#[test]
fn taint_marks_round_trip() {
    let mut r = Rng::new(0x7A1);
    for _ in 0..3000 {
        let s = r.text(40);
        let m = taint::mark(&s);
        assert_eq!(taint::clean(&m), s, "{s:?}");
        assert!(taint::has(&m) || s.is_empty(), "{s:?}");
        let _ = taint::clean(&s);
    }
}
