//! V104-01 Regex flavors on one engine. Mentions run on `fancy-regex` (Rust's syntax plus
//! lookaround and backreferences); JavaScript and Python patterns are rewritten to it
//! first, where the two read the same text differently. What can't be carried over is an
//! error or a note, shown under the pattern.

use cyex_settings::schema::RegexFlavor;

/// A pattern ready for the engine.
#[derive(Debug, PartialEq, Eq)]
pub struct Translated {
    pub pattern: String,
    /// JavaScript's `/…/i`: ignore case whatever the switch says.
    pub ignore_case: bool,
    /// Something that works differently here than in the flavor.
    pub note: Option<&'static str>,
}

/// JavaScript's `\b` and `\B`: only ASCII letters, digits and `_` make words.
const JS_EDGE: &str = r"(?:(?<![0-9A-Za-z_])(?=[0-9A-Za-z_])|(?<=[0-9A-Za-z_])(?![0-9A-Za-z_]))";
const JS_NOT_EDGE: &str = r"(?:(?<=[0-9A-Za-z_])(?=[0-9A-Za-z_])|(?<![0-9A-Za-z_])(?![0-9A-Za-z_]))";

/// Characters the engine reads as syntax, so a literal one needs a backslash.
const META: &str = r"\.+*?()|[]{}^$#&-~";

pub fn translate(src: &str, flavor: RegexFlavor) -> Result<Translated, String> {
    match flavor {
        RegexFlavor::Rust => Ok(Translated {
            pattern: src.to_owned(),
            ignore_case: false,
            note: None,
        }),
        RegexFlavor::JavaScript => {
            let (body, flags) = js_literal(src);
            let mut t = Scan::new(body, flavor).run()?;
            let mut prefix = String::new();
            for f in flags.chars() {
                match f {
                    'i' => t.ignore_case = true,
                    'm' | 's' => {
                        prefix.push_str("(?");
                        prefix.push(f);
                        prefix.push(')');
                    }
                    // Global, sticky, indices, Unicode: nothing to do for a yes or no.
                    'g' | 'y' | 'd' | 'u' | 'v' => {}
                    other => return Err(format!("Unknown flag “{other}”.")),
                }
            }
            t.pattern.insert_str(0, &prefix);
            Ok(t)
        }
        RegexFlavor::Python => Scan::new(src, flavor).run(),
    }
}

/// `/body/flags` → (body, flags); anything else is all body.
fn js_literal(src: &str) -> (&str, &str) {
    if let Some(rest) = src.strip_prefix('/')
        && let Some(end) = rest.rfind('/')
        && rest[end + 1..].bytes().all(|b| b.is_ascii_alphabetic())
    {
        return (&rest[..end], &rest[end + 1..]);
    }
    (src, "")
}

struct Scan<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    js: bool,
    out: String,
    in_class: bool,
    note: Option<&'static str>,
}

impl<'a> Scan<'a> {
    fn new(src: &'a str, flavor: RegexFlavor) -> Self {
        Self {
            chars: src.chars().peekable(),
            js: flavor == RegexFlavor::JavaScript,
            out: String::with_capacity(src.len() + 8),
            in_class: false,
            note: None,
        }
    }

    fn run(mut self) -> Result<Translated, String> {
        while let Some(c) = self.chars.next() {
            match c {
                '\\' => self.escape()?,
                '[' if self.in_class => self.out.push_str(r"\["),
                '[' => self.open_class(),
                ']' if self.in_class => {
                    self.in_class = false;
                    self.out.push(']');
                }
                '&' | '~' if self.in_class => self.literal(c),
                '-' if self.in_class && self.chars.peek() == Some(&'-') => {
                    self.out.push('-');
                    self.chars.next();
                    self.out.push_str(r"\-");
                }
                '{' if self.in_class => self.literal(c),
                '{' => self.brace(),
                ']' | '}' => self.literal(c),
                '(' if !self.in_class && self.chars.peek() == Some(&'?') => self.group()?,
                c => self.out.push(c),
            }
        }
        Ok(Translated {
            pattern: self.out,
            ignore_case: false,
            note: self.note,
        })
    }

    fn literal(&mut self, c: char) {
        if c.is_whitespace() || c.is_control() {
            self.out.push_str(&format!(r"\x{{{:X}}}", c as u32));
        } else {
            if META.contains(c) {
                self.out.push('\\');
            }
            self.out.push(c);
        }
    }

    /// JavaScript's `[]` never matches and `[^]` matches anything; Python reads a `]`
    /// straight after the bracket as a member.
    fn open_class(&mut self) {
        let mut ahead = self.chars.clone();
        let negated = ahead.peek() == Some(&'^');
        if negated {
            ahead.next();
        }
        if ahead.peek() == Some(&']') {
            if self.js {
                self.out.push_str(if negated { r"[\s\S]" } else { r"[^\s\S]" });
                self.chars = ahead;
                self.chars.next();
                return;
            }
            self.out.push_str(if negated { r"[^\]" } else { r"[\]" });
            self.chars = ahead;
            self.chars.next();
            self.in_class = true;
            return;
        }
        self.out.push('[');
        if negated {
            self.out.push('^');
            self.chars.next();
        }
        self.in_class = true;
    }

