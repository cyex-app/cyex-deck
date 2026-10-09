//! Markdown as channel panels use it (1.21): headings, paragraphs, lists, quotes, rules,
//! bold, italic, code, links and images. Every line break counts, like on Twitch. Turned
//! into blocks here, so the UI never renders HTML. Only http(s) links and images survive.

use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum Block {
    Heading { level: u8, spans: Vec<Span> },
    Paragraph { spans: Vec<Span> },
    List { ordered: bool, items: Vec<Vec<Span>> },
    Quote { spans: Vec<Span> },
    Rule,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum Span {
    Text { text: String, bold: bool, italic: bool },
    Code { text: String },
    Link { text: String, url: String },
    Image { url: String, alt: String },
    Break,
}

pub fn parse(src: &str) -> Vec<Block> {
    let src = src.replace("\r\n", "\n").replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    let mut b = Blocks::default();
    for raw in src.lines() {
        let line = raw.trim_end();
        let t = line.trim_start();
        if t.is_empty() {
            b.flush();
        } else if let Some((level, text)) = heading(t) {
            b.flush();
            b.out.push(Block::Heading {
                level,
                spans: inline(text),
            });
        } else if is_rule(t) {
            b.flush();
            b.out.push(Block::Rule);
        } else if let Some(rest) = t.strip_prefix('>') {
            b.flush_except(Open::Quote);
            b.quote.push(rest.strip_prefix(' ').unwrap_or(rest).to_owned());
        } else if let Some((ordered, item)) = list_item(t).filter(|_| line.len() - t.len() < 4) {
            b.flush_except(Open::List);
            match &mut b.list {
                Some((o, items)) if *o == ordered => items.push(vec![item.to_owned()]),
                _ => {
                    b.flush();
                    b.list = Some((ordered, vec![vec![item.to_owned()]]));
                }
            }
        } else if let Some((_, items)) = b.list.as_mut().filter(|_| line.starts_with("  ")) {
            if let Some(last) = items.last_mut() {
                last.push(t.to_owned());
            }
        } else {
            b.flush_except(Open::Paragraph);
            b.para.push(t.to_owned());
        }
    }
    b.flush();
    b.out
}

#[derive(Default)]
struct Blocks {
    out: Vec<Block>,
    para: Vec<String>,
    quote: Vec<String>,
    /// Ordered, then each item's lines.
    list: Option<(bool, Vec<Vec<String>>)>,
}

#[derive(PartialEq)]
enum Open {
    Paragraph,
    Quote,
    List,
}

impl Blocks {
    fn flush(&mut self) {
        self.flush_except(Open::Paragraph);
        if !self.para.is_empty() {
            let spans = lines(&std::mem::take(&mut self.para));
            self.out.push(Block::Paragraph { spans });
        }
    }

    /// Ends every open block but `keep`.
    fn flush_except(&mut self, keep: Open) {
        if keep != Open::Paragraph && !self.para.is_empty() {
            let spans = lines(&std::mem::take(&mut self.para));
            self.out.push(Block::Paragraph { spans });
        }
        if keep != Open::Quote && !self.quote.is_empty() {
            let spans = lines(&std::mem::take(&mut self.quote));
            self.out.push(Block::Quote { spans });
        }
        if keep != Open::List
            && let Some((ordered, items)) = self.list.take()
        {
            let items = items.iter().map(|l| lines(l)).collect();
            self.out.push(Block::List { ordered, items });
        }
    }
}

/// Lines with a break between each.
fn lines(lines: &[String]) -> Vec<Span> {
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if i > 0 {
            out.push(Span::Break);
        }
        out.extend(inline(l));
    }
    out
}

fn heading(t: &str) -> Option<(u8, &str)> {
    let level = t.bytes().take_while(|&c| c == b'#').count();
    let rest = &t[level..];
    if !(1..=6).contains(&level) || !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    Some((level as u8, rest.trim().trim_end_matches('#').trim_end()))
}

fn is_rule(t: &str) -> bool {
    let mut marks = t.chars().filter(|c| !c.is_whitespace());
    let Some(first) = marks.next() else { return false };
    matches!(first, '-' | '*' | '_') && marks.clone().count() >= 2 && marks.all(|c| c == first)
}

