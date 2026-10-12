//! S1 Menu entries and usercard buttons from the user's lines, Chatty's format:
//! `Label=command`, `/Name` (a custom command by name), `@Submenu` followed by
//! `.Label=command` lines, and `-` for a divider. `#` lines are comments.

use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "t", rename_all = "camelCase")]
#[ts(export)]
pub enum MenuEntry {
    Item {
        label: String,
        /// A command body, filled in when picked.
        command: String,
    },
    Sub {
        label: String,
        items: Vec<MenuEntry>,
    },
    Separator,
}

pub fn parse_menu(lines: &[String]) -> Vec<MenuEntry> {
    let mut out: Vec<MenuEntry> = Vec::new();
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('@') {
            out.push(MenuEntry::Sub {
                label: name.trim().to_owned(),
                items: Vec::new(),
            });
            continue;
        }
        let (inner, line) = match line.strip_prefix('.') {
            Some(rest) => (true, rest.trim()),
            None => (false, line),
        };
        let entry = if line.chars().all(|c| c == '-') {
            MenuEntry::Separator
        } else if let Some((label, command)) = line.split_once('=') {
            MenuEntry::Item {
                label: label.trim().to_owned(),
                command: command.trim().to_owned(),
            }
        } else if let Some(name) = line.strip_prefix('/') {
            // `/Ban_Reason` shows as "Ban Reason" and runs `/ban_reason $1-`.
            MenuEntry::Item {
                label: name.replace('_', " "),
                command: format!("/{} $1-", name.to_ascii_lowercase()),
            }
        } else {
            continue;
        };
        match (inner, out.last_mut()) {
            (true, Some(MenuEntry::Sub { items, .. })) => items.push(entry),
            _ => out.push(entry),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus() {
        let lines: Vec<String> = [
            "# mine",
            "Logs=/openlogs $1",
            "-",
            "@Copy",
            ".Name=/copy $1",
            ".Message=/copy $(msg)",
            "/Shoutout",
            "junk",
        ]
        .map(String::from)
        .to_vec();
        let m = parse_menu(&lines);
        assert_eq!(m.len(), 4);
        assert!(matches!(&m[1], MenuEntry::Separator));
        let MenuEntry::Sub { label, items } = &m[2] else {
            panic!()
        };
        assert_eq!((label.as_str(), items.len()), ("Copy", 2));
        assert_eq!(
            m[3],
            MenuEntry::Item {
                label: "Shoutout".into(),
                command: "/shoutout $1-".into()
            }
        );
    }
}
