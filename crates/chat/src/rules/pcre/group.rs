//! V105-16 `(…)`: groups, lookaround, verbs, conditions, subroutine calls and inline flags.

use super::{Ascii, Conv, Scope, unsupported};

const NOT_CLOSED: &str = "A (?C or (?# isn’t closed.";

impl Conv {
    /// Opens a scope that lasts until its `)`.
    fn open(&mut self, text: &str) {
        let mut scope = self.scope();
        scope.wrap = std::mem::take(&mut self.wrap_next);
        self.scopes.push(scope);
        self.out.push_str(text);
    }

    /// At a `(`.
    pub(super) fn group(&mut self) -> Result<(), String> {
        if self.eat("(*") {
            return self.verb();
        }
        if !self.eat("(?") {
            self.i += 1;
            if self.scope().n {
                self.open("(?:");
            } else {
                self.groups += 1;
                self.open("(");
            }
            return Ok(());
        }
        let Some(c) = self.peek() else {
            self.out.push_str("(?");
            return Ok(());
        };
        match c {
            '#' => {
                self.until(')').ok_or(NOT_CLOSED)?;
            }
            'C' => self.callout()?,
            '|' => {
                if self.numbered_refs {
                    return Err(unsupported("(?| with numbered groups"));
                }
                self.i += 1;
                self.open("(?:");
            }
            ':' | '=' | '!' | '>' => {
                self.i += 1;
                self.open(&format!("(?{c}"));
            }
            '<' if matches!(self.at(1), Some('=' | '!')) => {
                let t = format!("(?<{}", self.at(1).unwrap_or('='));
                self.i += 2;
                self.open(&t);
            }
            '<' | '\'' => {
                self.named_open(c);
            }
            'P' if self.at(1) == Some('<') => {
                self.i += 2;
                let name = self.until('>').unwrap_or_default();
                self.groups += 1;
                self.open(&format!("(?P<{name}>"));
            }
            'P' if matches!(self.at(1), Some('=' | '>')) => {
                self.out.push_str("(?");
                self.copy_to_close();
            }
            '&' => {
                self.i += 1;
                let name = self.until(')').ok_or("A subroutine call isn’t closed.")?;
                self.out.push_str(&format!(r"\g<{name}>"));
            }
            'R' if self.at(1) == Some(')') => {
                self.i += 2;
                self.out.push_str(r"\g<0>");
            }
            '0'..='9' | '+' | '-' if self.is_call() => {
                let n = self.until(')').ok_or("A subroutine call isn’t closed.")?;
                self.out.push_str(&format!(r"\g<{n}>"));
            }
            '(' => self.condition()?,
            _ => self.flags()?,
        }
        Ok(())
    }

    /// `(?N)`, `(?+N)`, `(?-N)`, but not `(?-i)`.
    fn is_call(&self) -> bool {
        let digit = |k| self.at(k).is_some_and(|c| c.is_ascii_digit());
        digit(0) || (matches!(self.peek(), Some('+' | '-')) && digit(1))
    }

    /// `(?<name>` or `(?'name'`: a capturing group.
    fn named_open(&mut self, open: char) {
        let close = if open == '<' { '>' } else { '\'' };
        self.i += 1;
        let name = self.until(close).unwrap_or_default();
        self.groups += 1;
        self.open(&format!("(?{open}{name}{close}"));
    }

    /// Copies source text as is up to and including the next `)`.
    fn copy_to_close(&mut self) {
        while let Some(c) = self.next() {
            self.out.push(c);
            if c == ')' {
                break;
            }
        }
    }