    /// `{n}`, `{n,}` and `{n,m}` repeat; Python also takes `{,m}`. Any other brace is
    /// a plain character in both flavors.
    fn brace(&mut self) {
        let mut ahead = self.chars.clone();
        let mut body = String::new();
        let closed = loop {
            match ahead.next() {
                Some('}') => break true,
                Some(c) if c.is_ascii_digit() || c == ',' => body.push(c),
                _ => break false,
            }
        };
        let (min, max) = body.split_once(',').unwrap_or((&body, &body));
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        let python_open = !self.js && min.is_empty() && digits(max);
        let valid = closed
            && body.matches(',').count() <= 1
            && (digits(min) && (max.is_empty() || digits(max)) || python_open);
        if !valid {
            self.literal('{');
            return;
        }
        self.out.push('{');
        if python_open {
            self.out.push('0');
        }
        self.out.push_str(&body);
        self.out.push('}');
        self.chars = ahead;
    }

    /// `(?` constructs. Python's comments go, its inline flags keep what the engine has.
    fn group(&mut self) -> Result<(), String> {
        self.chars.next();
        if self.js {
            self.out.push_str("(?");
            return Ok(());
        }
        if self.chars.peek() == Some(&'#') {
            for c in self.chars.by_ref() {
                if c == ')' {
                    return Ok(());
                }
            }
            return Err("A (?# comment isn't closed.".into());
        }
        let mut ahead = self.chars.clone();
        let mut flags = String::new();
        let end = loop {
            match ahead.next() {
                Some(c @ ('a' | 'i' | 'L' | 'm' | 's' | 'u' | 'x' | '-')) => flags.push(c),
                Some(c @ (')' | ':')) => break Some(c),
                _ => break None,
            }
        };
        let Some(end) = end.filter(|_| !flags.is_empty()) else {
            self.out.push_str("(?");
            return Ok(());
        };
        if flags.contains('L') {
            return Err("Locale mode (?L) isn't supported.".into());
        }
        if flags.contains('a') {
            self.note =
                Some(r"ASCII mode (?a) isn't supported: \w, \d and \b also match other letters and digits.");
        }
        let kept: String = flags.chars().filter(|c| !matches!(c, 'a' | 'u')).collect();
        self.chars = ahead;
        let kept = kept.trim_end_matches('-');
        match (kept.is_empty(), end) {
            (true, ')') => {}
            (true, _) => self.out.push_str("(?:"),
            (false, end) => {
                self.out.push_str("(?");
                self.out.push_str(kept);
                self.out.push(end);
            }
        }
        Ok(())
    }

