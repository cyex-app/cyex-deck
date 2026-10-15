//! Reads a filter into a tree and checks its types once (Chatterino's grammar):
//! `||` < `&&` < `!` < one comparison < math, where math runs left to right.

use regex::{Regex, RegexBuilder};

use super::vars::{Type, Var};

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    Str(String),
    Regex(Regex),
    List(Vec<Expr>),
    Var(Var),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Contains,
    StartsWith,
    EndsWith,
    Match,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Op {
    fn comparison(word: &str) -> Option<Self> {
        Some(match word {
            "==" => Op::Eq,
            "!=" => Op::Ne,
            "<" => Op::Lt,
            ">" => Op::Gt,
            "<=" => Op::Le,
            ">=" => Op::Ge,
            "contains" => Op::Contains,
            "startswith" => Op::StartsWith,
            "endswith" => Op::EndsWith,
            "match" => Op::Match,
            _ => return None,
        })
    }

    fn math(word: &str) -> Option<Self> {
        Some(match word {
            "+" => Op::Add,
            "-" => Op::Sub,
            "*" => Op::Mul,
            "/" => Op::Div,
            "%" => Op::Mod,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Word(String),
    Int(i64),
    Str(String),
    Regex(String, bool),
    Sym(String),
}

fn lex(text: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let quote_at = match c {
            '"' => Some(i),
            'r' if chars.get(i + 1) == Some(&'"') => Some(i + 1),
            'r' if chars.get(i + 1) == Some(&'i') && chars.get(i + 2) == Some(&'"') => Some(i + 2),
            _ => None,
        };
        if let Some(q) = quote_at {
            let mut s = String::new();
            let mut j = q + 1;
            loop {
                match chars.get(j) {
                    None => return Err("A quote isn’t closed".into()),
                    Some('"') => break,
                    Some('\\') if chars.get(j + 1) == Some(&'"') => {
                        s.push('"');
                        j += 2;
                    }
                    Some(ch) => {
                        s.push(*ch);
                        j += 1;
                    }
                }
            }
            out.push(match q - i {
                0 => Tok::Str(s),
                1 => Tok::Regex(s, false),
                _ => Tok::Regex(s, true),
            });
            i = j + 1;
        } else if c.is_alphanumeric() || c == '_' || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            out.push(match word.parse::<i64>() {
                Ok(n) => Tok::Int(n),
                Err(_) => Tok::Word(word),
            });
        } else if "<>!=|&+-*/%".contains(c) {
            let start = i;
            while i < chars.len() && "<>!=|&+-*/%".contains(chars[i]) {
                i += 1;
            }
            let ops: String = chars[start..i].iter().collect();
            // `!!x` or `a==!b`: split a run into known operators, longest first.
            let mut rest = ops.as_str();
            while !rest.is_empty() {
                let two = rest
                    .get(..2)
                    .filter(|t| ["<=", ">=", "!=", "==", "||", "&&"].contains(t));
                let take = two.map_or(1, |_| 2);
                let sym = &rest[..take];
                if take == 1 && ["|", "&", "="].contains(&sym) {
                    return Err(format!("Unknown operator '{ops}'"));
                }
                out.push(Tok::Sym(sym.to_owned()));
                rest = &rest[take..];
            }
        } else if "(){},".contains(c) {
            out.push(Tok::Sym(c.to_string()));
            i += 1;
        } else {
            return Err(format!("Unexpected '{c}'"));
        }
    }
    Ok(out)
}

pub fn parse(text: &str) -> Result<Expr, String> {
    let mut p = Parser {
        toks: lex(text)?,
        pos: 0,
    };
    if p.toks.is_empty() {
        return Err("The filter is empty".into());
    }
    let e = p.or()?;
    if p.pos < p.toks.len() {
        return Err(format!("Unexpected {:?}", p.toks[p.pos]));
    }
    match type_of(&e)? {
        Type::Bool => Ok(e),
        t => Err(format!("The filter must end up true or false, not {}", t.name())),
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn sym(&mut self, s: &str) -> bool {
        let hit = matches!(self.toks.get(self.pos), Some(Tok::Sym(x)) if x == s);
        if hit {
            self.pos += 1;
        }
        hit
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut e = self.and()?;
        while self.sym("||") {
            e = Expr::Bin(Op::Or, Box::new(e), Box::new(self.and()?));
        }
        Ok(e)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut e = self.not()?;
        while self.sym("&&") {
            e = Expr::Bin(Op::And, Box::new(e), Box::new(self.not()?));
        }
        Ok(e)
    }

    fn not(&mut self) -> Result<Expr, String> {
        if self.sym("!") {
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let left = self.math()?;
        let op = match self.toks.get(self.pos) {
            Some(Tok::Sym(s) | Tok::Word(s)) => Op::comparison(s),
            _ => None,
        };
        let Some(op) = op else { return Ok(left) };
        self.pos += 1;
        Ok(Expr::Bin(op, Box::new(left), Box::new(self.math()?)))
    }

    fn math(&mut self) -> Result<Expr, String> {
        let mut e = self.value()?;
        loop {
            let op = match self.toks.get(self.pos) {
                Some(Tok::Sym(s)) => Op::math(s),
                _ => None,
            };
            let Some(op) = op else { return Ok(e) };
            self.pos += 1;
            e = Expr::Bin(op, Box::new(e), Box::new(self.value()?));
        }
    }

    fn value(&mut self) -> Result<Expr, String> {
        let Some(tok) = self.toks.get(self.pos).cloned() else {
            return Err("The filter ends too early".into());
        };
        self.pos += 1;
        Ok(match tok {
            Tok::Int(n) => Expr::Int(n),
            Tok::Str(s) => Expr::Str(s),
            Tok::Regex(s, ci) => Expr::Regex(
                RegexBuilder::new(&s)
                    .case_insensitive(ci)
                    .build()
                    .map_err(|e| format!("Bad regex: {e}"))?,
            ),
            Tok::Word(w) => Expr::Var(Var::from_name(&w).ok_or_else(|| format!("Unknown name '{w}'"))?),
            Tok::Sym(s) if s == "(" => {
                let e = self.or()?;
                if !self.sym(")") {
                    return Err("Missing ')'".into());
                }
                e
            }
            Tok::Sym(s) if s == "{" => {
                let mut items = Vec::new();
                if !self.sym("}") {
                    loop {
                        items.push(self.or()?);
                        if self.sym("}") {
                            break;
                        }
                        if !self.sym(",") {
                            return Err("Missing '}'".into());
                        }
                    }
                }
                Expr::List(items)
            }
            Tok::Sym(s) => return Err(format!("Unexpected '{s}'")),
        })
    }
}

/// The type an expression ends up as, or why it doesn't make sense.
pub fn type_of(e: &Expr) -> Result<Type, String> {
    use Type::*;
    Ok(match e {
        Expr::Int(_) => Int,
        Expr::Str(_) => Str,
        Expr::Regex(_) => Regex,
        Expr::List(items) => {
            for i in items {
                type_of(i)?;
            }
            List
        }
        Expr::Var(v) => v.ty(),
        Expr::Not(inner) => match type_of(inner)? {
            Bool => Bool,
            t => return Err(format!("! needs true or false, not {}", t.name())),
        },
        Expr::Bin(op, l, r) => {
            let (a, b) = (type_of(l)?, type_of(r)?);
            let bad = || format!("Can’t use {:?} on {} and {}", op, a.name(), b.name());
            match op {
                Op::Or | Op::And if a == Bool && b == Bool => Bool,
                Op::Eq | Op::Ne if a == b || (a == Str && b == Int) || (a == Int && b == Str) => Bool,
                Op::Lt | Op::Gt | Op::Le | Op::Ge if a == Int && b == Int => Bool,
                Op::Contains | Op::StartsWith | Op::EndsWith if (a == Str || a == List) && b != Regex => Bool,
                Op::Match if a == Str && b == Regex => Bool,
                Op::Match if a == Str && b == List => Str,
                Op::Add if a == Str || b == Str => Str,
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod if a == Int && b == Int => Int,
                _ => return Err(bad()),
            }
        }
    })
}
