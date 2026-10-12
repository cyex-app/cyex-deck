//! `$calc`: `+ - * / % ^`, parentheses and `sqrt sin cos tan`, like Chatty.

pub fn calc(expr: &str) -> Option<f64> {
    let tokens = lex(expr)?;
    let mut p = Calc { tokens, pos: 0 };
    let v = p.sum()?;
    (p.pos == p.tokens.len() && v.is_finite()).then_some(v)
}

/// 8.0 → "8", 3.5 → "3.5".
pub fn number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Op(char),
    Name(String),
}

fn lex(expr: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            out.push(Tok::Num(chars[start..i].iter().collect::<String>().parse().ok()?));
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            out.push(Tok::Name(
                chars[start..i].iter().collect::<String>().to_ascii_lowercase(),
            ));
        } else if "+-*/%^()".contains(c) {
            out.push(Tok::Op(c));
            i += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

struct Calc {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Calc {
    fn op(&mut self, ops: &str) -> Option<char> {
        match self.tokens.get(self.pos) {
            Some(Tok::Op(c)) if ops.contains(*c) => {
                self.pos += 1;
                Some(*c)
            }
            _ => None,
        }
    }

    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        while let Some(op) = self.op("+-") {
            let r = self.product()?;
            v = if op == '+' { v + r } else { v - r };
        }
        Some(v)
    }

    fn product(&mut self) -> Option<f64> {
        let mut v = self.power()?;
        while let Some(op) = self.op("*/%") {
            let r = self.power()?;
            v = match op {
                '*' => v * r,
                '/' => v / r,
                _ => v % r,
            };
        }
        Some(v)
    }

    /// Right to left: 2^3^2 = 2^9.
    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.op("^").is_some() {
            return Some(base.powf(self.power()?));
        }
        Some(base)
    }

    fn unary(&mut self) -> Option<f64> {
        if self.op("-").is_some() {
            return Some(-self.unary()?);
        }
        if self.op("+").is_some() {
            return self.unary();
        }
        self.atom()
    }

    fn atom(&mut self) -> Option<f64> {
        match self.tokens.get(self.pos).cloned()? {
            Tok::Num(n) => {
                self.pos += 1;
                Some(n)
            }
            Tok::Op('(') => {
                self.pos += 1;
                let v = self.sum()?;
                self.op(")")?;
                Some(v)
            }
            Tok::Name(name) => {
                self.pos += 1;
                let f: fn(f64) -> f64 = match name.as_str() {
                    "sqrt" => f64::sqrt,
                    "sin" => f64::sin,
                    "cos" => f64::cos,
                    "tan" => f64::tan,
                    _ => return None,
                };
                self.op("(")?;
                let v = self.sum()?;
                self.op(")")?;
                Some(f(v))
            }
            Tok::Op(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::eval::tests::run;

    #[test]
    fn math() {
        assert_eq!(calc("2^3").map(number).as_deref(), Some("8"));
        assert_eq!(calc("(3+4) / 2").map(number).as_deref(), Some("3.5"));
        assert_eq!(calc("sqrt(81) * (1 + 4/2)").map(number).as_deref(), Some("27"));
        assert_eq!(calc("-2 * -3 % 4").map(number).as_deref(), Some("2"));
        assert_eq!(calc("1/0"), None);
        assert_eq!(calc("2 +"), None);
        assert_eq!(run(r"$calc(2^3) $calc( (3+4\) / 2)", "").unwrap(), "8 3.5");
    }
}