    fn escape(&mut self) -> Result<(), String> {
        let Some(e) = self.chars.next() else {
            return Err(r"Ends with a lone \.".into());
        };
        let class = self.in_class;
        match e {
            'd' | 'D' | 'w' | 'W' if self.js => self.out.push_str(match (e, class) {
                ('d', false) => "[0-9]",
                ('d', true) => "0-9",
                ('w', false) => "[0-9A-Za-z_]",
                ('w', true) => "0-9A-Za-z_",
                ('D', _) => "[^0-9]",
                _ => "[^0-9A-Za-z_]",
            }),
            'b' if class => self.out.push_str(r"\x08"),
            // The engine can't switch Unicode off inline, so JavaScript's ASCII word
            // edges are spelled out.
            'b' | 'B' if self.js => self.out.push_str(if e == 'b' { JS_EDGE } else { JS_NOT_EDGE }),
            'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'b' | 'B' | 'f' | 'n' | 'r' | 't' | 'v' => {
                self.out.push('\\');
                self.out.push(e);
            }
            'a' if !self.js => self.out.push_str(r"\x07"),
            '1'..='9' if !class => {
                self.out.push('\\');
                self.out.push(e);
            }
            '0' if !self.chars.peek().is_some_and(char::is_ascii_digit) => self.out.push_str(r"\x00"),
            'x' => {
                let hex = self.hex(2, 2)?;
                self.code(&hex)?;
            }
            'u' if self.js && self.chars.peek() == Some(&'{') => {
                self.chars.next();
                let hex: String = self.chars.by_ref().take_while(|c| *c != '}').collect();
                self.code(&hex)?;
            }
            'u' => {
                let hex = self.hex(4, 4)?;
                self.code(&hex)?;
            }
            'U' if !self.js => {
                let hex = self.hex(8, 8)?;
                self.code(&hex)?;
            }
            'c' if self.js => match self.chars.next() {
                Some(l) if l.is_ascii_alphabetic() => {
                    self.out
                        .push_str(&format!(r"\x{:02X}", l.to_ascii_uppercase() as u8 - b'@'));
                }
                _ => return Err(r"\c needs a letter after it.".into()),
            },
            'k' if self.js && !class => self.out.push_str(r"\k"),
            'p' | 'P' if self.js => {
                self.out.push('\\');
                self.out.push(e);
            }
            'A' | 'Z' if !self.js && !class => self.out.push_str(if e == 'A' { r"\A" } else { r"\z" }),
            'N' if !self.js => return Err(r"\N{…} names aren't supported. Use the character itself.".into()),
            'q' if self.js => return Err(r"\q{…} isn't supported.".into()),
            c if c.is_ascii_alphanumeric() => {
                let flavor = if self.js { "JavaScript" } else { "Python" };
                return Err(format!(r"\{c} isn't a {flavor} escape."));
            }
            c => self.literal(c),
        }
        Ok(())
    }

    fn hex(&mut self, min: usize, max: usize) -> Result<String, String> {
        let mut hex = String::new();
        while hex.len() < max
            && let Some(c) = self.chars.peek().filter(|c| c.is_ascii_hexdigit())
        {
            hex.push(*c);
            self.chars.next();
        }
        if hex.len() < min {
            return Err(format!("A \\x or \\u escape needs {min} hex digits."));
        }
        Ok(hex)
    }

    fn code(&mut self, hex: &str) -> Result<(), String> {
        let n = u32::from_str_radix(hex, 16).map_err(|_| "Not a hex number.".to_owned())?;
        if char::from_u32(n).is_none() {
            return Err(format!("U+{n:X} isn't a character."));
        }
        self.out.push_str(&format!(r"\x{{{n:X}}}"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RegexFlavor::{JavaScript as Js, Python as Py};

    fn t(src: &str, flavor: RegexFlavor) -> String {
        translate(src, flavor).unwrap().pattern
    }

    #[test]
    fn javascript() {
        assert_eq!(t(r"\d+\w\b", Js), format!("[0-9]+[0-9A-Za-z_]{JS_EDGE}"));
        assert_eq!(t(r"[\d\w-]", Js), r"[0-90-9A-Za-z_-]");
        assert_eq!(t(r"a{2,}b{x}c{,3}", Js), r"a{2,}b\{x\}c\{,3\}");
        assert_eq!(t(r"[^]x[]", Js), r"[\s\S]x[^\s\S]");
        assert_eq!(t(r"\u00e9\u{1F600}\cJ\/", Js), r"\x{E9}\x{1F600}\x0A/");
        assert_eq!(t(r"(?<n>a)\k<n>(?<=b)", Js), r"(?<n>a)\k<n>(?<=b)");
        let lit = translate("/cy(ex)?/gi", Js).unwrap();
        assert_eq!((lit.pattern.as_str(), lit.ignore_case), ("cy(ex)?", true));
        assert_eq!(t("/^a.b$/ms", Js), "(?m)(?s)^a.b$");
        assert!(translate("/a/q", Js).is_err());
        assert!(translate(r"\A", Js).is_err());
    }

    #[test]
    fn python() {
        assert_eq!(t(r"(?P<n>a)(?P=n)\Z", Py), r"(?P<n>a)(?P=n)\z");
        assert_eq!(t(r"a(?#note)b{,2}", Py), r"ab{0,2}");
        assert_eq!(t(r"[]a][a&&b]", Py), r"[\]a][a\&\&b]");
        assert_eq!(t(r"(?ai)x(?u:y)", Py), r"(?i)x(?:y)");
        assert!(translate(r"(?ai)x", Py).unwrap().note.is_some());
        assert!(translate(r"(?L)x", Py).is_err());
        assert!(translate(r"\N{EM DASH}", Py).is_err());
        assert_eq!(t(r"\d\w\b", Py), r"\d\w\b", "Unicode like Rust");
    }

    #[test]
    fn rewrites_compile() {
        let cases = [
            (r"[\d\w-]\D\W\b\B", Js),
            (r"a{x}b}c]", Js),
            (r"[^][]", Js),
            (r"é\u{1F600}\cJ\/\-\!\@", Js),
            (r"[a&&b~~c]", Js),
            (r"(?<=a+)b(?<!c)(?<n>x)\k<n>", Js),
            (r"[]a]\Z\A(?#x)(?i:a)(?ai)b", Py),
            (r"(?P<n>a)(?P=n)\U0001F600\a", Py),
        ];
        for (src, flavor) in cases {
            let p = t(src, flavor);
            assert!(fancy_regex::Regex::new(&p).is_ok(), "{src} → {p}");
        }
    }

    #[test]
    fn rust_is_as_written() {
        assert_eq!(t(r"(?i)\bcyex\b", RegexFlavor::Rust), r"(?i)\bcyex\b");
    }
}
