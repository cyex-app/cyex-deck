//! V105-16 Chatterino's highlight regexes (Qt's PCRE2 10.43+ with Unicode properties) in
//! the Rust flavor highlights run on (fancy-regex). Only what reads differently is
//! rewritten; what has no equivalent is an error naming it.

mod class;
mod escape;
mod group;
mod props;
#[cfg(test)]
mod tests;

/// `(?a…)` restrictions in force.
#[derive(Clone, Copy, Default)]
struct Ascii {
    d: bool,
    s: bool,
    w: bool,
    posix: bool,
    digit: bool,
}

/// Options that last until their group closes.
#[derive(Clone, Copy, Default)]
struct Scope {
    n: bool,
    x: bool,
    xx: bool,
    ascii: Ascii,
    /// A condition's assertion: closes as `))`.
    wrap: bool,
}

struct Conv {
    src: Vec<char>,
    i: usize,
    out: String,
    scopes: Vec<Scope>,
    groups: u32,
    numbered_refs: bool,
    /// The next group is a condition's assertion.
    wrap_next: bool,
}

/// A PCRE2 pattern in the Rust flavor, or why it can't be carried over.
pub fn to_rust(src: &str) -> Result<String, String> {
    let mut c = Conv::new(src);
    c.options()?;
    c.run()?;
    Ok(c.out)
}

fn unsupported(what: &str) -> String {
    format!("Uses {what}, which isn't supported here.")
}

impl Conv {
    fn new(src: &str) -> Self {
        let src: Vec<char> = src.chars().collect();
        Self {
            numbered_refs: has_numbered_refs(&src),
            src,
            i: 0,
            out: String::new(),
            scopes: vec![Scope::default()],
            groups: 0,
            wrap_next: false,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.i).copied()
    }

    fn at(&self, n: usize) -> Option<char> {
        self.src.get(self.i + n).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        self.i += 1;
        c
    }

    fn starts(&self, s: &str) -> bool {
        s.chars().enumerate().all(|(k, c)| self.at(k) == Some(c))
    }

    fn eat(&mut self, s: &str) -> bool {
        let yes = self.starts(s);
        if yes {
            self.i += s.chars().count();
        }
        yes
    }

    fn scope(&self) -> Scope {
        self.scopes.last().copied().unwrap_or_default()
    }

    fn scope_mut(&mut self) -> &mut Scope {
        if self.scopes.is_empty() {
            self.scopes.push(Scope::default());
        }
        let last = self.scopes.len() - 1;
        &mut self.scopes[last]
    }

    /// Everything up to `end` (consumed too), or `None` when it never comes.
    fn until(&mut self, end: char) -> Option<String> {
        let at = self.src.get(self.i..)?.iter().position(|&c| c == end)?;
        let text: String = self.src[self.i..self.i + at].iter().collect();
        self.i += at + 1;
        Some(text)
    }

    /// One set: `[body]`, or inside a class the body itself (nested `[^body]` when negated).
    fn set(&mut self, body: &str, negated: bool, in_class: bool) {
        if in_class && !negated {
            self.out.push_str(body);
            return;
        }
        self.out.push_str(if negated { "[^" } else { "[" });
        self.out.push_str(body);
        self.out.push(']');
    }

    /// Leading `(*OPTION)`s: dropped.
    fn options(&mut self) -> Result<(), String> {
        const KNOWN: [&str; 11] = [
            "UTF",
            "UCP",
            "NO_AUTO_POSSESS",
            "NO_DOTSTAR_ANCHOR",
            "NO_JIT",
            "NO_START_OPT",
            "CR",
            "LF",
            "CRLF",
            "ANYCRLF",
            "ANY",
        ];
        const MORE: [&str; 4] = ["NUL", "BSR_ANYCRLF", "BSR_UNICODE", "LIMIT_"];
        while self.starts("(*") {
            let Some(close) = self.src[self.i..].iter().position(|&c| c == ')') else {
                break;
            };
            let name: String = self.src[self.i + 2..self.i + close].iter().collect();
            if name.starts_with("NOTEMPTY") {
                return Err(unsupported("(*NOTEMPTY)"));
            }
            let known = KNOWN.contains(&name.as_str()) || MORE.iter().any(|m| name.starts_with(m));
            if !known {
                break;
            }
            self.i += close + 1;
        }
        Ok(())
    }

