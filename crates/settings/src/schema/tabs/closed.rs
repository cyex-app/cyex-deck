//! U10 Closed tabs and channels, kept so they can come back where they were.

use std::collections::HashSet;

use super::ops::average_width;
use super::{Column, Schema, Tab, TabGroup, TabOp, Tabs, Tile};

/// A tab or channel the user closed.
#[derive(Clone, Debug, PartialEq)]
pub enum Closed {
    Tab {
        tab: Tab,
        /// Its group, so a closed group comes back with its tabs.
        group: Option<TabGroup>,
        /// The tabs after it, nearest first. It comes back before the first one left.
        next: Vec<String>,
    },
    /// One channel of a tab that stayed open.
    Tile {
        tile: Tile,
        tab: String,
        column: usize,
        row: usize,
        /// It had its column to itself.
        alone: bool,
    },
}

impl Tabs {
    /// What `op` would close, in list order. Call it before applying `op`.
    pub fn closing(&self, op: &TabOp) -> Vec<Closed> {
        match op {
            TabOp::Close { tabs } => (0..self.tabs.len())
                .filter(|&i| tabs.contains(&self.tabs[i].id))
                .map(|i| self.closed_tab(i))
                .collect(),
            TabOp::CloseTile { tile } => {
                let Ok(at) = self.locate(tile) else { return Vec::new() };
                let tab = &self.tabs[at.tab];
                if tab.tiles().count() == 1 {
                    return vec![self.closed_tab(at.tab)];
                }
                let column = &tab.columns[at.column];
                vec![Closed::Tile {
                    tile: column.tiles[at.row].clone(),
                    tab: tab.id.clone(),
                    column: at.column,
                    row: at.row,
                    alone: column.tiles.len() == 1,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// Puts back what one edit closed and shows it. Returns the tab shown.
    pub fn restore(&mut self, closed: Vec<Closed>, new_id: &mut dyn FnMut() -> String) -> Option<String> {
        let mut shown = None;
        // Last first, so each one's neighbours are back before it.
        for c in closed.into_iter().rev() {
            shown = Some(match c {
                Closed::Tab { tab, group, next } => self.restore_tab(tab, group, &next, new_id),
                Closed::Tile {
                    tile,
                    tab,
                    column,
                    row,
                    alone,
                } => self.restore_tile(tile, &tab, column, row, alone, new_id),
            });
        }
        self.active = shown.clone().or(self.active.take());
        self.sanitize();
        shown
    }

    fn closed_tab(&self, i: usize) -> Closed {
        let tab = &self.tabs[i];
        Closed::Tab {
            tab: tab.clone(),
            group: tab
                .group
                .as_ref()
                .and_then(|g| self.groups.iter().find(|x| &x.id == g))
                .cloned(),
            next: self.tabs[i + 1..].iter().map(|t| t.id.clone()).collect(),
        }
    }

    fn restore_tab(
        &mut self,
        mut tab: Tab,
        group: Option<TabGroup>,
        next: &[String],
        new_id: &mut dyn FnMut() -> String,
    ) -> String {
        let mut taken: HashSet<String> = self.ids().into_iter().map(str::to_owned).collect();
        renew(&mut tab.id, &mut taken, new_id);
        for tile in tab.columns.iter_mut().flat_map(|c| c.tiles.iter_mut()) {
            renew(&mut tile.id, &mut taken, new_id);
        }
        if let Some(g) = group
            && !self.groups.iter().any(|x| x.id == g.id)
        {
            self.groups.push(g);
        }
        let at = next
            .iter()
            .find_map(|n| self.tabs.iter().position(|t| &t.id == n))
            .unwrap_or(self.tabs.len());
        let id = tab.id.clone();
        self.tabs.insert(at, tab);
        id
    }

    fn restore_tile(
        &mut self,
        mut tile: Tile,
        tab: &str,
        column: usize,
        row: usize,
        alone: bool,
        new_id: &mut dyn FnMut() -> String,
    ) -> String {
        let mut taken: HashSet<String> = self.ids().into_iter().map(str::to_owned).collect();
        renew(&mut tile.id, &mut taken, new_id);
        tile.size = 1.0;
        let Some(i) = self.tabs.iter().position(|t| t.id == tab) else {
            // Its tab is gone too: it gets its own, last.
            let id = self.fresh(new_id);
            self.tabs.push(Tab {
                id: id.clone(),
                columns: vec![Column::of(tile, 1.0)],
                ..Tab::default()
            });
            return id;
        };
        let t = &mut self.tabs[i];
        if alone || column >= t.columns.len() {
            let size = average_width(t);
            t.columns.insert(column.min(t.columns.len()), Column::of(tile, size));
        } else {
            let c = &mut t.columns[column];
            c.tiles.insert(row.min(c.tiles.len()), tile);
        }
        t.id.clone()
    }
}

/// Swaps `id` for a new one if something else uses it now.
fn renew(id: &mut String, taken: &mut HashSet<String>, new_id: &mut dyn FnMut() -> String) {
    while id.is_empty() || taken.contains(id.as_str()) {
        *id = new_id();
    }
    taken.insert(id.clone());
}

#[cfg(test)]
mod tests {
    use super::super::testing::{counter, order, tab, tile};
    use super::super::GroupColor;
    use super::*;

    fn tabs(ids: &[&str]) -> Tabs {
        Tabs {
            tabs: ids.iter().map(|id| tab(id, id)).collect(),
            active: ids.first().map(|s| (*s).to_owned()),
            ..Tabs::default()
        }
    }

    /// Applies `op` and returns what it closed.
    fn close(t: &mut Tabs, op: TabOp) -> Vec<Closed> {
        let closed = t.closing(&op);
        t.apply(op, &mut counter()).unwrap();
        closed
    }

    fn close_tabs(t: &mut Tabs, ids: &[&str]) -> Vec<Closed> {
        let tabs = ids.iter().map(|s| (*s).to_owned()).collect();
        close(t, TabOp::Close { tabs })
    }

    #[test]
    fn a_closed_tab_comes_back_in_place_and_shown() {
        let mut t = tabs(&["a", "b", "c"]);
        let closed = close_tabs(&mut t, &["b"]);
        assert_eq!(order(&t), ["a", "c"]);
        assert_eq!(t.restore(closed, &mut counter()).as_deref(), Some("b"));
        assert_eq!(order(&t), ["a", "b", "c"]);
        assert_eq!(t.active.as_deref(), Some("b"));
        assert_eq!(t.tab("b").unwrap().tiles().next().unwrap().id, "b-1", "ids kept when free");
    }

    #[test]
    fn tabs_closed_together_come_back_together() {
        let mut t = tabs(&["a", "b", "c", "d"]);
        let closed = close_tabs(&mut t, &["b", "c", "d"]);
        assert_eq!(order(&t), ["a"]);
        t.restore(closed, &mut counter());
        assert_eq!(order(&t), ["a", "b", "c", "d"]);
        assert_eq!(t.active.as_deref(), Some("b"), "the first one shows");
    }

    #[test]
    fn falls_back_to_a_later_neighbour_or_the_end() {
        let mut t = tabs(&["a", "b", "c", "d"]);
        let closed = close_tabs(&mut t, &["b"]);
        close_tabs(&mut t, &["c"]);
        t.restore(closed, &mut counter());
        assert_eq!(order(&t), ["a", "b", "d"]);

        let mut t = tabs(&["a", "b"]);
        let closed = close_tabs(&mut t, &["b"]);
        t.restore(closed, &mut counter());
        assert_eq!(order(&t), ["a", "b"]);
    }

    #[test]
    fn a_closed_group_comes_back() {
        let mut t = tabs(&["a", "b", "c"]);
        let g = t
            .apply(
                TabOp::NewGroup {
                    tabs: vec!["b".into(), "c".into()],
                    name: "Fav".into(),
                    color: GroupColor::Red,
                },
                &mut counter(),
            )
            .unwrap()
            .unwrap();
        let closed = close_tabs(&mut t, &["b", "c"]);
        assert!(t.groups.is_empty(), "an empty group goes");
        t.restore(closed, &mut counter());
        assert_eq!(order(&t), ["a", "b", "c"]);
        assert_eq!(t.groups.len(), 1);
        assert_eq!(t.groups[0].name, "Fav");
        assert!(t.tabs[1..].iter().all(|x| x.group.as_ref() == Some(&g)));
    }

    #[test]
    fn taken_ids_get_new_ones() {
        let mut t = tabs(&["a", "b"]);
        let closed = close_tabs(&mut t, &["b"]);
        t.tabs.push(tab("b", "other"));
        let shown = t.restore(closed, &mut counter()).unwrap();
        assert_ne!(shown, "b");
        assert_eq!(t.tabs.len(), 3);
        let ids: HashSet<_> = t.tiles().map(|x| x.id.clone()).collect();
        assert_eq!(ids.len(), 3, "tile ids stay unique");
    }

    #[test]
    fn closing_the_last_tile_counts_as_closing_the_tab() {
        let mut t = tabs(&["a", "b"]);
        let closed = close(&mut t, TabOp::CloseTile { tile: "b-1".into() });
        assert!(matches!(closed.as_slice(), [Closed::Tab { .. }]));
        t.restore(closed, &mut counter());
        assert_eq!(order(&t), ["a", "b"]);
    }

    fn two_columns() -> Tabs {
        let mut t = tabs(&["a"]);
        t.tabs[0].columns.push(Column::of(tile("x", "xqc"), 1.0));
        t.tabs[0].columns[0].tiles.push(tile("y", "lirik"));
        t
    }

    fn layout(t: &Tabs) -> Vec<Vec<&str>> {
        t.tabs[0]
            .columns
            .iter()
            .map(|c| c.tiles.iter().map(|x| x.id.as_str()).collect())
            .collect()
    }

    #[test]
    fn a_closed_channel_goes_back_into_its_tab() {
        let mut t = two_columns();
        assert_eq!(layout(&t), [vec!["a-1", "y"], vec!["x"]]);

        let closed = close(&mut t, TabOp::CloseTile { tile: "a-1".into() });
        assert_eq!(layout(&t), [vec!["y"], vec!["x"]]);
        assert_eq!(t.restore(closed, &mut counter()).as_deref(), Some("a"));
        assert_eq!(layout(&t), [vec!["a-1", "y"], vec!["x"]]);

        let closed = close(&mut t, TabOp::CloseTile { tile: "x".into() });
        assert_eq!(layout(&t), [vec!["a-1", "y"]]);
        t.restore(closed, &mut counter());
        assert_eq!(layout(&t), [vec!["a-1", "y"], vec!["x"]]);
        let widths: Vec<f32> = t.tabs[0].columns.iter().map(|c| c.size).collect();
        assert_eq!(widths, [1.0, 1.0]);
    }

    #[test]
    fn a_closed_channel_whose_tab_went_gets_its_own() {
        let mut t = two_columns();
        t.tabs.push(tab("b", "b"));
        let closed = close(&mut t, TabOp::CloseTile { tile: "x".into() });
        close_tabs(&mut t, &["a"]);
        let shown = t.restore(closed, &mut counter()).unwrap();
        assert_eq!(order(&t), ["b", shown.as_str()]);
        assert_eq!(t.tab(&shown).unwrap().tiles().next().unwrap().id, "x");
    }

    #[test]
    fn other_edits_close_nothing() {
        let t = tabs(&["a", "b"]);
        let op = TabOp::Detach {
            tile: "b-1".into(),
            before: None,
        };
        assert!(t.closing(&op).is_empty());
        assert!(t.closing(&TabOp::Close { tabs: vec!["zz".into()] }).is_empty());
    }
}
