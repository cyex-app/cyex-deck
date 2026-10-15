//! V105-16 `[…]`: PCRE2's class syntax in Rust's (nested classes, `&&`, `~~`, `--`).

use super::{Conv, props};

impl Conv {
    /// At a `[`.
    pub(super) fn class(&mut self) -> Result<(), String> {
        if self.eat("[[:<:]]") {
            self.out.push_str(r"\b(?=\w)");
            return Ok(());
        }
        if self.eat("[[:>:]]") {
            self.out.push_str(r"\b(?<=\w)");
            return Ok(());
        }
        self.i += 1;
        self.out.push('[');
        if self.peek() == Some('^') {
            self.i += 1;
            self.out.push('^');
        }
        if self.peek() == Some(']') {
            self.i += 1;
            self.out.push_str(r"\]");
        }
        let scope = self.scope();
        let mut dash = false;
        while let Some(c) = self.next() {
            let was_dash = std::mem::take(&mut dash);
            match c {
                ']' => {
                    self.out.push(']');
                    return Ok(());
                }
                '\\' => self.escape(true)?,
                '[' => self.posix_or_bracket()?,
                '&' | '~' => {
                    self.out.push('\\');
                    self.out.push(c);
                }
                '-' => {
                    dash = !was_dash;
                    self.out.push_str(if was_dash { r"\-" } else { "-" });
                }
                ' ' | '\t' if scope.xx => {}
                c => self.out.push(c),
            }
        }
        Ok(())
    }

    /// After a `[` inside a class: `[:name:]` or a literal bracket.
    fn posix_or_bracket(&mut self) -> Result<(), String> {
        if self.peek() == Some(':') {
            let rest = &self.src[self.i + 1..];
            if let Some(end) = rest.windows(2).position(|w| w == [':', ']']) {
                let word: String = rest[..end].iter().collect();
                let (negated, name) = match word.strip_prefix('^') {
                    Some(n) => (true, n),
                    None => (false, word.as_str()),
                };
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase()) {
                    let body = props::posix(name, self.scope().ascii)
                        .ok_or_else(|| format!("Unknown class [:{name}:]."))?;
                    self.i += end + 3;
                    self.set(&body, negated, true);
                    return Ok(());
                }
            }
        }
        self.out.push_str(r"\[");
        Ok(())
    }
}
