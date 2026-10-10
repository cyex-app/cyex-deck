//! Text functions: `$replace`, `$sort`, `$round` and the `/chain` and `/foreach` escapes.

use regex::{Captures, NoExpand, Regex, RegexBuilder};

use super::eval::{Host, Params, eval};
use super::parse::Items;

/// Each run of `c` gets one more, so `/chain` (`|`) and `/foreach` (`>`) don't split
/// on text a replacement put in.
pub fn double_runs(text: &str, c: char) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = 0;
    for ch in text.chars() {
        if ch == c {
            run += 1;
        } else if run > 0 {
            out.extend(std::iter::repeat_n(c, run + 1));
            run = 0;
        }
        if ch != c {
            out.push(ch);
        }
    }
    if run > 0 {
        out.extend(std::iter::repeat_n(c, run + 1));
    }
    out
}

/// `abc` ignores case, `Abc` doesn't.
pub fn sort(input: &str, case_sensitive: bool, sep: &str) -> String {
    let sep = if sep.is_empty() { " " } else { sep };
    let mut parts: Vec<&str> = input.split(sep).collect();
    if case_sensitive {
        parts.sort_unstable();
    } else {
        parts.sort_by_key(|s| s.to_lowercase());
    }
    parts.join(sep)
}

/// `$round(number, places, mode, min places)`. Not a number: given back as is.
pub fn round(n: &str, places: &str, mode: &str, min: &str) -> String {
    let Ok(n) = n.trim().parse::<f64>() else {
        return n.to_owned();
    };
    let places = places.trim().parse::<i32>().unwrap_or(0).clamp(0, 15);
    let min = min.trim().parse::<usize>().unwrap_or(0).min(15);
    let scale = 10f64.powi(places);
    let x = n * scale;
    let r = match mode {
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "up" => x.abs().ceil().copysign(x),
        "down" => x.trunc(),
        "half-down" => {
            let t = x.trunc();
            if (x - t).abs() > 0.5 + f64::EPSILON * x.abs() {
                t + x.signum()
            } else {
                t
            }
        }
        // Half up, away from zero; nudged so 1.335 rounds like the decimal it was typed as.
        _ => (x + x.signum() * 1e-9 * x.abs().max(1.0)).round(),
    } / scale;
    let mut s = format!("{r:.*}", places as usize);
    if s.contains('.') {
        while s.ends_with('0') && s.len() - s.find('.').unwrap_or(0) - 1 > min {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if min > 0 {
        let have = s.find('.').map_or(0, |d| s.len() - d - 1);
        if have == 0 {
            s.push('.');
        }
        s.extend(std::iter::repeat_n('0', min.saturating_sub(have)));
    }
    if s == "-0" { "0".into() } else { s }
}

/// `$replace(input, search, replace, [method])`.
pub fn replace(args: &[Items], p: &Params, h: &mut dyn Host) -> Option<String> {
    let input = eval(&args[0], p, h)?;
    let search = eval(&args[1], p, h)?;
    let with = eval(&args[2], p, h)?;
    let method = match args.get(3) {
        Some(m) => eval(m, p, h)?,
        None => String::new(),
    };
    let regex = |case_insensitive: bool| {
        RegexBuilder::new(&search)
            .case_insensitive(case_insensitive)
            .build()
    };
    let out = match method.as_str() {
        "cs" => input.replace(&search, &with),
        "reg" | "regFirst" | "regRef" | "regFirstRef" => {
            let re = match regex(false) {
                Ok(re) => re,
                Err(e) => return Some(format!("Error: {e}")),
            };
            let first = method.starts_with("regFirst");
            let limit = if first { 1 } else { 0 };
            if method.ends_with("Ref") {
                re.replacen(&input, limit, java_replacement(&with).as_str())
                    .into_owned()
            } else {
                re.replacen(&input, limit, NoExpand(&with)).into_owned()
            }
        }
        "regCustom" | "regFunc" => {
            let re = match regex(false) {
                Ok(re) => re,
                Err(e) => return Some(format!("Error: {e}")),
            };
            custom(&re, &input, &args[2], &with, method == "regFunc", p, h)?
        }
        _ => match RegexBuilder::new(&regex::escape(&search))
            .case_insensitive(true)
            .build()
        {
            Ok(re) if !search.is_empty() => re.replace_all(&input, NoExpand(&with)).into_owned(),
            _ => input,
        },
    };
    Some(out)
}

/// Each match is replaced by the `replace` items (or the custom replacement it names),
/// with `$1-` the whole match and `$(g1)` its groups.
fn custom(
    re: &Regex,
    input: &str,
    items: &Items,
    name: &str,
    by_name: bool,
    p: &Params,
    h: &mut dyn Host,
) -> Option<String> {
    let mut out = String::new();
    let mut last = 0;
    let caps: Vec<Captures> = re.captures_iter(input).collect();
    for c in caps {
        let whole = c.get(0).expect("group 0 is the match");
        out.push_str(&input[last..whole.start()]);
        let mut sub = p.clone();
        sub.set_args(whole.as_str());
        for i in 1..c.len() {
            sub.set(&format!("g{i}"), c.get(i).map_or("", |m| m.as_str()));
        }
        let value = if by_name {
            match h.custom(name) {
                Some(body) => eval(&body, &sub, h)?,
                None => String::new(),
            }
        } else {
            eval(items, &sub, h)?
        };
        out.push_str(&value);
        last = whole.end();
    }
    out.push_str(&input[last..]);
    Some(out)
}

/// Java's replacement syntax (`$1`, `\$`, `\\`) to the regex crate's (`${1}`, `$$`).
fn java_replacement(java: &str) -> String {
    let mut out = String::new();
    let mut chars = java.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('$') => out.push_str("$$"),
                Some(other) => out.push(other),
                None => {}
            },
            '$' => {
                let mut digits = String::new();
                while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                    digits.push(*d);
                    chars.next();
                }
                if digits.is_empty() {
                    out.push_str("$$");
                } else {
                    out.push_str(&format!("${{{digits}}}"));
                }
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::eval::tests::run;

    #[test]
    fn escapes_runs() {
        assert_eq!(double_runs("a|b||c|", '|'), "a||b|||c||");
        assert_eq!(double_runs("x > y", '>'), "x >> y");
    }

    #[test]
    fn sorting() {
        assert_eq!(sort("B c a", false, " "), "a B c");
        assert_eq!(sort("B c a", true, " "), "B a c");
        assert_eq!(sort("b,a", false, ","), "a,b");
    }

    #[test]
    fn rounding() {
        assert_eq!(round("1.335", "", "", ""), "1");
        assert_eq!(round("1.335", "2", "", ""), "1.34");
        assert_eq!(round("345.5", "3", "floor", "3"), "345.500");
        assert_eq!(round("2.565", "2", "", "2"), "2.57");
        assert_eq!(round("2.5", "0", "half-down", ""), "2");
        assert_eq!(round("-1.2", "0", "up", ""), "-2");
        assert_eq!(round("abc", "2", "", ""), "abc");
    }

    #[test]
    fn replacing() {
        assert_eq!(
            run("$replace(A b and another B,b,bee)", "").unwrap(),
            "A bee and another bee"
        );
        assert_eq!(run("$replace(A b B,b,x,cs)", "").unwrap(), "A x B");
        assert_eq!(
            run(
                r"$replace(Was timed out for 30 seconds,.*?(\\d+) seconds,Number: \$1,regRef)",
                ""
            )
            .unwrap(),
            "Number: 30"
        );
        assert_eq!(
            run(r#"$replace($1-,$"(\w+)",$"($1)",regRef)"#, "hi you").unwrap(),
            "(hi) (you)"
        );
        assert_eq!(
            run(r#"$replace($1-,$"(\w+)",[$(g1)],regCustom)"#, "hi you").unwrap(),
            "[hi] [you]"
        );
        assert_eq!(run(r#"$replace(aaa,a,b,regFirst)"#, "").unwrap(), "baa");
        assert!(run(r#"$replace(x,$"(",y,reg)"#, "").unwrap().starts_with("Error"));
    }
}
