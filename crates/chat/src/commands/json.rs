//! `$json(input, output)` and `$j(path, default, sub…)`: keys joined by `->`, and
//! brackets for `[0]`, `[last]`, `[size]`, `[sort]`, `[unique]`, `[join:…]` and
//! `[filter:…]`, `[collect:…]`, `[combine:…]` with an optional `=regex`.

use std::sync::Arc;

use regex::Regex;
use serde_json::Value;

use super::eval::{Host, Params, eval, json_of};
use super::parse::Items;

pub fn path_item(
    path: &Items,
    default: Option<&Items>,
    subs: &[(Items, bool)],
    p: &Params,
    h: &mut dyn Host,
) -> Option<String> {
    let path = eval(path, p, h)?;
    let data = json_of(p)?.clone();
    let steps = parse_path(&path)?;
    let mut result = resolve(&steps, &data);
    for (items, each) in subs {
        let current = result.take().unwrap_or(Value::Null);
        result = Some(if *each {
            let mut out = Vec::new();
            match &current {
                Value::Object(map) => {
                    for (k, v) in map {
                        let mut sub = p.with_json(Arc::new(v.clone()));
                        sub.set("key", k.clone());
                        out.push(Value::String(eval(items, &sub, h)?));
                    }
                }
                Value::Array(list) => {
                    for (i, v) in list.iter().enumerate() {
                        let mut sub = p.with_json(Arc::new(v.clone()));
                        sub.set("index", i.to_string());
                        out.push(Value::String(eval(items, &sub, h)?));
                    }
                }
                _ => {}
            }
            Value::Array(out)
        } else {
            Value::String(eval(items, &p.with_json(Arc::new(current)), h)?)
        });
    }
    let mut text = result.as_ref().map(text_of).unwrap_or_default();
    if text.is_empty()
        && let Some(d) = default
    {
        text = eval(d, p, h)?;
    }
    Some(text)
}