    fn run(&mut self) -> Result<(), String> {
        while let Some(c) = self.peek() {
            match c {
                '\\' => {
                    self.i += 1;
                    self.escape(false)?;
                }
                '[' => self.class()?,
                '(' => self.group()?,
                ')' => {
                    self.i += 1;
                    let closed = if self.scopes.len() > 1 {
                        self.scopes.pop()
                    } else {
                        None
                    };
                    self.out
                        .push_str(if closed.is_some_and(|s| s.wrap) { "))" } else { ")" });
                }
                '{' => self.brace(),
                '#' if self.scope().x => {
                    while let Some(c) = self.peek().filter(|&c| c != '\n') {
                        self.out.push(c);
                        self.i += 1;
                    }
                }
                c => {
                    self.out.push(c);
                    self.i += 1;
                }
            }
        }
        Ok(())
    }

    /// `{n}` `{n,}` `{n,m}` `{,m}` (spaces allowed); any other `{` is a literal.
    fn brace(&mut self) {
        let mut k = 1;
        let num = |me: &Self, k: &mut usize| {
            while me.at(*k) == Some(' ') {
                *k += 1;
            }
            let mut n = String::new();
            while let Some(d) = me.at(*k).filter(char::is_ascii_digit) {
                n.push(d);
                *k += 1;
            }
            while me.at(*k) == Some(' ') {
                *k += 1;
            }
            n
        };
        let lo = num(self, &mut k);
        let comma = self.at(k) == Some(',');
        let mut hi = String::new();
        if comma {
            k += 1;
            hi = num(self, &mut k);
        }
        if self.at(k) == Some('}') && (!lo.is_empty() || !hi.is_empty()) {
            self.i += k + 1;
            let lo = if lo.is_empty() { "0".to_owned() } else { lo };
            self.out.push_str(&match (comma, hi.is_empty()) {
                (false, _) => format!("{{{lo}}}"),
                (true, true) => format!("{{{lo},}}"),
                (true, false) => format!("{{{lo},{hi}}}"),
            });
        } else {
            self.i += 1;
            self.out.push_str(r"\{");
        }
    }
}

/// Is a group referred to by number anywhere? Only `(?|` cares.
fn has_numbered_refs(src: &[char]) -> bool {
    let num = |k: usize| src.get(k).is_some_and(char::is_ascii_digit);
    let signed = |k: usize| num(k) || (matches!(src.get(k), Some('+' | '-')) && num(k + 1));
    let mut i = 0;
    while i < src.len() {
        match src[i] {
            '\\' => {
                match src.get(i + 1) {
                    Some('1'..='9') => return true,
                    Some('g') => {
                        let k = i + 2;
                        if num(k) || src.get(k) == Some(&'-') {
                            return true;
                        }
                        if matches!(src.get(k), Some('{' | '<' | '\'')) && signed(k + 1) {
                            return true;
                        }
                    }
                    Some('Q') => {
                        i += 2;
                        while i < src.len() && !(src[i] == '\\' && src.get(i + 1) == Some(&'E')) {
                            i += 1;
                        }
                    }
                    _ => {}
                }
                i += 2;
            }
            '[' => {
                i += 1;
                if src.get(i) == Some(&'^') {
                    i += 1;
                }
                if src.get(i) == Some(&']') {
                    i += 1;
                }
                while i < src.len() && src[i] != ']' {
                    i += if src[i] == '\\' { 2 } else { 1 };
                }
                i += 1;
            }
            '(' if src.get(i + 1) == Some(&'?') => {
                if signed(i + 2) || (src.get(i + 2) == Some(&'(') && signed(i + 3)) {
                    return true;
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    false
}