fn list_item(t: &str) -> Option<(bool, &str)> {
    if let Some(rest) = t.strip_prefix(['-', '*', '+'])
        && let Some(item) = rest.strip_prefix(' ')
    {
        return Some((false, item.trim_start()));
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    let rest = &t[digits..];
    if (1..10).contains(&digits)
        && let Some(item) = rest.strip_prefix(['.', ')']).and_then(|r| r.strip_prefix(' '))
    {
        return Some((true, item.trim_start()));
    }
    None
}

/// Inline marks in one line.
pub fn inline(s: &str) -> Vec<Span> {
    let c: Vec<char> = s.chars().collect();
    let mut p = Inline::default();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '\\' if c.get(i + 1).is_some_and(char::is_ascii_punctuation) => {
                p.text.push(c[i + 1]);
                i += 2;
            }
            '`' if let Some(end) = find(&c, i + 1, '`') => {
                p.flush();
                p.out.push(Span::Code {
                    text: c[i + 1..end].iter().collect(),
                });
                i = end + 1;
            }
            '!' if c.get(i + 1) == Some(&'[') && let Some((alt, url, end)) = bracket(&c, i + 1) => {
                p.flush();
                p.out.push(match web(&url) {
                    Some(url) => Span::Image { url, alt },
                    None => text_span(alt),
                });
                i = end;
            }
            '[' if let Some((text, url, end)) = bracket(&c, i) => {
                p.flush();
                let text = plain(&text);
                p.out.push(match web(&url) {
                    Some(url) => Span::Link { text, url },
                    None => text_span(text),
                });
                i = end;
            }
            m @ ('*' | '_') => i = p.emphasis(&c, i, m),
            'h' if (i == 0 || !c[i - 1].is_alphanumeric()) && let Some(end) = bare_url(&c, i) => {
                p.flush();
                let url: String = c[i..end].iter().collect();
                p.out.push(Span::Link { text: url.clone(), url });
                i = end;
            }
            ch => {
                p.text.push(ch);
                i += 1;
            }
        }
    }
    p.flush();
    p.out
}

#[derive(Default)]
struct Inline {
    out: Vec<Span>,
    text: String,
    bold: bool,
    italic: bool,
}

impl Inline {
    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.out.push(Span::Text {
                text: std::mem::take(&mut self.text),
                bold: self.bold,
                italic: self.italic,
            });
        }
    }

    /// A run of `*` or `_` at `i`: one toggles italic, two bold, three both, when the
    /// other end is there. Returns where to go on.
    fn emphasis(&mut self, c: &[char], i: usize, m: char) -> usize {
        let run = c[i..].iter().take_while(|&&x| x == m).count();
        let end = i + run;
        let before = i.checked_sub(1).map(|j| c[j]);
        let after = c.get(end).copied();
        // `_` inside a word (snake_case) is just text.
        let word = m == '_' && before.is_some_and(char::is_alphanumeric) && after.is_some_and(char::is_alphanumeric);
        let (bold, italic) = match run {
            1 => (false, true),
            2 => (true, false),
            3 => (true, true),
            _ => (false, false),
        };
        let closing = (bold && self.bold || italic && self.italic) && before.is_some_and(|b| !b.is_whitespace());
        let opening = !closing
            && (bold || italic)
            && after.is_some_and(|a| !a.is_whitespace())
            && closer_after(c, end, m, run);
        if word || !(closing || opening) {
            self.text.extend(&c[i..end]);
            return end;
        }
        self.flush();
        if bold {
            self.bold = !self.bold;
        }
        if italic {
            self.italic = !self.italic;
        }
        end
    }
}

fn closer_after(c: &[char], from: usize, m: char, run: usize) -> bool {
    let mut i = from;
    while i < c.len() {
        if c[i] == m {
            let len = c[i..].iter().take_while(|&&x| x == m).count();
            if len == run && !c[i - 1].is_whitespace() {
                return true;
            }
            i += len;
        } else {
            i += 1;
        }
    }
    false
}

fn find(c: &[char], from: usize, ch: char) -> Option<usize> {
    c[from..].iter().position(|&x| x == ch).map(|p| from + p)
}