/// Strings plain, everything else as JSON.
fn text_of(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[derive(Debug)]
enum Step {
    Key(String),
    Index(Option<usize>),
    Size,
    Sort,
    Unique,
    Join(String),
    Pick {
        kind: Pick,
        path: Vec<Step>,
        matching: Option<Regex>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pick {
    Filter,
    Collect,
    Combine,
}

fn resolve(steps: &[Step], data: &Value) -> Option<Value> {
    let mut cur = data.clone();
    for step in steps {
        cur = match step {
            Step::Key(k) => cur.as_object()?.get(k)?.clone(),
            Step::Index(i) => {
                let list = cur.as_array()?;
                list.get(i.unwrap_or(list.len().checked_sub(1)?))?.clone()
            }
            Step::Size => Value::from(match &cur {
                Value::Array(a) => a.len(),
                Value::Object(o) => o.len(),
                Value::String(s) => s.chars().count(),
                _ => return None,
            }),
            Step::Sort => {
                let mut list = cur.as_array()?.clone();
                if list.iter().all(Value::is_string) {
                    list.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                } else if list.iter().all(Value::is_number) {
                    list.sort_by(|a, b| {
                        a.as_f64()
                            .partial_cmp(&b.as_f64())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                } else {
                    return None;
                }
                Value::Array(list)
            }
            Step::Unique => {
                let mut out: Vec<Value> = Vec::new();
                for v in cur.as_array()? {
                    if !out.contains(v) {
                        out.push(v.clone());
                    }
                }
                Value::Array(out)
            }
            Step::Join(sep) => {
                Value::String(cur.as_array()?.iter().map(text_of).collect::<Vec<_>>().join(sep))
            }
            Step::Pick { kind, path, matching } => {
                let items: Vec<&Value> = match &cur {
                    Value::Array(a) => a.iter().collect(),
                    Value::Object(o) => o.values().collect(),
                    _ => return None,
                };
                let fits = |v: &Value| matching.as_ref().is_none_or(|re| re.is_match(&text_of(v)));
                let mut out = Vec::new();
                for item in items {
                    let Some(value) = resolve(path, item) else {
                        continue;
                    };
                    match kind {
                        Pick::Filter if fits(&value) => out.push(item.clone()),
                        Pick::Collect if fits(&value) => out.push(value),
                        Pick::Combine => {
                            if let Value::Array(inner) = value {
                                out.extend(inner.into_iter().filter(|v| !v.is_null() && fits(v)));
                            }
                        }
                        _ => {}
                    }
                }
                Value::Array(out)
            }
        };
    }
    Some(cur)
}

fn parse_path(text: &str) -> Option<Vec<Step>> {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let steps = path_until(&chars, &mut pos, &[])?;
    (pos == chars.len()).then_some(steps)
}

fn path_until(chars: &[char], pos: &mut usize, stop: &[char]) -> Option<Vec<Step>> {
    let mut steps = Vec::new();
    let mut key = String::new();
    let finish = |key: &mut String, steps: &mut Vec<Step>| {
        if !key.is_empty() {
            steps.push(Step::Key(std::mem::take(key)));
        }
    };
    while let Some(&c) = chars.get(*pos) {
        if stop.contains(&c) {
            break;
        }
        if c == '[' {
            *pos += 1;
            finish(&mut key, &mut steps);
            steps.push(bracket(chars, pos)?);
        } else if c == '-' && chars.get(*pos + 1) == Some(&'>') {
            *pos += 2;
            finish(&mut key, &mut steps);
        } else {
            key.push(c);
            *pos += 1;
        }
    }
    finish(&mut key, &mut steps);
    Some(steps)
}

fn bracket(chars: &[char], pos: &mut usize) -> Option<Step> {
    let read = |pos: &mut usize, f: fn(char) -> bool| {
        let start = *pos;
        while chars.get(*pos).is_some_and(|c| f(*c)) {
            *pos += 1;
        }
        chars[start..*pos].iter().collect::<String>()
    };
    let digits = read(pos, |c| c.is_ascii_digit());
    let step = if !digits.is_empty() {
        Step::Index(Some(digits.parse().ok()?))
    } else {
        match read(pos, |c| c.is_ascii_lowercase()).as_str() {
            "last" => Step::Index(None),
            "size" => Step::Size,
            "sort" => Step::Sort,
            "unique" => Step::Unique,
            "join" => Step::Join(if accept(chars, pos, ':') {
                param(chars, pos)
            } else {
                ", ".into()
            }),
            kind @ ("filter" | "collect" | "combine") => {
                if !accept(chars, pos, ':') {
                    return None;
                }
                let path = path_until(chars, pos, &[']', '='])?;
                let matching = if accept(chars, pos, '=') {
                    Some(Regex::new(&format!("^(?:{})$", param(chars, pos))).ok()?)
                } else {
                    None
                };
                let kind = match kind {
                    "filter" => Pick::Filter,
                    "collect" => Pick::Collect,
                    _ => Pick::Combine,
                };
                Step::Pick { kind, path, matching }
            }
            _ => return None,
        }
    };
    accept(chars, pos, ']').then_some(step)
}

fn accept(chars: &[char], pos: &mut usize, c: char) -> bool {
    let hit = chars.get(*pos) == Some(&c);
    if hit {
        *pos += 1;
    }
    hit
}

/// Up to `]`, or quoted with `'` (doubled inside).
fn param(chars: &[char], pos: &mut usize) -> String {
    let mut out = String::new();
    if accept(chars, pos, '\'') {
        while let Some(&c) = chars.get(*pos) {
            *pos += 1;
            if c == '\'' {
                if accept(chars, pos, '\'') {
                    out.push('\'');
                } else {
                    break;
                }
            } else {
                out.push(c);
            }
        }
        return out;
    }
    while let Some(&c) = chars.get(*pos).filter(|c| **c != ']') {
        out.push(c);
        *pos += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::eval::tests::TestHost;
    use super::super::eval::{Params, eval};
    use super::super::parse::parse;

    const BOOKS: &str = r#"{"books":[{"title":"book1","author":"author1","tags":["tag1","tag2"]},{"title":"book2","author":"author2","tags":["tag1"]},{"title":"book3","author":"author2","tags":["tag1","tag3"]},{"title":"book4","author":"author2"}],"authors":{"author1":{"name":"name1","age":24},"author2":{"name":"name2","age":62}},"numBooks":4}"#;

    fn j(output: &str) -> String {
        let mut h = TestHost::default();
        h.vars.insert("j".into(), BOOKS.into());
        let body = format!("$json($get(var,j),{output})");
        eval(&parse(&body).unwrap(), &Params::new(""), &mut h).unwrap()
    }

    #[test]
    fn paths() {
        assert_eq!(
            j("There are $j(numBooks) books of $j(books->[collect:author][unique][size]) authors."),
            "There are 4 books of 2 authors."
        );
        assert_eq!(
            j("$j(books[last]->title) by $j(books[last]->author)"),
            "book4 by author2"
        );
        assert_eq!(j("$j(books[10]->title,Book not found)"), "Book not found");
        assert_eq!(
            j("$j(books[filter:author=author2][collect:title])"),
            r#"["book2","book3","book4"]"#
        );
        assert_eq!(
            j("$j(books[filter:author=author2][combine:tags][unique][join])"),
            "tag1, tag3"
        );
        assert_eq!(j("$j(authors[collect:name][sort][join])"), "name1, name2");
        assert_eq!(
            j(r"$j(authors,,each:$(key): $j(name) \(age $j(age)\),$j([sort][join]))"),
            "author1: name1 (age 24), author2: name2 (age 62)"
        );
        assert_eq!(j("$j(books[0]->tags[join:'/'])"), "tag1/tag2");
    }
}
