//! V110-A43 What an import loses to a list's cap. The lists are still imported (first rows
//! kept); the result says which were cut, one line per list.

use serde_json::Value;

use crate::Partition;
use crate::schema::{MAX_AUTOMATIONS, MAX_FILTERS, MAX_GLOBAL, MAX_LINES, MAX_NEVER, MAX_RULES, MAX_WORDS};

/// (key in the file, name in the line, cap). The caps mirror `ui/.../rules/caps.ts`.
fn lists(p: Partition) -> &'static [(&'static str, &'static str, usize)] {
    match p {
        Partition::Commands => &[
            ("lines", "commands", MAX_LINES),
            ("userMenu", "name menu entries", MAX_LINES),
            ("messageMenu", "message menu entries", MAX_LINES),
            ("channelMenu", "chat menu entries", MAX_LINES),
            ("usercardButtons", "usercard buttons", MAX_LINES),
            ("automations", "automations", MAX_AUTOMATIONS),
        ],
        Partition::Highlights => &[
            ("phrases", "phrases", MAX_WORDS),
            ("globalList", "global phrases", MAX_GLOBAL),
            ("users", "people", MAX_RULES),
            ("badges", "badge rules", MAX_RULES),
            ("apps", "app rules", MAX_RULES),
            ("ignores", "ignores", MAX_RULES),
            ("never", "never-highlight names", MAX_NEVER),
            ("muted", "silent chats", MAX_NEVER),
            ("filters", "filters", MAX_FILTERS),
        ],
        _ => &[],
    }
}

/// Row counts of the capped lists in a raw partition, in the order of [`lists`].
pub(crate) fn counts(p: Partition, raw: &Value) -> Vec<usize> {
    lists(p).iter().map(|(key, ..)| raw.get(key).and_then(Value::as_array).map_or(0, Vec::len)).collect()
}

/// "Kept the first 500 commands." for each list that came in longer than its cap and ended up
/// at it (`before` and `after` are [`counts`] of the file and of the saved partition).
pub(crate) fn lines(p: Partition, before: &[usize], after: &[usize]) -> Vec<String> {
    lists(p)
        .iter()
        .zip(before.iter().zip(after))
        .filter(|((_, _, cap), (b, a))| **b > *cap && **a >= *cap)
        .map(|((_, name, cap), _)| format!("Kept the first {} {name}.", group(*cap)))
        .collect()
}

/// V110-A-idea Lines for the import review: what a partition of a file or share code brings
/// ("12 commands", "3 sound files"). Lists that are empty are left out.
pub(crate) fn summary(p: Partition, raw: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for ((_, name, cap), n) in lists(p).iter().zip(counts(p, raw)) {
        // V110-A43 Over the cap: said before Apply, not only in the toast after it.
        if n > *cap {
            out.push(format!("{} {name}, only the first {} come in", group(n), group(*cap)));
        } else if n > 0 {
            out.push(format!("{} {}", group(n), if n == 1 { singular(name) } else { (*name).to_owned() }));
        }
    }
    let mut add = |n: usize, one: &str, many: &str| {
        if n > 0 {
            out.push(format!("{} {}", group(n), if n == 1 { one } else { many }));
        }
    };
    let len = |key: &str, object: bool| match raw.get(key) {
        Some(Value::Array(a)) if !object => a.len(),
        Some(Value::Object(o)) if object => o.len(),
        _ => 0,
    };
    match p {
        Partition::Commands => add(len("vars", true), "variable", "variables"),
        Partition::Appearance => add(len("themes", false), "theme", "themes"),
        Partition::Tabs => add(len("tabs", false), "tab", "tabs"),
        _ => {}
    }
    add(sound_files(raw), "sound file", "sound files");
    out
}

/// "commands" as "command", "entries" as "entry".
fn singular(name: &str) -> String {
    match name.strip_suffix("ies") {
        Some(stem) => format!("{stem}y"),
        None => name.strip_suffix('s').unwrap_or(name).to_owned(),
    }
}

/// Different non-empty `soundFile` values anywhere in `v`.
fn sound_files(v: &Value) -> usize {
    fn walk<'a>(v: &'a Value, found: &mut std::collections::BTreeSet<&'a str>) {
        match v {
            Value::Object(map) => {
                for (k, x) in map {
                    match x.as_str() {
                        Some(s) if k == "soundFile" && !s.is_empty() => {
                            found.insert(s);
                        }
                        _ => walk(x, found),
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, found)),
            _ => {}
        }
    }
    let mut found = std::collections::BTreeSet::new();
    walk(v, &mut found);
    found.len()
}

/// `1000` as `1.000`, the app's grouping.
fn group(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_list_over_its_cap_gets_a_line() {
        let rows: Vec<Value> = (0..MAX_LINES + 3).map(|i| json!(format!("/c{i} x"))).collect();
        let raw = json!({"lines": rows, "automations": []});
        let before = counts(Partition::Commands, &raw);
        let after = vec![MAX_LINES, 0, 0, 0, 0, 0];
        assert_eq!(lines(Partition::Commands, &before, &after), ["Kept the first 500 commands."]);
        // Cut down by something else (duplicates): no line.
        let after = vec![MAX_LINES - 5, 0, 0, 0, 0, 0];
        assert!(lines(Partition::Commands, &before, &after).is_empty());
    }

    #[test]
    fn the_review_counts_what_comes_in() {
        let raw = json!({
            "lines": ["/a x", "/b y"],
            "automations": [{"name": "one"}],
            "vars": {"a": "1", "b": "2", "c": "3"},
            "x": {"soundFile": "C:/a.wav", "y": [{"soundFile": "C:/a.wav"}, {"soundFile": "C:/b.mp3"}, {"soundFile": ""}]}
        });
        assert_eq!(
            summary(Partition::Commands, &raw),
            ["2 commands", "1 automation", "3 variables", "2 sound files"]
        );
        assert!(summary(Partition::General, &json!({})).is_empty());
    }

    #[test]
    fn the_review_says_a_list_over_its_cap_is_cut() {
        let raw = json!({ "lines": vec!["/a x"; MAX_LINES + 100], "automations": [{"name": "one"}] });
        assert_eq!(
            summary(Partition::Commands, &raw),
            [format!("{} commands, only the first {} come in", group(MAX_LINES + 100), group(MAX_LINES)), "1 automation".to_owned()]
        );
    }

    #[test]
    fn grouping() {
        assert_eq!(group(500), "500");
        assert_eq!(group(1000), "1.000");
    }
}
