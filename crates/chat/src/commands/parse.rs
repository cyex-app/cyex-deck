//! Reads a command body into a tree, the way Chatty's parser does: `$` starts a
//! replacement or function, `\` takes the next character as is, `$"…"` is literal text.

use std::fmt;

pub type Items = Vec<Item>;

/// What a replacement refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Id {
    /// `$2` is word 2; `$2-` is word 2 and everything after it. Counted from 1.
    Arg { from: usize, rest: bool },
    /// `$(chan)`, `$(msg-id)`, `$(_mine)` (a custom replacement). Lowercase.
    Name(String),
}

/// The first parameter of `$if`, `$join` and friends: an identifier, or text when it
/// starts with `$`.
#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Id(Id),
    Items(Items),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    Trim,
    Rand,
    RandNum,
    DateTime,
    UrlEncode,
    /// `$cs()` escapes for `/chain`, `$fs()` for `/foreach`.
    Cs,
    Fs,
    Sort,
    Replace,
    Is,
    Get,
    Calc,
    Round,
    Input,
    Request,
    Json,
    Quote,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Text(String),
    /// `$1`, `$(chan)`, `$(_custom,args)`. `req` (`$$`) cancels the command when empty.
    Repl {
        id: Id,
        args: Option<Items>,
        req: bool,
    },
    If {
        src: Src,
        then: Items,
        other: Option<Items>,
        req: bool,
    },
    IfEq {
        src: Src,
        cmp: Items,
        then: Items,
        other: Option<Items>,
        req: bool,
    },
    Switch {
        src: Src,
        cases: Vec<(Items, Items)>,
        default: Items,
        req: bool,
    },
    Join {
        src: Src,
        sep: Items,
        req: bool,
    },
    Lower {
        src: Src,
        req: bool,
    },
    Upper {
        src: Src,
        req: bool,
    },
    /// `$j(path, default, sub…)`; a sub prefixed `each:` runs on every element.
    J {
        path: Items,
        default: Option<Items>,
        subs: Vec<(Items, bool)>,
        req: bool,
    },
    Call {
        f: Func,
        args: Vec<Items>,
        req: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Character offset.
    pub at: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at {})", self.message, self.at + 1)
    }
}

type Result<T> = std::result::Result<T, ParseError>;

const QUOTES: [char; 3] = ['"', '`', '\''];