    /// After `(?C`: callouts do nothing under Qt.
    fn callout(&mut self) -> Result<(), String> {
        self.i += 1;
        if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.i += 1;
            }
        } else if let Some(open) = self.peek().filter(|c| "`'\"^%#$".contains(*c) || *c == '{') {
            let close = if open == '{' { '}' } else { open };
            self.i += 1;
            loop {
                match self.next() {
                    Some(c) if c == close && self.peek() == Some(close) => self.i += 1,
                    Some(c) if c == close => break,
                    Some(_) => {}
                    None => return Err(NOT_CLOSED.to_owned()),
                }
            }
        }
        if self.next() == Some(')') {
            Ok(())
        } else {
            Err(NOT_CLOSED.to_owned())
        }
    }

    /// After `(?` at the `(` of a condition.
    fn condition(&mut self) -> Result<(), String> {
        let after = self.at(1);
        if matches!(after, Some('?' | '*')) {
            // The assertion keeps its own group, wrapped as fancy wants it.
            self.out.push_str("(?(");
            let mut scope = self.scope();
            scope.wrap = false;
            self.scopes.push(scope);
            self.wrap_next = true;
            return Ok(());
        }
        let rest: String = self.src[self.i + 1..].iter().take_while(|&&c| c != ')').collect();
        if rest == "R"
            || rest.starts_with("R&")
            || (rest.starts_with('R') && rest[1..].bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(unsupported("a recursion check"));
        }
        if rest.starts_with("VERSION") {
            return Err(unsupported("a version check"));
        }
        self.out.push_str("(?");
        self.copy_to_close();
        let scope = self.scope();
        self.scopes.push(Scope { wrap: false, ..scope });
        Ok(())
    }

    /// After `(*`.
    fn verb(&mut self) -> Result<(), String> {
        let Some(end) = self.src[self.i..].iter().position(|&c| c == ')' || c == ':') else {
            self.out.push_str("(*");
            return Ok(());
        };
        let name: String = self.src[self.i..self.i + end].iter().collect();
        let colon = self.src[self.i + end] == ':';
        let looks = match name.as_str() {
            "pla" | "positive_lookahead" => Some("(?="),
            "nla" | "negative_lookahead" => Some("(?!"),
            "plb" | "positive_lookbehind" => Some("(?<="),
            "nlb" | "negative_lookbehind" => Some("(?<!"),
            "atomic" => Some("(?>"),
            _ => None,
        };
        if let (Some(open), true) = (looks, colon) {
            self.i += end + 1;
            self.open(open);
            return Ok(());
        }
        match name.as_str() {
            "napla" | "naplb" | "non_atomic_positive_lookahead" | "non_atomic_positive_lookbehind" => {
                return Err(unsupported("a non-atomic lookaround"));
            }
            "sr" | "asr" | "script_run" | "atomic_script_run" => return Err(unsupported("a script run")),
            "THEN" => return Err(unsupported("(*THEN)")),
            "SKIP" if colon => return Err(unsupported("(*SKIP:name)")),
            "NOTEMPTY" | "NOTEMPTY_ATSTART" => return Err(unsupported("(*NOTEMPTY)")),
            "MARK" | "" if colon => {
                self.i += end;
                self.until(')').ok_or("A verb isn’t closed.")?;
            }
            // fancy-regex 0.19.2 refuses every verb but (*FAIL).
            "PRUNE" | "COMMIT" | "ACCEPT" | "SKIP" => return Err(unsupported(&format!("(*{name})"))),
            _ => {
                self.out.push_str("(*");
                self.copy_to_close();
            }
        }
        Ok(())
    }

    /// Inline flags `(?imsxnUJa^…)` / `(?flags:`.
    fn flags(&mut self) -> Result<(), String> {
        let Some(end) = self.src[self.i..].iter().position(|&c| c == ')' || c == ':') else {
            self.out.push_str("(?");
            return Ok(());
        };
        let spec: Vec<char> = self.src[self.i..self.i + end].to_vec();
        if !spec.iter().all(|c| "imsxnUJaDSWPT^-".contains(*c)) {
            self.out.push_str("(?");
            return Ok(());
        }
        if spec.contains(&'J') {
            return Err(unsupported("(?J)"));
        }
        let closes = self.src[self.i + end] == ')';
        self.i += end + 1;
        let mut scope = self.scope();
        let (mut on, mut off) = (String::new(), String::new());
        let mut neg = false;
        let mut k = 0;
        while k < spec.len() {
            let c = spec[k];
            k += 1;
            match c {
                '^' => {
                    scope.n = false;
                    scope.x = false;
                    scope.xx = false;
                    on.clear();
                    off = "imsx".to_owned();
                }
                '-' => neg = true,
                'a' => {
                    let mut a = scope.ascii;
                    match spec.get(k) {
                        Some(&p @ ('D' | 'S' | 'W' | 'P' | 'T')) => {
                            k += 1;
                            set_ascii(&mut a, p);
                        }
                        _ => {
                            a = Ascii {
                                d: true,
                                s: true,
                                w: true,
                                posix: true,
                                digit: true,
                            }
                        }
                    }
                    scope.ascii = a;
                }
                'n' => scope.n = !neg,
                'x' => {
                    if neg {
                        scope.x = false;
                        scope.xx = false;
                        push_once(&mut off, 'x');
                    } else {
                        scope.xx = k.checked_sub(2).and_then(|p| spec.get(p)) == Some(&'x');
                        scope.x = true;
                        push_once(&mut on, 'x');
                        off.retain(|o| o != 'x');
                    }
                }
                f => {
                    let (add, other) = if neg {
                        (&mut off, &mut on)
                    } else {
                        (&mut on, &mut off)
                    };
                    push_once(add, f);
                    other.retain(|o| o != f);
                }
            }
        }
        let flags = format!("{on}{}{off}", if off.is_empty() { "" } else { "-" });
        if closes {
            *self.scope_mut() = scope;
            if !on.is_empty() || !off.is_empty() {
                self.out.push_str(&format!("(?{flags})"));
            }
        } else {
            self.scopes.push(Scope { wrap: false, ..scope });
            self.out.push_str(&format!("(?{flags}:"));
        }
        Ok(())
    }
}

fn push_once(s: &mut String, c: char) {
    if !s.contains(c) {
        s.push(c);
    }
}

fn set_ascii(a: &mut Ascii, c: char) {
    match c {
        'D' => a.d = true,
        'S' => a.s = true,
        'W' => a.w = true,
        'P' => a.posix = true,
        _ => a.digit = true,
    }
}
