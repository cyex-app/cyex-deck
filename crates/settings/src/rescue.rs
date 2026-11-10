//! V110-A44 A `tabs.json` that was cut short (a crash or a full disk mid-write) still holds most
//! of its tabs. Every complete tab that can be read is kept before the start falls back to the
//! newest daily backup.

use serde_json::Value;

use crate::Partition;

/// The complete elements of the top-level array `key` in `text`, in order. Stops at the first
/// one that is cut short or isn't JSON. Empty when the key isn't found.
fn elements(text: &str, key: &str) -> Vec<Value> {
    let Some(start) = array_start(text, key) else { return Vec::new() };
    let mut out = Vec::new();
    let mut rest = &text[start..];
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        if rest.is_empty() || rest.starts_with(']') {
            break;
        }
        let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<Value>();
        match stream.next() {
            Some(Ok(v)) => {
                let used = stream.byte_offset();
                out.push(v);
                rest = &rest[used..];
            }
            _ => break,
        }
    }
    out
}

/// Byte offset just after the `[` of the top-level member `key`.
fn array_start(text: &str, key: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            b'"' => {
                let begin = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                if depth == 1 && text.get(begin..i) == Some(key) {
                    let after = text.get(i + 1..)?;
                    let trimmed = after.trim_start();
                    if let Some(value) = trimmed.strip_prefix(':') {
                        let value = value.trim_start();
                        if value.starts_with('[') {
                            return Some(text.len() - value.len() + 1);
                        }
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The tabs (and tab groups) of a cut-off `tabs.json`, normalized, and how many tabs were kept.
/// `None` when no tab can be read.
pub(crate) fn tabs(text: &str) -> Option<(Value, usize)> {
    let p = Partition::Tabs;
    let readable = |key: &str| -> Vec<Value> {
        elements(text, key)
            .into_iter()
            .filter(|v| p.normalize(serde_json::json!({ key: [v] })).is_ok())
            .collect()
    };
    let tabs = readable("tabs");
    if tabs.is_empty() {
        return None;
    }
    let kept = tabs.len();
    let groups = readable("groups");
    let value = p.normalize(serde_json::json!({"tabs": tabs, "groups": groups})).ok()?;
    Some((value, kept))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tab(id: &str) -> Value {
        json!({"id": id, "columns": [{"size": 1.0, "tiles": [{"id": format!("t{id}"), "channel": {"id": "1", "login": "a", "name": "A"}}]}]})
    }

    #[test]
    fn a_cut_off_file_keeps_every_complete_tab() {
        let full = serde_json::to_string_pretty(&json!({"tabs": [tab("a"), tab("b"), tab("c")], "active": "c"})).unwrap();
        // Cut in the middle of the third tab.
        let cut = &full[..full.rfind("\"columns\"").unwrap()];
        let (value, kept) = tabs(cut).unwrap();
        assert_eq!(kept, 2);
        let ids: Vec<_> = value["tabs"].as_array().unwrap().iter().map(|t| t["id"].as_str().unwrap().to_owned()).collect();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn nothing_readable_gives_none() {
        assert!(tabs("").is_none());
        assert!(tabs("{\"tabs\": [{\"id\": \"a\", \"chan").is_none());
        assert!(tabs("not json at all").is_none());
    }

    #[test]
    fn a_tabs_key_inside_a_tab_or_a_string_is_not_the_list() {
        let text = r#"{"note": "\"tabs\": [", "x": {"tabs": [1]}, "tabs": [{"id": "a", "columns": [{"tiles": [{"id": "ta", "channel": {"id": "1"}}]}]}, {"id": "b""#;
        let (_, kept) = tabs(text).unwrap();
        assert_eq!(kept, 1);
    }
}