pub fn parse(text: &str) -> Result<Items> {
    let mut p = Parser {
        chars: text.chars().collect(),
        pos: 0,
    };
    let items = p.until(&[])?;
    Ok(items)
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn accept(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        if hit {
            self.pos += 1;
        }
        hit
    }

    fn accept_str(&mut self, s: &str) -> bool {
        let n = s.chars().count();
        let hit = self.chars.len() >= self.pos + n
            && self.chars[self.pos..self.pos + n].iter().copied().eq(s.chars());
        if hit {
            self.pos += n;
        }
        hit
    }

    fn expect(&mut self, c: char) -> Result<()> {
        if self.accept(c) {
            Ok(())
        } else {
            Err(self.error(format!("Expected '{c}'")))
        }
    }

    fn error(&self, message: String) -> ParseError {
        ParseError {
            message,
            at: self.pos,
        }
    }

    /// Reads items until one of `stop` (not consumed) or the end.
    fn until(&mut self, stop: &[char]) -> Result<Items> {
        let mut items = Items::new();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if stop.contains(&c) {
                break;
            }
            self.pos += 1;
            match c {
                '$' => match self.special()? {
                    Item::Text(t) => text.push_str(&t),
                    item => {
                        if !text.is_empty() {
                            items.push(Item::Text(std::mem::take(&mut text)));
                        }
                        items.push(item);
                    }
                },
                '\\' => {
                    if let Some(next) = self.peek() {
                        self.pos += 1;
                        text.push(next);
                    }
                }
                c => text.push(c),
            }
        }
        if !text.is_empty() {
            items.push(Item::Text(text));
        }
        Ok(items)
    }

    fn param(&mut self) -> Result<Items> {
        self.until(&[',', ')'])
    }

    /// The text of a `$` thing, right after the `$`.
    fn special(&mut self) -> Result<Item> {
        if let Some(q) = self.peek().filter(|c| QUOTES.contains(c)) {
            self.pos += 1;
            return Ok(Item::Text(self.literal(q)));
        }
        let req = self.accept('$');
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.pos += 1;
        }
        let name: String = self.chars[start..self.pos].iter().collect();
        if name.is_empty() {
            return self.replacement(req);
        }
        let item = match name.as_str() {
            "if" => {
                self.expect('(')?;
                let src = self.src()?;
                self.expect(',')?;
                let then = self.param()?;
                let other = if self.accept(',') {
                    Some(self.until(&[')'])?)
                } else {
                    None
                };
                Item::If {
                    src,
                    then,
                    other,
                    req,
                }
            }
            "ifeq" => {
                self.expect('(')?;
                let src = self.src()?;
                self.expect(',')?;
                let cmp = self.param()?;
                self.expect(',')?;
                let then = self.param()?;
                let other = if self.accept(',') {
                    Some(self.until(&[')'])?)
                } else {
                    None
                };
                Item::IfEq {
                    src,
                    cmp,
                    then,
                    other,
                    req,
                }
            }
            "switch" => self.switch(req)?,
            "join" => {
                self.expect('(')?;
                let src = self.src()?;
                self.expect(',')?;
                let sep = self.until(&[')'])?;
                if sep.is_empty() {
                    return Err(self.error("Expected a separator".into()));
                }
                Item::Join { src, sep, req }
            }
            "lower" | "upper" => {
                self.expect('(')?;
                let src = self.src()?;
                if name == "lower" {
                    Item::Lower { src, req }
                } else {
                    Item::Upper { src, req }
                }
            }
            "j" => {
                self.expect('(')?;
                let path = self.param()?;
                let default = if self.accept(',') {
                    Some(self.param()?)
                } else {
                    None
                };
                let mut subs = Vec::new();
                while self.accept(',') {
                    let each = self.accept_str("each:");
                    subs.push((self.param()?, each));
                }
                Item::J {
                    path,
                    default,
                    subs,
                    req,
                }
            }
            "replace" => {
                self.expect('(')?;
                let input = self.until(&[','])?;
                self.expect(',')?;
                let search = self.until(&[','])?;
                self.expect(',')?;
                let mut args = vec![input, search, self.param()?];
                if self.accept(',') {
                    args.push(self.param()?);
                }
                Item::Call {
                    f: Func::Replace,
                    args,
                    req,
                }
            }
            other => {
                let (f, min, max) = match other {
                    "trim" => (Func::Trim, 1, 1),
                    "rand" => (Func::Rand, 1, usize::MAX),
                    "randnum" => (Func::RandNum, 1, 2),
                    "datetime" => (Func::DateTime, 1, 4),
                    "urlencode" => (Func::UrlEncode, 1, 1),
                    "cs" => (Func::Cs, 1, 1),
                    "fs" => (Func::Fs, 1, 1),
                    "sort" => (Func::Sort, 1, 3),
                    "is" => (Func::Is, 1, 1),
                    "get" => (Func::Get, 1, 2),
                    "calc" => (Func::Calc, 1, 1),
                    "round" => (Func::Round, 1, 4),
                    "input" => (Func::Input, 1, 3),
                    "request" => (Func::Request, 1, usize::MAX),
                    "json" => (Func::Json, 2, 2),
                    "quote" => (Func::Quote, 1, 2),
                    _ => {
                        self.pos = start;
                        return Err(self.error(format!("Unknown function '{other}'")));
                    }
                };
                self.expect('(')?;
                let mut args = vec![self.param()?];
                while args.len() < max && self.accept(',') {
                    args.push(self.param()?);
                }
                if args.len() < min {
                    return Err(self.error(format!("${other} needs {min} parameters")));
                }
                Item::Call { f, args, req }
            }
        };
        self.expect(')')?;
        Ok(item)
    }

    fn switch(&mut self, req: bool) -> Result<Item> {
        self.expect('(')?;
        let src = self.src()?;
        self.expect(',')?;
        let mut cases = Vec::new();
        let mut default = Items::new();
        loop {
            let key = self.until(&[',', ')', ':'])?;
            if self.accept(':') {
                cases.push((key, self.param()?));
            } else {
                default = key;
                break;
            }
            if !self.accept(',') {
                break;
            }
        }
        if cases.is_empty() {
            return Err(self.error("$switch needs a case".into()));
        }
        Ok(Item::Switch {
            src,
            cases,
            default,
            req,
        })
    }

    fn src(&mut self) -> Result<Src> {
        if self.peek() == Some('$') {
            Ok(Src::Items(self.param()?))
        } else {
            Ok(Src::Id(self.identifier()?))
        }
    }

    fn identifier(&mut self) -> Result<Id> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            self.pos += 1;
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        if word.is_empty() {
            return Err(self.error("Expected a name or number".into()));
        }
        let digits = word.trim_end_matches('-');
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && word.len() - digits.len() <= 1
        {
            let from: usize = digits.parse().map_err(|_| self.error("Number too big".into()))?;
            if from == 0 {
                return Err(self.error("Parameters start at 1".into()));
            }
            return Ok(Id::Arg {
                from,
                rest: word.ends_with('-'),
            });
        }
        Ok(Id::Name(word.to_ascii_lowercase()))
    }

    fn replacement(&mut self, req: bool) -> Result<Item> {
        if self.accept('(') {
            let id = self.identifier()?;
            let args = if self.accept(',') {
                Some(self.param()?)
            } else {
                None
            };
            self.expect(')')?;
            return Ok(Item::Repl { id, args, req });
        }
        match self.peek() {
            Some(c @ '1'..='9') => {
                self.pos += 1;
                let rest = self.accept('-');
                Ok(Item::Repl {
                    id: Id::Arg {
                        from: c as usize - '0' as usize,
                        rest,
                    },
                    args: None,
                    req,
                })
            }
            _ => Err(self.error("Expected a number after $".into())),
        }
    }

    /// `$"…"`: a doubled quote is one quote.
    fn literal(&mut self, q: char) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == q {
                if self.accept(q) {
                    out.push(q);
                } else {
                    break;
                }
            } else {
                out.push(c);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg(from: usize, rest: bool, req: bool) -> Item {
        Item::Repl {
            id: Id::Arg { from, rest },
            args: None,
            req,
        }
    }

    #[test]
    fn replacements_and_text() {
        let items = parse("/me slaps $$1 with $2-!").unwrap();
        assert_eq!(
            items,
            [
                Item::Text("/me slaps ".into()),
                arg(1, false, true),
                Item::Text(" with ".into()),
                arg(2, true, false),
                Item::Text("!".into()),
            ]
        );
        assert_eq!(parse("$(10-)").unwrap(), [arg(10, true, false)]);
        assert_eq!(
            parse("$(Chan)").unwrap(),
            [Item::Repl {
                id: Id::Name("chan".into()),
                args: None,
                req: false
            }]
        );
    }

    #[test]
    fn escapes_and_literals() {
        assert_eq!(
            parse(r"cost \$5 \\ ok").unwrap(),
            [Item::Text(r"cost $5 \ ok".into())]
        );
        assert_eq!(
            parse(r#"a $"b $1 ""c""" d"#).unwrap(),
            [Item::Text(r#"a b $1 "c" d"#.into())]
        );
    }

    #[test]
    fn functions_nest() {
        let items = parse(r"$if(2,$2,trout) $switch(1,a:x,b:y,z) $replace($1-,(\\w+),[\$1],regRef)").unwrap();
        assert!(matches!(
            &items[0],
            Item::If {
                src: Src::Id(Id::Arg { from: 2, .. }),
                other: Some(_),
                ..
            }
        ));
        let Item::Switch { cases, default, .. } = &items[2] else {
            panic!()
        };
        assert_eq!(cases.len(), 2);
        assert_eq!(default, &[Item::Text("z".into())]);
        let Item::Call {
            f: Func::Replace,
            args,
            ..
        } = &items[4]
        else {
            panic!()
        };
        assert_eq!(args[1], [Item::Text(r"(\w+)".into())]);
        assert_eq!(args[2], [Item::Text("[$1]".into())]);
        assert_eq!(args[3], [Item::Text("regRef".into())]);
    }

    #[test]
    fn j_subs() {
        let items = parse("$j(authors,,each:$(key) $j(name),$j([join]))").unwrap();
        let Item::J { subs, default, .. } = &items[0] else {
            panic!()
        };
        assert_eq!(default.as_deref(), Some(&[][..]));
        assert_eq!(subs.len(), 2);
        assert!(subs[0].1 && !subs[1].1);
    }

    #[test]
    fn errors() {
        assert!(parse("$nope(1)").is_err());
        assert!(parse("$if(1,x").is_err());
        assert!(parse("$0").is_err());
        assert!(parse("$switch(1,x)").is_err());
    }
}
