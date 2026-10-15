//! V105-16 `\…`: PCRE2's escapes in the Rust flavor.

use super::props::{self, H, V};
use super::{Conv, unsupported};
use crate::mentions::flavor::{JS_EDGE, JS_NOT_EDGE, push_literal};

impl Conv {
    /// After a `\`. Inside a class the sets are bodies, not brackets.
    pub(super) fn escape(&mut self, in_class: bool) -> Result<(), String> {
        let Some(e) = self.next() else {
            return Err(r"Ends with a lone \.".to_owned());
        };
        let ascii = self.scope().ascii;
        match e {
            'Q' => {
                while !self.starts(r"\E") {
                    let Some(c) = self.next() else { break };
                    push_literal(&mut self.out, c);
                }
                self.eat(r"\E");
            }
            'E' => {}
            'h' | 'H' => self.set(H, e == 'H', in_class),
            'v' | 'V' => self.set(V, e == 'V', in_class),
            'R' | 'K' | 'G' | 'A' | 'z' | 'Z' | 'a' | 'e' | 'f' | 'n' | 'r' | 't' => {
                self.out.push('\\');
                self.out.push(e);
            }
            'X' | 'C' => return Err(unsupported(&format!("\\{e}"))),
            'N' => {
                if self.starts("{U+") {
                    self.i += 3;
                    let hex = self.until('}').unwrap_or_default();
                    self.out.push_str(&format!(r"\x{{{hex}}}"));
                } else {
                    self.out.push_str(r"\N");
                }
            }
            'x' => {
                let hex = if self.peek() == Some('{') {
                    self.i += 1;
                    self.until('}').ok_or(r"\x{ isn't closed.")?
                } else {
                    let mut h = String::new();
                    while h.len() < 2 && self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                        h.extend(self.next());
                    }
                    h
                };
                self.code(u32::from_str_radix(if hex.is_empty() { "0" } else { &hex }, 16))?;
            }
            'o' => {
                if self.next() != Some('{') {
                    return Err(r"\o needs braces.".to_owned());
                }
                let oct = self.until('}').ok_or(r"\o{ isn't closed.")?;
                self.code(u32::from_str_radix(&oct, 8))?;
            }
            '0' => {
                self.i -= 1;
                self.octal();
            }
            '1'..='9' => {
                self.i -= 1;
                if in_class {
                    self.class_digit();
                } else {
                    self.number();
                }
            }
            'c' => {
                let Some(x) = self.next().filter(char::is_ascii) else {
                    return Err(r"\c needs a character after it.".to_owned());
                };
                self.out
                    .push_str(&format!(r"\x{{{:X}}}", x.to_ascii_uppercase() as u32 ^ 0x40));
            }
            'g' => self.back_ref_g()?,
            'k' => {
                if self.peek() == Some('{') {
                    self.i += 1;
                    let name = self.until('}').ok_or(r"\k{ isn't closed.")?;
                    self.out.push_str(&format!(r"\k<{name}>"));
                } else {
                    self.out.push_str(r"\k");
                }
            }
            'p' | 'P' => self.property(e == 'P', in_class)?,
            'd' | 'D' if ascii.d => self.set("0-9", e == 'D', in_class),
            's' | 'S' if ascii.s => self.set(r"\t\n\x0B\f\r\x{20}", e == 'S', in_class),
            'w' | 'W' if ascii.w => self.set("0-9A-Za-z_", e == 'W', in_class),
            'b' | 'B' if ascii.w && !in_class => {
                self.out.push_str(if e == 'b' { JS_EDGE } else { JS_NOT_EDGE })
            }
            'd' | 'D' | 's' | 'S' | 'w' | 'W' | 'b' | 'B' => {
                self.out.push('\\');
                self.out.push(e);
            }
            c if c.is_ascii_alphanumeric() => return Err(format!("\\{c} isn't a PCRE2 escape.")),
            c => push_literal(&mut self.out, c),
        }
        Ok(())
    }

    /// A code point as `\x{H}`.
    fn code(&mut self, n: Result<u32, std::num::ParseIntError>) -> Result<(), String> {
        let n = n.map_err(|_| "That character code is too big.".to_owned())?;
        self.out.push_str(&format!(r"\x{{{n:X}}}"));
        Ok(())
    }

    /// Up to 3 octal digits at the cursor, as `\x{H}`.
    fn octal(&mut self) {
        let mut n = 0u32;
        let mut k = 0;
        while k < 3 {
            let Some(d) = self.peek().and_then(|c| c.to_digit(8)) else {
                break;
            };
            n = n * 8 + d;
            self.i += 1;
            k += 1;
        }
        self.out.push_str(&format!(r"\x{{{n:X}}}"));
    }

    /// `\1`-`\9…` in a class: octal; `\8` and `\9` are those digits.
    fn class_digit(&mut self) {
        match self.peek() {
            Some(d @ ('8' | '9')) => {
                self.i += 1;
                self.out.push(d);
            }
            _ => self.octal(),
        }
    }

    /// `\N` outside a class: a backreference, or octal.
    fn number(&mut self) {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        let digits: String = self.src[start..self.i].iter().collect();
        let n: u32 = digits.parse().unwrap_or(u32::MAX);
        if n < 10 || digits.starts_with(['8', '9']) || n <= self.groups {
            self.out.push('\\');
            self.out.push_str(&digits);
            return;
        }
        self.i = start;
        self.octal();
        self.out.extend(&self.src[self.i..start + digits.len()]);
        self.i = start + digits.len();
    }

    /// `\g…`: numbers (also relative) become `\N`, names `\k<name>`, `\g<…>` stays.
    fn back_ref_g(&mut self) -> Result<(), String> {
        let from = self.i - 2;
        if matches!(self.peek(), Some('<' | '\'')) {
            self.out.push_str(r"\g");
            return Ok(());
        }
        let braced = self.peek() == Some('{');
        if braced {
            self.i += 1;
        }
        let mut word = String::new();
        if braced {
            word = self.until('}').ok_or(r"\g{ isn't closed.")?;
        } else {
            if self.peek() == Some('-') {
                word.extend(self.next());
            }
            while let Some(d) = self.peek().filter(char::is_ascii_digit) {
                word.push(d);
                self.i += 1;
            }
        }
        let num = word.strip_prefix('-').unwrap_or(&word);
        if num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) {
            self.out.push_str(&format!(r"\k<{word}>"));
            return Ok(());
        }
        let n: u32 = num.parse().unwrap_or(u32::MAX);
        let abs = if word.starts_with('-') {
            (self.groups + 1).checked_sub(n).filter(|_| n > 0)
        } else {
            Some(n)
        };
        match abs {
            Some(a) if a >= 1 || !word.starts_with('-') => self.out.push_str(&format!("\\{a}")),
            _ => {
                let text: String = self.src[from..self.i].iter().collect();
                return Err(format!("{text} points before the first group."));
            }
        }
        Ok(())
    }

    /// `\p{…}`, `\P{…}`, `\pX`.
    fn property(&mut self, mut negated: bool, in_class: bool) -> Result<(), String> {
        let name = if self.peek() == Some('{') {
            self.i += 1;
            self.until('}').ok_or(r"\p{ isn't closed.")?
        } else {
            let Some(c) = self.next() else {
                return Err(r"\p needs a property name.".to_owned());
            };
            self.out.push_str(if negated { r"\P" } else { r"\p" });
            self.out.push(c);
            return Ok(());
        };
        let name = match name.strip_prefix('^') {
            Some(rest) => {
                negated = !negated;
                rest.to_owned()
            }
            None => name,
        };
        if name == "Xwd" {
            self.out.push_str(if negated { r"\W" } else { r"\w" });
        } else if let Some(body) = props::property(&name) {
            self.set(body, negated, in_class);
        } else {
            let name = if name == "L&" || name == "Lc" { "LC" } else { &name };
            self.out
                .push_str(&format!(r"\{}{{{name}}}", if negated { 'P' } else { 'p' }));
        }
        Ok(())
    }
}
