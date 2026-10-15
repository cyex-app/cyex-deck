//! Fills in a parsed command (Chatty's rules): a required replacement (`$$`) that comes
//! out empty cancels the whole command, which shows as `None`.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::parse::{Func, Id, Item, Items, Src};
use super::vars::Var;
use super::{calc, datetime, json, text};

/// What a command can read: its words (`$1`) and named values (`$(chan)`).
#[derive(Clone, Debug, Default)]
pub struct Params {
    args: Vec<String>,
    values: HashMap<String, String>,
    /// The JSON `$j` reads inside `$json`.
    json: Option<Arc<Value>>,
    /// Custom replacements already running, so one can't call itself.
    running: Vec<String>,
    /// Escape top-level results for `/chain` (`|`) or `/foreach` (`>`).
    pub(super) escape: Option<char>,
}

impl Params {
    /// `args` are split on single spaces, like Chatty.
    pub fn new(args: &str) -> Self {
        let mut p = Self::default();
        p.set_args(args);
        p
    }

    pub fn set_args(&mut self, args: &str) {
        let mut words: Vec<String> = args.split(' ').map(str::to_owned).collect();
        while words.len() > 1 && words.last().is_some_and(String::is_empty) {
            words.pop();
        }
        self.args = words;
    }

    /// Empty values count as missing.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        if !value.is_empty() {
            self.values.insert(key.to_owned(), value);
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Names set so far, sorted.
    pub fn keys(&self) -> Vec<&str> {
        let mut k: Vec<&str> = self.values.keys().map(String::as_str).collect();
        k.sort_unstable();
        k
    }

    fn range(&self, from: usize, rest: bool) -> String {
        match self.args.get(from - 1) {
            None => String::new(),
            Some(one) if !rest => one.clone(),
            Some(_) => self.args[from - 1..].join(" "),
        }
    }

    pub(super) fn with_json(&self, json: Arc<Value>) -> Self {
        let mut p = self.clone();
        p.json = Some(json);
        p
    }
}

/// The app's side of a command: custom replacements, variables, web requests.
pub trait Host {
    /// `$(_name)`: the user's custom replacement for this channel.
    fn custom(&self, name: &str) -> Option<Arc<Items>>;
    /// `$get(var,key)` and other values.
    fn get(&self, name: &str, key: Option<&str>) -> Option<String>;
    /// `$request`: a GET, answered with the body.
    fn request(&mut self, url: &str, headers: &[(String, String)]) -> Result<String, String>;
    /// `$is`: whether the highlight-style match fits.
    fn is(&self, _matcher: &str) -> bool {
        false
    }
    /// `$input`: asks the user. `None` cancels.
    fn input(&mut self, _message: &str, _initial: &str) -> Option<String> {
        None
    }
    /// V1051-04 A 7TV variable. Empty when it can't be read; never fails.
    fn cosmetic(&mut self, _v: &Var) -> String {
        String::new()
    }
    /// A number below `below`.
    fn random(&mut self, below: u64) -> u64;
    /// Unix milliseconds.
    fn now_ms(&self) -> i64;
    /// Local time's distance from UTC.
    fn utc_offset_secs(&self) -> i32 {
        0
    }
}

pub fn eval(items: &Items, p: &Params, h: &mut dyn Host) -> Option<String> {
    let mut out = String::new();
    for it in items {
        out.push_str(&item(it, p, h)?);
    }
    Some(out)
}

/// Like `eval`, escaping what replacements put in when the command chains or loops.
pub(super) fn eval_top(items: &Items, p: &Params, h: &mut dyn Host) -> Option<String> {
    let mut out = String::new();
    for it in items {
        let value = item(it, p, h)?;
        match (it, p.escape) {
            (Item::Text(_), _) | (_, None) => out.push_str(&value),
            (_, Some(c)) => out.push_str(&text::double_runs(&value, c)),
        }
    }
    Some(out)
}

