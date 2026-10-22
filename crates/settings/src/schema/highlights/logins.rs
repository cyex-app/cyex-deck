//! V107-Mq "Never highlight" and "Silent chats": one login per row.

use serde::{Deserialize, Deserializer};

/// A list, or the old text "a, b c" (a hand-edited or pre-list file), read as rows.
pub(super) fn de_logins<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        List(Vec<String>),
        Text(String),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Raw::deserialize(d)? {
        Raw::List(l) => l,
        Raw::Text(t) => vec![t],
        Raw::Other(_) => Vec::new(),
    })
}

/// Lowercased logins without `@`/`#`, one per row: a row holding "a, b" becomes two.
/// Repeats go (the first stays, order kept). Blank rows stay while being filled in;
/// `prune_blank` drops them.
pub(super) fn clean(list: &mut Vec<String>, max: usize) {
    let mut out: Vec<String> = Vec::with_capacity(list.len());
    for row in list.drain(..) {
        let mut parts = row
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(|p| p.trim_start_matches(['@', '#']).to_ascii_lowercase())
            .filter(|p| !p.is_empty())
            .peekable();
        if parts.peek().is_none() {
            out.push(String::new());
            continue;
        }
        for p in parts {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out.truncate(max);
    *list = out;
}

#[cfg(test)]
mod tests {
    use super::super::Highlights;
    use crate::schema::Schema;

    #[test]
    fn old_text_and_joined_rows_become_one_login_per_row() {
        let mut h: Highlights = serde_json::from_value(serde_json::json!({
            "never": "Nightbot, @StreamElements  moobot,nightbot",
            "muted": ["#Caressz", "rvbdev seryxx_test", "caressz", ""]
        }))
        .unwrap();
        h.sanitize();
        assert_eq!(h.never, ["nightbot", "streamelements", "moobot"]);
        assert_eq!(h.muted, ["caressz", "rvbdev", "seryxx_test", ""], "blank row kept while editing");
        assert!(h.prune_blank());
        assert_eq!(h.muted, ["caressz", "rvbdev", "seryxx_test"]);
    }

    #[test]
    fn current_lists_load_unchanged_and_odd_values_do_not_break_the_file() {
        let mut h: Highlights =
            serde_json::from_value(serde_json::json!({"never": ["moobot", "nightbot"], "muted": 5, "global": true}))
                .unwrap();
        h.sanitize();
        assert_eq!(h.never, ["moobot", "nightbot"]);
        assert!(h.muted.is_empty() && h.global);
    }
}
