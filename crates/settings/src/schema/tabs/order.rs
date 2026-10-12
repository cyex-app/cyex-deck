//! Tab order rules (U5): a group's tabs sit together, and closing the shown tab shows
//! the one next to it.

use std::collections::HashMap;

use super::Tab;

/// Moves each group's tabs next to its first tab, keeping their order.
pub(super) fn group_together(tabs: &mut Vec<Tab>) {
    enum Slot {
        Tab(Tab),
        Group(String),
    }
    let mut slots = Vec::with_capacity(tabs.len());
    let mut groups: HashMap<String, Vec<Tab>> = HashMap::new();
    for tab in tabs.drain(..) {
        match tab.group.clone() {
            None => slots.push(Slot::Tab(tab)),
            Some(g) => {
                if !groups.contains_key(&g) {
                    slots.push(Slot::Group(g.clone()));
                }
                groups.entry(g).or_default().push(tab);
            }
        }
    }
    for slot in slots {
        match slot {
            Slot::Tab(tab) => tabs.push(tab),
            Slot::Group(g) => tabs.extend(groups.remove(&g).unwrap_or_default()),
        }
    }
}

/// The tab to show once `closing` are gone and `active` was one of them: the next one
/// to the right, else to the left.
pub(super) fn neighbor(tabs: &[Tab], active: &str, closing: &dyn Fn(&str) -> bool) -> Option<String> {
    let at = tabs.iter().position(|t| t.id == active)?;
    let keep = |t: &&Tab| !closing(&t.id);
    tabs[at..]
        .iter()
        .find(keep)
        .or_else(|| tabs[..at].iter().rev().find(keep))
        .map(|t| t.id.clone())
}

#[cfg(test)]
mod tests {
    use super::super::testing::tab;
    use super::*;

    fn grouped(id: &str, group: Option<&str>) -> Tab {
        Tab {
            group: group.map(Into::into),
            ..tab(id, id)
        }
    }

    fn ids(tabs: &[Tab]) -> Vec<&str> {
        tabs.iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn groups_gather_at_their_first_tab() {
        let mut tabs = vec![
            grouped("a", None),
            grouped("b", Some("g")),
            grouped("c", None),
            grouped("d", Some("h")),
            grouped("e", Some("g")),
        ];
        group_together(&mut tabs);
        assert_eq!(ids(&tabs), ["a", "b", "e", "c", "d"]);
    }

    #[test]
    fn closing_shows_the_next_tab() {
        let tabs = vec![tab("a", "a"), tab("b", "b"), tab("c", "c")];
        assert_eq!(neighbor(&tabs, "b", &|id| id == "b").as_deref(), Some("c"));
        assert_eq!(neighbor(&tabs, "c", &|id| id == "c").as_deref(), Some("b"));
        assert_eq!(neighbor(&tabs, "b", &|id| id != "a").as_deref(), Some("a"));
        assert_eq!(neighbor(&tabs, "a", &|_| true), None);
    }
}
