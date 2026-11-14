//! V108-C31 FFZ has no live feed a third-party client can use, so a room's emotes are read
//! again now and then; this is what changed between two reads, by emote id.

use std::collections::HashMap;
use std::sync::Arc;

use cyex_chat::model::NoticeEmote;

use crate::model::Emote;

/// More changes than this in one read read as one line: likely a partial answer, not edits.
const MAX_LINES: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added(Arc<Emote>),
    Removed(Arc<Emote>),
    Renamed { old: String, new: Arc<Emote> },
}

/// What happened to a room's emotes between `old` and `new`, in the order added, removed,
/// renamed.
pub fn diff(old: &[Arc<Emote>], new: &[Arc<Emote>]) -> Vec<Change> {
    let before: HashMap<&str, &str> = old.iter().map(|e| (e.id.as_str(), e.name.as_str())).collect();
    let after: HashMap<&str, &str> = new.iter().map(|e| (e.id.as_str(), e.name.as_str())).collect();
    let mut out = Vec::new();
    for e in new {
        if !before.contains_key(e.id.as_str()) {
            out.push(Change::Added(e.clone()));
        }
    }
    for e in old {
        if !after.contains_key(e.id.as_str()) {
            out.push(Change::Removed(e.clone()));
        }
    }
    for e in new {
        if let Some(was) = before.get(e.id.as_str())
            && *was != e.name
        {
            out.push(Change::Renamed {
                old: (*was).to_owned(),
                new: e.clone(),
            });
        }
    }
    out
}

/// The chat lines for `changes`, no one named: FFZ does not say who. V111-01 Each comes with its
/// emote for the picture (the folded line has none).
pub fn lines(changes: &[Change]) -> Vec<(String, Option<NoticeEmote>)> {
    if changes.len() > MAX_LINES {
        return vec![(format!("{} FFZ emotes changed.", changes.len()), None)];
    }
    changes
        .iter()
        .map(|c| match c {
            Change::Added(e) => (format!("The FFZ emote {} was added.", e.name), Some(e.notice())),
            Change::Removed(e) => (format!("The FFZ emote {} was removed.", e.name), Some(e.notice())),
            Change::Renamed { old, new } => (format!("The FFZ emote {old} is now {}.", new.name), Some(new.notice())),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EmoteSet, Scope};
    use cyex_chat::model::Provider;

    fn set(items: &[(&str, &str)]) -> Vec<Arc<Emote>> {
        EmoteSet::new(
            "s",
            Provider::Ffz,
            Scope::Channel,
            items.iter().map(|(i, n)| Emote::new(Provider::Ffz, *i, *n)).collect(),
        )
        .emotes
    }

    #[test]
    fn added_removed_renamed() {
        let old = set(&[("1", "a"), ("2", "b"), ("3", "c")]);
        let new = set(&[("1", "a"), ("3", "see"), ("4", "d")]);
        let ch = diff(&old, &new);
        let shape: Vec<String> = ch
            .iter()
            .map(|c| match c {
                Change::Added(e) => format!("+{}", e.name),
                Change::Removed(e) => format!("-{}", e.name),
                Change::Renamed { old, new } => format!("{old}>{}", new.name),
            })
            .collect();
        assert_eq!(shape, ["+d", "-b", "c>see"]);
        let got = lines(&ch);
        let text: Vec<&str> = got.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(
            text,
            [
                "The FFZ emote d was added.",
                "The FFZ emote b was removed.",
                "The FFZ emote c is now see."
            ]
        );
        // V111-01 Each line carries its emote (the renamed one as it is now).
        let ids: Vec<(String, String)> = got.into_iter().filter_map(|(_, e)| e).map(|e| (e.id, e.name)).collect();
        let want = |i: &str, n: &str| (i.to_owned(), n.to_owned());
        assert_eq!(ids, [want("4", "d"), want("2", "b"), want("3", "see")]);
    }

    #[test]
    fn same_set_is_quiet() {
        let s = set(&[("1", "a")]);
        assert!(diff(&s, &s).is_empty());
        assert!(diff(&[], &[]).is_empty());
    }

    #[test]
    fn many_changes_fold_into_one_line() {
        let new: Vec<(String, String)> = (0..20).map(|i| (i.to_string(), format!("e{i}"))).collect();
        let new: Vec<(&str, &str)> = new.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let ch = diff(&[], &set(&new));
        assert_eq!(lines(&ch), [("20 FFZ emotes changed.".to_owned(), None)]);
    }

    #[test]
    fn fixture_room_against_itself_and_one_short() {
        let raw = std::fs::read_to_string(format!(
            "{}/tests/fixtures/ffz/room.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let raw: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let room = super::super::api::parse_room(&raw).unwrap().unwrap();
        let emotes = room.set.unwrap().emotes;
        assert!(!emotes.is_empty());
        assert!(diff(&emotes, &emotes).is_empty());
        let short = &emotes[1..];
        assert_eq!(diff(&emotes, short), [Change::Removed(emotes[0].clone())]);
        assert_eq!(diff(short, &emotes), [Change::Added(emotes[0].clone())]);
    }
}
