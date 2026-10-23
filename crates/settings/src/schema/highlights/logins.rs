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
        // V107-H6c Past `max` the rest would be cut anyway: a huge hand-edited list stays quick.
        if out.len() >= max {
            break;
        }
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
    fn v107_h6c_a_huge_list_stops_at_the_cap() {
        let mut l: Vec<String> = (0..200_000).map(|i| format!("u{i}, u{i}")).collect();
        super::clean(&mut l, 3);
        assert_eq!(l, ["u0", "u1", "u2"]);
        let mut l: Vec<String> = ["a b c d"].map(String::from).to_vec();
        super::clean(&mut l, 2);
        assert_eq!(l, ["a", "b"], "a row past the cap is cut too");
    }

    #[test]
    fn empty_and_separator_only_text_leaves_no_login() {
        // V107-H4a An old file with "" or " , ," must not keep a name that matches nobody.
        for old in ["", "  ", " , ,, ", "@", "# @"] {
            let mut h: Highlights = serde_json::from_value(serde_json::json!({ "never": old })).unwrap();
            h.sanitize();
            h.prune_blank();
            assert!(h.never.is_empty(), "{old:?} -> {:?}", h.never);
        }
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