/// `[text](url)` starting at `i`: text, url and the index after it.
fn bracket(c: &[char], i: usize) -> Option<(String, String, usize)> {
    let close = find(c, i + 1, ']')?;
    if c.get(close + 1) != Some(&'(') {
        return None;
    }
    let mut depth = 0;
    let mut end = None;
    for (j, &ch) in c.iter().enumerate().skip(close + 2) {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => {
                end = Some(j);
                break;
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    let end = end?;
    let target: String = c[close + 2..end].iter().collect();
    // Drop a title: `(url "title")`.
    let url = target.split_whitespace().next().unwrap_or("").trim_matches(['<', '>']).to_owned();
    Some((c[i + 1..close].iter().collect(), url, end + 1))
}

fn bare_url(c: &[char], i: usize) -> Option<usize> {
    let rest: String = c[i..c.len().min(i + 8)].iter().collect();
    if !(rest.starts_with("https://") || rest.starts_with("http://")) {
        return None;
    }
    let mut end = c[i..].iter().position(|ch| ch.is_whitespace()).map_or(c.len(), |p| i + p);
    while end > i && matches!(c[end - 1], '.' | ',' | ':' | ';' | '!' | '?' | '"' | '\'' | ')' | '*' | '_') {
        let opens = c[i..end].iter().filter(|&&x| x == '(').count();
        let closes = c[i..end].iter().filter(|&&x| x == ')').count();
        if c[end - 1] == ')' && closes <= opens {
            break;
        }
        end -= 1;
    }
    (end > i + 8).then_some(end)
}

fn web(url: &str) -> Option<String> {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")).then(|| url.to_owned())
}

fn plain(s: &str) -> String {
    inline(s)
        .into_iter()
        .map(|s| match s {
            Span::Text { text, .. } | Span::Code { text } | Span::Link { text, .. } => text,
            Span::Image { alt, .. } => alt,
            Span::Break => " ".into(),
        })
        .collect()
}

fn text_span(text: String) -> Span {
    Span::Text {
        text,
        bold: false,
        italic: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &str) -> Span {
        text_span(text.into())
    }

    fn b(text: &str) -> Span {
        Span::Text {
            text: text.into(),
            bold: true,
            italic: false,
        }
    }

    fn i(text: &str) -> Span {
        Span::Text {
            text: text.into(),
            bold: false,
            italic: true,
        }
    }

    #[test]
    fn panel_specs_keep_their_lines() {
        let blocks = parse("**CPU:** Intel i9\n**RAM:** 64 GB\n\nThanks!");
        assert_eq!(
            blocks,
            [
                Block::Paragraph {
                    spans: vec![b("CPU:"), t(" Intel i9"), Span::Break, b("RAM:"), t(" 64 GB")]
                },
                Block::Paragraph { spans: vec![t("Thanks!")] },
            ]
        );
    }

    #[test]
    fn headings_lists_quotes_rules() {
        let blocks = parse("# Rules ##\n- be nice\n- no spam\n  really\n1. one\n2) two\n> quoted\n---\n####### no");
        assert_eq!(blocks.len(), 6);
        assert_eq!(blocks[0], Block::Heading { level: 1, spans: vec![t("Rules")] });
        assert_eq!(
            blocks[1],
            Block::List {
                ordered: false,
                items: vec![vec![t("be nice")], vec![t("no spam"), Span::Break, t("really")]]
            }
        );
        assert!(matches!(&blocks[2], Block::List { ordered: true, items } if items.len() == 2));
        assert_eq!(blocks[3], Block::Quote { spans: vec![t("quoted")] });
        assert_eq!(blocks[4], Block::Rule);
        assert_eq!(blocks[5], Block::Paragraph { spans: vec![t("####### no")] });
    }

    #[test]
    fn links_and_images_only_over_http() {
        let spans = inline("[**Shop**](https://a.com/x_(y) \"t\") ![logo](https://i.com/l.png) [bad](javascript:alert(1))");
        assert_eq!(
            spans,
            [
                Span::Link { text: "Shop".into(), url: "https://a.com/x_(y)".into() },
                t(" "),
                Span::Image { url: "https://i.com/l.png".into(), alt: "logo".into() },
                t(" "),
                t("bad"),
            ]
        );
        assert_eq!(
            inline("see https://x.com/a_b. ok"),
            [t("see "), Span::Link { text: "https://x.com/a_b".into(), url: "https://x.com/a_b".into() }, t(". ok")]
        );
    }

    #[test]
    fn emphasis_needs_both_ends() {
        assert_eq!(inline("*hi* and _there_"), [i("hi"), t(" and "), i("there")]);
        assert_eq!(inline("5 * 3 = 15"), [t("5 * 3 = 15")]);
        assert_eq!(inline("snake_case_name"), [t("snake_case_name")]);
        assert_eq!(inline("**open"), [t("**open")]);
        assert_eq!(
            inline("***both*** `co*de` \\*lit\\*"),
            [
                Span::Text { text: "both".into(), bold: true, italic: true },
                t(" "),
                Span::Code { text: "co*de".into() },
                t(" *lit*"),
            ]
        );
    }

    #[test]
    fn html_breaks_become_lines() {
        assert_eq!(parse("a<br>b"), [Block::Paragraph { spans: vec![t("a"), Span::Break, t("b")] }]);
    }
}