/// Empty and required: cancel.
fn req(value: String, required: bool) -> Option<String> {
    if required && value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn opt(items: Option<&Items>, p: &Params, h: &mut dyn Host) -> Option<String> {
    items.map_or(Some(String::new()), |i| eval(i, p, h))
}

fn id(id: &Id, p: &Params, h: &mut dyn Host) -> Option<String> {
    match id {
        Id::Arg { from, rest } => Some(p.range(*from, *rest)),
        Id::Name(name) => {
            if let Some(v) = p.values.get(name) {
                return Some(v.clone());
            }
            if !name.starts_with('_') {
                return Some(String::new());
            }
            if p.running.contains(name) {
                return Some("[recursion not allowed]".into());
            }
            let Some(body) = h.custom(name) else {
                return Some(String::new());
            };
            let mut sub = p.clone();
            sub.running.push(name.clone());
            sub.escape = None;
            eval(&body, &sub, h)
        }
    }
}

fn src(s: &Src, p: &Params, h: &mut dyn Host) -> Option<String> {
    match s {
        Src::Id(i) => id(i, p, h),
        Src::Items(items) => eval(items, p, h),
    }
}

fn item(it: &Item, p: &Params, h: &mut dyn Host) -> Option<String> {
    match it {
        Item::Text(t) => Some(t.clone()),
        Item::Var(v) => Some(h.cosmetic(v)),
        Item::Repl { id: i, args, req: r } => {
            let value = match args {
                Some(args) => {
                    let mut sub = p.clone();
                    sub.set_args(&eval(args, p, h)?);
                    id(i, &sub, h)?
                }
                None => id(i, p, h)?,
            };
            req(value, *r)
        }
        Item::If {
            src: s,
            then,
            other,
            req: r,
        } => {
            let value = src(s, p, h)?;
            let out = if value.is_empty() {
                opt(other.as_ref(), p, h)?
            } else {
                eval(then, p, h)?
            };
            req(out, *r)
        }
        Item::IfEq {
            src: s,
            cmp,
            then,
            other,
            req: r,
        } => {
            let value = src(s, p, h)?;
            let out = if value == eval(cmp, p, h)? {
                eval(then, p, h)?
            } else {
                opt(other.as_ref(), p, h)?
            };
            req(out, *r)
        }
        Item::Switch {
            src: s,
            cases,
            default,
            req: r,
        } => {
            let value = src(s, p, h)?;
            for (key, result) in cases {
                if eval(key, p, h)? == value {
                    return req(eval(result, p, h)?, *r);
                }
            }
            req(eval(default, p, h)?, *r)
        }
        Item::Join { src: s, sep, req: r } => {
            let value = src(s, p, h)?;
            let sep = eval(sep, p, h)?;
            req(value.split(' ').collect::<Vec<_>>().join(&sep), *r)
        }
        Item::Lower { src: s, req: r } => req(src(s, p, h)?.to_lowercase(), *r),
        Item::Upper { src: s, req: r } => req(src(s, p, h)?.to_uppercase(), *r),
        Item::J {
            path,
            default,
            subs,
            req: r,
        } => {
            let out = json::path_item(path, default.as_ref(), subs, p, h)?;
            req(out, *r)
        }
        Item::Call { f, args, req: r } => req(call(*f, args, p, h)?, *r),
    }
}

fn call(f: Func, args: &[Items], p: &Params, h: &mut dyn Host) -> Option<String> {
    let arg = |i: usize, h: &mut dyn Host| -> Option<String> { opt(args.get(i), p, h) };
    // An optional parameter: `Some(None)` when it wasn't given.
    let given = |i: usize, h: &mut dyn Host| -> Option<Option<String>> {
        match args.get(i) {
            Some(a) => eval(a, p, h).map(Some),
            None => Some(None),
        }
    };
    Some(match f {
        Func::Trim => arg(0, h)?.trim_matches(|c: char| c <= ' ').to_owned(),
        Func::Rand => {
            let pick = h.random(args.len() as u64) as usize;
            arg(pick, h)?
        }
        Func::RandNum => {
            let a = arg(0, h)?;
            let (from, to) = match args.len() {
                1 => (0, a.trim().parse().unwrap_or(i64::MAX / 2)),
                _ => (
                    a.trim().parse().unwrap_or(0),
                    arg(1, h)?.trim().parse().unwrap_or(i64::MAX / 2),
                ),
            };
            let (lo, hi) = (from.min(to), from.max(to));
            let span = (hi - lo) as u64 + 1;
            (lo + h.random(span) as i64).to_string()
        }
        Func::DateTime => {
            let format = arg(0, h)?;
            let zone = arg(1, h)?;
            let at = arg(3, h)?.trim().parse::<i64>().unwrap_or_else(|_| h.now_ms());
            datetime::format(&format, &zone, at, h.utc_offset_secs())
        }
        Func::UrlEncode => url::form_urlencoded::byte_serialize(arg(0, h)?.as_bytes()).collect(),
        Func::Cs => text::double_runs(&arg(0, h)?, '|'),
        Func::Fs => text::double_runs(&arg(0, h)?, '>'),
        Func::Sort => {
            let input = arg(0, h)?;
            let kind = arg(1, h)?;
            let sep = given(2, h)?.unwrap_or_else(|| " ".into());
            text::sort(&input, kind == "Abc", &sep)
        }
        Func::Replace => text::replace(args, p, h)?,
        Func::Is => {
            let matcher = arg(0, h)?;
            if h.is(&matcher) {
                "true".into()
            } else {
                String::new()
            }
        }
        Func::Get => {
            let name = arg(0, h)?;
            let key = given(1, h)?;
            h.get(&name, key.as_deref()).unwrap_or_default()
        }
        Func::Calc => calc::calc(&arg(0, h)?).map(calc::number).unwrap_or_default(),
        Func::Round => {
            let n = arg(0, h)?;
            let places = arg(1, h)?;
            let mode = arg(2, h)?;
            let min = arg(3, h)?;
            text::round(&n, &places, &mode, &min)
        }
        Func::Input => {
            let message = arg(0, h)?;
            let initial = arg(1, h)?;
            h.input(&message, &initial)?
        }
        Func::Request => {
            let url = arg(0, h)?;
            let mut headers = Vec::new();
            let (mut show_error, mut trim) = (false, false);
            for i in 1..args.len() {
                let o = arg(i, h)?;
                match o.as_str() {
                    "error" => show_error = true,
                    "trim" => trim = true,
                    _ => {
                        if let Some((k, v)) = o
                            .strip_prefix("h[")
                            .and_then(|r| r.strip_suffix(']'))
                            .and_then(|r| r.split_once(':'))
                        {
                            headers.push((k.trim().to_owned(), v.trim().to_owned()));
                        }
                    }
                }
            }
            match h.request(&url, &headers) {
                Ok(body) if trim => body.trim().to_owned(),
                Ok(body) => body,
                Err(e) if show_error => e,
                Err(_) => String::new(),
            }
        }
        Func::Json => {
            let input = arg(0, h)?;
            let value = serde_json::from_str::<Value>(&input).unwrap_or(Value::Null);
            let sub = p.with_json(Arc::new(value));
            opt(args.get(1), &sub, h)?
        }
        Func::Quote => {
            let input = arg(0, h)?;
            let q = given(1, h)?
                .filter(|q| !q.is_empty())
                .unwrap_or_else(|| "\"".into());
            format!("{q}{}{q}", input.replace(&q, &format!("{q}{q}")))
        }
    })
}

pub(super) fn json_of(p: &Params) -> Option<&Arc<Value>> {
    p.json.as_ref()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::commands::parse::parse;

    #[derive(Default)]
    pub struct TestHost {
        pub customs: HashMap<String, Arc<Items>>,
        pub vars: HashMap<String, String>,
        pub next_random: u64,
    }

    impl Host for TestHost {
        fn custom(&self, name: &str) -> Option<Arc<Items>> {
            self.customs.get(name).cloned()
        }
        fn get(&self, name: &str, key: Option<&str>) -> Option<String> {
            (name == "var").then(|| self.vars.get(key?).cloned()).flatten()
        }
        fn request(&mut self, url: &str, _: &[(String, String)]) -> Result<String, String> {
            Ok(format!(" body of {url} "))
        }
        fn cosmetic(&mut self, v: &Var) -> String {
            match v {
                Var::Paint => "P".into(),
                Var::PaintId => "a|b".into(),
                _ => String::new(),
            }
        }
        fn random(&mut self, below: u64) -> u64 {
            self.next_random % below
        }
        fn now_ms(&self) -> i64 {
            1_552_489_908_000
        }
    }

    pub fn run(body: &str, args: &str) -> Option<String> {
        let mut p = Params::new(args);
        p.set("chan", "forsen");
        eval(&parse(body).unwrap(), &p, &mut TestHost::default())
    }

    #[test]
    fn parameters() {
        assert_eq!(
            run("/me slaps $$1 around with a $if(2,$2-,trout)", "nightbot").unwrap(),
            "/me slaps nightbot around with a trout"
        );
        assert_eq!(
            run("/me slaps $$1 around with a $if(2,$2-,trout)", "a big fish").unwrap(),
            "/me slaps a around with a big fish"
        );
        assert_eq!(run("/me slaps $$1", ""), None);
        assert_eq!(run("in $(chan): $(nope)$3", "a b").unwrap(), "in forsen: ");
        assert_eq!(run("$join(1-,/)", "a b c").unwrap(), "a/b/c");
        assert_eq!(run("$$join(1-,/)", ""), None);
    }

    #[test]
    fn decisions() {
        assert_eq!(run("$ifeq(1,cake,yummy,meh)", "cake").unwrap(), "yummy");
        assert_eq!(run("$ifeq(1,cake,yummy)", "pie").unwrap(), "");
        assert_eq!(run("$switch(1,25:50,50:18,25)", "50").unwrap(), "18");
        assert_eq!(run("$switch(1,25:50,50:18,25)", "x").unwrap(), "25");
        assert_eq!(
            run("$upper(1) $lower($1-)", "Fremily A B").unwrap(),
            "FREMILY fremily a b"
        );
    }

    #[test]
    fn custom_replacements_and_recursion() {
        let mut h = TestHost::default();
        h.customs
            .insert("_m".into(), Arc::new(parse("$ifeq(1,$(chan),,$$1: )").unwrap()));
        h.customs
            .insert("_loop".into(), Arc::new(parse("[$(_loop)]").unwrap()));
        let mut p = Params::new("bob");
        p.set("chan", "forsen");
        assert_eq!(eval(&parse("$(_m)FAQ").unwrap(), &p, &mut h).unwrap(), "bob: FAQ");
        assert_eq!(
            eval(&parse("$(_m,forsen)FAQ").unwrap(), &p, &mut h).unwrap(),
            "FAQ"
        );
        assert_eq!(
            eval(&parse("$(_loop)").unwrap(), &p, &mut h).unwrap(),
            "[[recursion not allowed]]"
        );
    }

    #[test]
    fn braces_vars() {
        assert_eq!(run("{paint}-{badge}", "").unwrap(), "P-");
        assert_eq!(run("$$1 {badge}", ""), None);
        assert_eq!(run("x {badge} y", "").unwrap(), "x  y");
        let mut p = Params::new("");
        let items = parse("/chain {paint.id}").unwrap();
        assert_eq!(crate::commands::run(&items, &p, &mut TestHost::default()).unwrap(), "/chain a||b");
        p.set("k", "v");
        assert_eq!(p.keys(), ["k"]);
    }

    #[test]
    fn functions() {
        assert_eq!(run("$quote(a\"b) $quote(x,')", "").unwrap(), "\"a\"\"b\" 'x'");
        assert_eq!(run("$urlencode($1-)", "a b&c").unwrap(), "a+b%26c");
        assert_eq!(run("$trim( x )", "").unwrap(), "x");
        assert_eq!(run("$rand(a,b,c)", "").unwrap(), "a");
        assert_eq!(run("$randnum(5,5)", "").unwrap(), "5");
        assert_eq!(run("$request(https://x,trim)", "").unwrap(), "body of https://x");
        assert_eq!(run("$cs(a|b)", "").unwrap(), "a||b");
    }
}
