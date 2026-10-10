//! Every change to the tab list (1.16-1.18, 1.22, U3, U5). Windows send a [`TabOp`];
//! the result is saved and sent to every window.

use std::fmt;

use serde::Deserialize;
use ts_rs::TS;

use super::order::neighbor;
use super::{Column, GroupColor, Schema, Tab, TabChannel, TabGroup, Tabs, Tile, TileShow};

#[derive(Clone, Debug, Deserialize, TS)]
#[serde(tag = "op", rename_all = "camelCase")]
#[ts(export)]
pub enum TabOp {
    /// A new tab, or a new column in the tab `into` (U3). Shows it.
    Add {
        channel: TabChannel,
        show: TileShow,
        account: Option<String>,
        alias: Option<String>,
        into: Option<String>,
    },
    /// H3 Several channels side by side in one new tab, as a grid. Only the first plays
    /// sound. Shows it. New single tabs keep starting from `last_show`.
    AddMany {
        channels: Vec<TabChannel>,
        show: TileShow,
        account: Option<String>,
    },
    /// 1.18 Another channel, stream/chat choice or account for a tile.
    Edit {
        tile: String,
        channel: TabChannel,
        show: TileShow,
        account: Option<String>,
    },
    /// V101-20 Stream only, chat only or both, from the tab menu. Unlike `Edit`, new tabs
    /// keep starting from `last_show`.
    Show { tile: String, show: TileShow },
    /// 1.22 `None` goes back to the channel names.
    Rename { tab: String, alias: Option<String> },
    Activate { tab: Option<String> },
    /// U5 One tab or several.
    Close { tabs: Vec<String> },
    /// Closing the last tile closes its tab.
    CloseTile { tile: String },
    /// Puts `tab` before `before` (last if `None`), in `group`.
    Move {
        tab: String,
        before: Option<String>,
        group: Option<String>,
    },
    /// U3 Next to `target` on the side `zone`. The middle swaps the two.
    MoveTile { tile: String, target: String, zone: Zone },
    /// U3 Into its own tab, before `before` (last if `None`).
    Detach { tile: String, before: Option<String> },
    /// U3 Into another tab, as its last column.
    SendTile { tile: String, tab: String },
    /// U3 Sizes after dragging a divider. Ignored if the layout changed meanwhile.
    Resize {
        tab: String,
        columns: Vec<f32>,
        rows: Vec<Vec<f32>>,
    },
    /// F4 A new tab showing a feed. Shows it.
    AddFeed { name: String },
    /// F3 The filters a tile's chat shows messages through.
    Filters { tile: String, filters: Vec<String> },
    /// 3.11 Chat's share of the tile.
    Ratio { tile: String, ratio: f32 },
    /// The tile's player volume.
    Sound { tile: String, volume: f32, muted: bool },
    /// U5 `None` takes the tabs out of their group.
    Group { tabs: Vec<String>, group: Option<String> },
    NewGroup {
        tabs: Vec<String>,
        name: String,
        color: GroupColor,
    },
    EditGroup {
        group: String,
        name: Option<String>,
        color: Option<GroupColor>,
        collapsed: Option<bool>,
    },
    /// Keeps the tabs, drops the group.
    Ungroup { group: String },
}

/// U3 Where a dragged tile lands on another one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Zone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TabError(pub &'static str);

impl fmt::Display for TabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TabError {}

const GONE_TAB: TabError = TabError("That tab is closed.");
const GONE_TILE: TabError = TabError("That channel isn't open anymore.");
const GONE_GROUP: TabError = TabError("That group is gone.");
const NO_TABS: TabError = TabError("Pick at least one tab.");
const NO_CHANNELS: TabError = TabError("Pick at least one channel.");

/// H3 A 3 by 3 grid at most.
pub const MAX_MULTI: usize = 9;

/// Where a tile sits.
#[derive(Clone, Copy, Debug)]
pub(super) struct At {
    pub tab: usize,
    pub column: usize,
    pub row: usize,
}

impl Tabs {
    /// Applies an edit, then repairs the list. Returns the id of a new tab or group.
    pub fn apply(&mut self, op: TabOp, new_id: &mut dyn FnMut() -> String) -> Result<Option<String>, TabError> {
        let made = self.run(op, new_id)?;
        self.sanitize();
        Ok(made)
    }

    fn run(&mut self, op: TabOp, new_id: &mut dyn FnMut() -> String) -> Result<Option<String>, TabError> {
        match op {
            TabOp::Add {
                channel,
                show,
                account,
                alias,
                into,
            } => {
                let tab_id = match into {
                    Some(tab) => {
                        self.tab_index(&tab)?;
                        tab
                    }
                    None => {
                        let id = self.fresh(new_id);
                        self.tabs.push(Tab {
                            id: id.clone(),
                            alias,
                            group: None,
                            columns: Vec::new(),
                        });
                        id
                    }
                };
                let tile = Tile {
                    id: self.fresh(new_id),
                    channel,
                    show,
                    account,
                    ratio: self.last_ratio,
                    size: 1.0,
                    volume: self.last_volume,
                    muted: false,
                    feed: None,
                    filters: Vec::new(),
                };
                let i = self.tab_index(&tab_id)?;
                let size = average_width(&self.tabs[i]);
                self.tabs[i].columns.push(Column::of(tile, size));
                self.last_show = show;
                self.active = Some(tab_id.clone());
                Ok(Some(tab_id))
            }
            TabOp::AddMany {
                channels,
                show,
                account,
            } => {
                if channels.is_empty() {
                    return Err(NO_CHANNELS);
                }
                let id = self.fresh(new_id);
                let count = channels.len().min(MAX_MULTI);
                let mut columns = vec![Column::default(); grid_columns(count)];
                let width = columns.len();
                for (i, channel) in channels.into_iter().take(MAX_MULTI).enumerate() {
                    let tile = Tile {
                        id: self.fresh(new_id),
                        channel,
                        show,
                        account: account.clone(),
                        ratio: self.last_ratio,
                        size: 1.0,
                        volume: self.last_volume,
                        muted: i > 0,
                        feed: None,
                        filters: Vec::new(),
                    };
                    columns[i % width].tiles.push(tile);
                }
                self.tabs.push(Tab {
                    id: id.clone(),
                    alias: None,
                    group: None,
                    columns,
                });
                self.active = Some(id.clone());
                Ok(Some(id))
            }
            TabOp::Edit {
                tile,
                channel,
                show,
                account,
            } => {
                let at = self.locate(&tile)?;
                let t = self.tile_mut(at);
                t.channel = channel;
                t.show = show;
                t.account = account;
                self.last_show = show;
                Ok(None)
            }
            TabOp::Show { tile, show } => {
                if !show.player && !show.chat {
                    return Ok(None);
                }
                let at = self.locate(&tile)?;
                self.tile_mut(at).show = show;
                Ok(None)
            }
            TabOp::Rename { tab, alias } => {
                let i = self.tab_index(&tab)?;
                self.tabs[i].alias = alias;
                Ok(None)
            }
            TabOp::Activate { tab } => {
                if let Some(t) = &tab {
                    self.tab_index(t)?;
                }
                self.active = tab;
                Ok(None)
            }
            TabOp::Close { tabs } => {
                self.remove_tabs(&|id| tabs.iter().any(|t| t == id));
                Ok(None)
            }
            TabOp::CloseTile { tile } => {
                let at = self.locate(&tile)?;
                self.take(at);
                Ok(None)
            }
            TabOp::Move { tab, before, group } => {
                if let Some(g) = &group {
                    self.group_index(g)?;
                }
                let i = self.tab_index(&tab)?;
                let mut moved = self.tabs.remove(i);
                moved.group = group;
                let at = self.position(before.as_deref());
                self.tabs.insert(at, moved);
                Ok(None)
            }
            TabOp::MoveTile { tile, target, zone } => {
                self.move_tile(&tile, &target, zone)?;
                Ok(None)
            }
            TabOp::Detach { tile, before } => {
                let at = self.locate(&tile)?;
                if self.tabs[at.tab].tiles().count() == 1 {
                    let tab = self.tabs[at.tab].id.clone();
                    return self.run(TabOp::Move { tab, before, group: None }, new_id);
                }
                let mut moved = self.take(at);
                moved.size = 1.0;
                let id = self.fresh(new_id);
                let at = self.position(before.as_deref());
                self.tabs.insert(
                    at,
                    Tab {
                        id: id.clone(),
                        alias: None,
                        group: None,
                        columns: vec![Column::of(moved, 1.0)],
                    },
                );
                self.active = Some(id.clone());
                Ok(Some(id))
            }
            TabOp::SendTile { tile, tab } => {
                let at = self.locate(&tile)?;
                let dest = self.tab_index(&tab)?;
                if at.tab == dest && self.tabs[dest].columns.len() == 1 {
                    return Ok(None);
                }
                let mut moved = self.take(at);
                moved.size = 1.0;
                let dest = self.tab_index(&tab)?;
                let size = average_width(&self.tabs[dest]);
                self.tabs[dest].columns.push(Column::of(moved, size));
                self.active = Some(tab);
                Ok(None)
            }
            TabOp::Resize { tab, columns, rows } => {
                let i = self.tab_index(&tab)?;
                let t = &mut self.tabs[i];
                let same_shape = columns.len() == t.columns.len()
                    && rows.len() == t.columns.len()
                    && rows.iter().zip(&t.columns).all(|(r, c)| r.len() == c.tiles.len());
                if same_shape {
                    for ((column, width), heights) in t.columns.iter_mut().zip(columns).zip(rows) {
                        column.size = width;
                        for (tile, height) in column.tiles.iter_mut().zip(heights) {
                            tile.size = height;
                        }
                    }
                }
                Ok(None)
            }
            TabOp::AddFeed { name } => {
                let name = name.trim().to_owned();
                if name.is_empty() {
                    return Err(TabError("Give the feed a name."));
                }
                let id = self.fresh(new_id);
                let tile = Tile {
                    id: self.fresh(new_id),
                    show: TileShow { player: false, chat: true },
                    feed: Some(name),
                    ..Tile::default()
                };
                self.tabs.push(Tab {
                    id: id.clone(),
                    alias: None,
                    group: None,
                    columns: vec![Column::of(tile, 1.0)],
                });
                self.active = Some(id.clone());
                Ok(Some(id))
            }
            TabOp::Filters { tile, filters } => {
                let at = self.locate(&tile)?;
                self.tile_mut(at).filters = filters;
                Ok(None)
            }
            TabOp::Ratio { tile, ratio } => {
                let at = self.locate(&tile)?;
                self.tile_mut(at).ratio = ratio;
                self.last_ratio = ratio;
                Ok(None)
            }
            TabOp::Sound { tile, volume, muted } => {
                let at = self.locate(&tile)?;
                let t = self.tile_mut(at);
                (t.volume, t.muted) = (volume, muted);
                self.last_volume = volume;
                Ok(None)
            }
            TabOp::Group { tabs, group } => {
                self.group(&tabs, group)?;
                Ok(None)
            }
            TabOp::NewGroup { tabs, name, color } => {
                if !self.tabs.iter().any(|t| tabs.contains(&t.id)) {
                    return Err(NO_TABS);
                }
                let id = self.fresh(new_id);
                self.groups.push(TabGroup {
                    id: id.clone(),
                    name,
                    color,
                    collapsed: false,
                });
                self.group(&tabs, Some(id.clone()))?;
                Ok(Some(id))
            }
            TabOp::EditGroup {
                group,
                name,
                color,
                collapsed,
            } => {
                let i = self.group_index(&group)?;
                let g = &mut self.groups[i];
                if let Some(name) = name {
                    g.name = name;
                }
                if let Some(color) = color {
                    g.color = color;
                }
                if let Some(collapsed) = collapsed {
                    g.collapsed = collapsed;
                }
                Ok(None)
            }
            TabOp::Ungroup { group } => {
                self.group_index(&group)?;
                for t in self.tabs.iter_mut().filter(|t| t.group.as_ref() == Some(&group)) {
                    t.group = None;
                }
                Ok(None)
            }
        }
    }

    fn tab_index(&self, id: &str) -> Result<usize, TabError> {
        self.tabs.iter().position(|t| t.id == id).ok_or(GONE_TAB)
    }

    fn group_index(&self, id: &str) -> Result<usize, TabError> {
        self.groups.iter().position(|g| g.id == id).ok_or(GONE_GROUP)
    }

    /// Index to insert at so the new tab lands before `before`, or last.
    fn position(&self, before: Option<&str>) -> usize {
        before
            .and_then(|b| self.tabs.iter().position(|t| t.id == b))
            .unwrap_or(self.tabs.len())
    }

    pub(super) fn locate(&self, tile: &str) -> Result<At, TabError> {
        for (tab, t) in self.tabs.iter().enumerate() {
            for (column, c) in t.columns.iter().enumerate() {
                if let Some(row) = c.tiles.iter().position(|x| x.id == tile) {
                    return Ok(At { tab, column, row });
                }
            }
        }
        Err(GONE_TILE)
    }

    fn tile_mut(&mut self, at: At) -> &mut Tile {
        &mut self.tabs[at.tab].columns[at.column].tiles[at.row]
    }

    /// Takes a tile out. An emptied column goes, and so does an emptied tab.
    fn take(&mut self, at: At) -> Tile {
        let tab = &mut self.tabs[at.tab];
        let column = &mut tab.columns[at.column];
        let tile = column.tiles.remove(at.row);
        if column.tiles.is_empty() {
            tab.columns.remove(at.column);
        }
        if tab.columns.is_empty() {
            let id = tab.id.clone();
            self.remove_tabs(&|t| t == id);
        }
        tile
    }

    fn remove_tabs(&mut self, closing: &dyn Fn(&str) -> bool) {
        if let Some(active) = self.active.clone()
            && closing(&active)
        {
            self.active = neighbor(&self.tabs, &active, closing);
        }
        self.tabs.retain(|t| !closing(&t.id));
    }

    fn move_tile(&mut self, tile: &str, target: &str, zone: Zone) -> Result<(), TabError> {
        if tile == target {
            return Ok(());
        }
        let from = self.locate(tile)?;
        let to = self.locate(target)?;
        let target_tab = self.tabs[to.tab].id.clone();
        if zone == Zone::Center {
            let a = self.tile_mut(from).clone();
            let b = self.tile_mut(to).clone();
            *self.tile_mut(from) = Tile { size: a.size, ..b };
            *self.tile_mut(to) = Tile { size: b.size, ..a };
            self.active = Some(target_tab);
            return Ok(());
        }
        let mut moved = self.take(from);
        let to = self.locate(target)?;
        let tab = &mut self.tabs[to.tab];
        match zone {
            Zone::Left | Zone::Right => {
                let column = &mut tab.columns[to.column];
                column.size /= 2.0;
                moved.size = 1.0;
                let at = to.column + usize::from(zone == Zone::Right);
                let size = column.size;
                tab.columns.insert(at, Column::of(moved, size));
            }
            Zone::Top | Zone::Bottom => {
                let column = &mut tab.columns[to.column];
                let next = &mut column.tiles[to.row];
                next.size /= 2.0;
                moved.size = next.size;
                let at = to.row + usize::from(zone == Zone::Bottom);
                column.tiles.insert(at, moved);
            }
            Zone::Center => unreachable!("handled above"),
        }
        self.active = Some(target_tab);
        Ok(())
    }

    /// U5 Joining tabs go to the end of their new group.
    fn group(&mut self, tabs: &[String], group: Option<String>) -> Result<(), TabError> {
        let first = self.tabs.iter().position(|t| tabs.contains(&t.id)).ok_or(NO_TABS)?;
        let Some(group) = group else {
            for t in self.tabs.iter_mut().filter(|t| tabs.contains(&t.id)) {
                t.group = None;
            }
            return Ok(());
        };
        self.group_index(&group)?;
        let (mut moving, rest): (Vec<Tab>, Vec<Tab>) =
            std::mem::take(&mut self.tabs).into_iter().partition(|t| tabs.contains(&t.id));
        self.tabs = rest;
        for t in &mut moving {
            t.group = Some(group.clone());
        }
        let at = self
            .tabs
            .iter()
            .rposition(|t| t.group.as_ref() == Some(&group))
            .map_or(first, |i| i + 1);
        self.tabs.splice(at..at, moving);
        Ok(())
    }
}

/// A new column's width: the average of the others, so it gets a fair share.
/// H3 Up to three side by side, then a square-ish grid filled row by row.
fn grid_columns(count: usize) -> usize {
    if count <= 3 {
        count
    } else {
        (count as f64).sqrt().ceil() as usize
    }
}

pub(super) fn average_width(tab: &Tab) -> f32 {
    if tab.columns.is_empty() {
        return 1.0;
    }
    tab.columns.iter().map(|c| c.size).sum::<f32>() / tab.columns.len() as f32
}

#[cfg(test)]
mod tests {
    use super::super::testing::{counter, order, tab};
    use super::*;

    fn tabs(ids: &[&str]) -> Tabs {
        Tabs {
            tabs: ids.iter().map(|id| tab(id, id)).collect(),
            active: ids.first().map(|s| (*s).to_owned()),
            ..Tabs::default()
        }
    }

    fn apply(t: &mut Tabs, op: TabOp) -> Option<String> {
        t.apply(op, &mut counter()).unwrap()
    }

    fn channel(login: &str) -> TabChannel {
        TabChannel {
            id: format!("u{login}"),
            login: login.into(),
            name: login.into(),
        }
    }

    fn add(t: &mut Tabs, login: &str, into: Option<&str>) -> String {
        let op = TabOp::Add {
            channel: channel(login),
            show: TileShow {
                player: false,
                chat: true,
            },
            account: Some("me".into()),
            alias: None,
            into: into.map(Into::into),
        };
        apply(t, op).unwrap()
    }

    /// Channel logins per column, left to right.
    fn layout(t: &Tab) -> Vec<Vec<&str>> {
        t.columns
            .iter()
            .map(|c| c.tiles.iter().map(|x| x.channel.login.as_str()).collect())
            .collect()
    }

    #[test]
    fn adding_opens_a_new_tab_and_remembers_the_choice() {
        let mut t = tabs(&["a"]);
        let id = add(&mut t, "xqc", None);
        assert_eq!(order(&t), ["a", id.as_str()]);
        assert_eq!(t.active.as_deref(), Some(id.as_str()));
        assert!(!t.last_show.player);
        let tile = &t.tabs[1].columns[0].tiles[0];
        assert_ne!(tile.id, id);
        assert_eq!(tile.account.as_deref(), Some("me"));
    }

    #[test]
    fn adding_many_makes_a_grid() {
        let mut t = tabs(&["a"]);
        let many = |logins: &[&str]| TabOp::AddMany {
            channels: logins.iter().map(|l| channel(l)).collect(),
            show: TileShow {
                player: true,
                chat: false,
            },
            account: None,
        };
        let id = apply(&mut t, many(&["x", "y", "z"])).unwrap();
        assert_eq!(t.active.as_deref(), Some(id.as_str()));
        assert_eq!(layout(t.tab(&id).unwrap()), [["x"], ["y"], ["z"]]);
        let muted: Vec<bool> = t.tab(&id).unwrap().tiles().map(|x| x.muted).collect();
        assert_eq!(muted, [false, true, true]);
        assert!(t.last_show.chat);

        let id = apply(&mut t, many(&["1", "2", "3", "4", "5"])).unwrap();
        assert_eq!(
            layout(t.tab(&id).unwrap()),
            [vec!["1", "4"], vec!["2", "5"], vec!["3"]]
        );
        let id = apply(&mut t, many(&["1", "2", "3", "4"])).unwrap();
        assert_eq!(layout(t.tab(&id).unwrap()), [["1", "3"], ["2", "4"]]);
        let lots: Vec<String> = (0..12).map(|i| i.to_string()).collect();
        let lots: Vec<&str> = lots.iter().map(String::as_str).collect();
        let id = apply(&mut t, many(&lots)).unwrap();
        assert_eq!(t.tab(&id).unwrap().tiles().count(), MAX_MULTI);
        assert_eq!(t.apply(many(&[]), &mut counter()), Err(NO_CHANNELS));
    }

    #[test]
    fn adding_into_a_tab_tiles_it() {
        let mut t = tabs(&["a"]);
        assert_eq!(add(&mut t, "lirik", Some("a")), "a");
        assert_eq!(layout(&t.tabs[0]), [["a"], ["lirik"]]);
        let gone = TabOp::Add {
            channel: channel("x"),
            show: TileShow::default(),
            account: None,
            alias: None,
            into: Some("nope".into()),
        };
        assert_eq!(t.apply(gone, &mut counter()), Err(GONE_TAB));
    }

    #[test]
    fn editing_changes_one_tile() {
        let mut t = tabs(&["a"]);
        let op = TabOp::Edit {
            tile: "a-1".into(),
            channel: channel("pokimane"),
            show: TileShow {
                player: true,
                chat: false,
            },
            account: None,
        };
        apply(&mut t, op);
        let tile = &t.tabs[0].columns[0].tiles[0];
        assert_eq!(tile.channel.login, "pokimane");
        assert!(tile.show.player && !tile.show.chat);
        assert_eq!(tile.account, None);
    }

    #[test]
    fn showing_hides_chat_or_stream_without_changing_the_default() {
        let mut t = tabs(&["a"]);
        let before = t.last_show;
        let show = |player, chat| TabOp::Show {
            tile: "a-1".into(),
            show: TileShow { player, chat },
        };
        apply(&mut t, show(true, false));
        let tile = &t.tabs[0].columns[0].tiles[0];
        assert!(tile.show.player && !tile.show.chat);
        assert_eq!(t.last_show, before);
        // Never neither.
        apply(&mut t, show(false, false));
        assert!(t.tabs[0].columns[0].tiles[0].show.player);
    }

    #[test]
    fn closing_shows_a_neighbor() {
        let mut t = tabs(&["a", "b", "c", "d"]);
        apply(&mut t, TabOp::Activate { tab: Some("b".into()) });
        apply(
            &mut t,
            TabOp::Close {
                tabs: vec!["b".into(), "c".into()],
            },
        );
        assert_eq!(order(&t), ["a", "d"]);
        assert_eq!(t.active.as_deref(), Some("d"));
        apply(&mut t, TabOp::CloseTile { tile: "d-1".into() });
        assert_eq!(order(&t), ["a"]);
        assert_eq!(t.active.as_deref(), Some("a"));
    }

    #[test]
    fn renaming_and_clearing_an_alias() {
        let mut t = tabs(&["a"]);
        let rename = |alias: Option<&str>| TabOp::Rename {
            tab: "a".into(),
            alias: alias.map(Into::into),
        };
        apply(&mut t, rename(Some("  🎮 my tab ")));
        assert_eq!(t.tabs[0].label(), "🎮 my tab");
        apply(&mut t, rename(Some("")));
        assert_eq!(t.tabs[0].label(), "A");
    }

    #[test]
    fn moving_a_tab() {
        let mut t = tabs(&["a", "b", "c"]);
        let mv = |tab: &str, before: Option<&str>| TabOp::Move {
            tab: tab.into(),
            before: before.map(Into::into),
            group: None,
        };
        apply(&mut t, mv("c", Some("a")));
        assert_eq!(order(&t), ["c", "a", "b"]);
        apply(&mut t, mv("c", None));
        assert_eq!(order(&t), ["a", "b", "c"]);
    }

    #[test]
    fn groups_keep_their_tabs_together() {
        let mut t = tabs(&["a", "b", "c", "d"]);
        let g = apply(
            &mut t,
            TabOp::NewGroup {
                tabs: vec!["a".into(), "c".into()],
                name: "games".into(),
                color: GroupColor::Blue,
            },
        )
        .unwrap();
        assert_eq!(order(&t), ["a", "c", "b", "d"]);
        apply(
            &mut t,
            TabOp::Group {
                tabs: vec!["d".into()],
                group: Some(g.clone()),
            },
        );
        assert_eq!(order(&t), ["a", "c", "d", "b"]);

        // Dragged out of the group and dropped at the end.
        apply(
            &mut t,
            TabOp::Move {
                tab: "a".into(),
                before: None,
                group: None,
            },
        );
        assert_eq!(order(&t), ["c", "d", "b", "a"]);

        apply(
            &mut t,
            TabOp::EditGroup {
                group: g.clone(),
                name: Some("fps".into()),
                color: None,
                collapsed: Some(true),
            },
        );
        assert_eq!((t.groups[0].name.as_str(), t.groups[0].collapsed), ("fps", true));
        assert_eq!(t.groups[0].color, GroupColor::Blue);

        apply(&mut t, TabOp::Ungroup { group: g });
        assert!(t.groups.is_empty());
        assert!(t.tabs.iter().all(|x| x.group.is_none()));
        let none = TabOp::NewGroup {
            tabs: vec!["zz".into()],
            name: String::new(),
            color: GroupColor::Red,
        };
        assert_eq!(t.apply(none, &mut counter()), Err(NO_TABS));
    }

    #[test]
    fn empty_groups_disappear() {
        let mut t = tabs(&["a", "b"]);
        apply(
            &mut t,
            TabOp::NewGroup {
                tabs: vec!["a".into()],
                name: "x".into(),
                color: GroupColor::Red,
            },
        );
        apply(&mut t, TabOp::Close { tabs: vec!["a".into()] });
        assert!(t.groups.is_empty());
    }

    #[test]
    fn tiles_move_beside_or_swap_with_others() {
        let mut t = tabs(&["a", "b"]);
        add(&mut t, "c", Some("a"));
        let tile_of = |t: &Tabs, login: &str| t.tiles().find(|x| x.channel.login == login).unwrap().id.clone();
        let mv = |t: &mut Tabs, from: &str, to: &str, zone: Zone| {
            let op = TabOp::MoveTile {
                tile: tile_of(t, from),
                target: tile_of(t, to),
                zone,
            };
            apply(t, op);
        };

        // Tab b's only tile moves below a: tab b closes.
        mv(&mut t, "b", "a", Zone::Bottom);
        assert_eq!(order(&t), ["a"]);
        assert_eq!(layout(&t.tabs[0]), [vec!["a", "b"], vec!["c"]]);
        let sizes: Vec<f32> = t.tabs[0].columns[0].tiles.iter().map(|x| x.size).collect();
        assert_eq!(sizes, [1.0, 1.0]);

        mv(&mut t, "c", "a", Zone::Top);
        assert_eq!(layout(&t.tabs[0]), [["c", "a", "b"]]);
        mv(&mut t, "b", "c", Zone::Left);
        assert_eq!(layout(&t.tabs[0]), [vec!["b"], vec!["c", "a"]]);
        mv(&mut t, "b", "a", Zone::Right);
        assert_eq!(layout(&t.tabs[0]), [vec!["c", "a"], vec!["b"]]);
        mv(&mut t, "b", "c", Zone::Center);
        assert_eq!(layout(&t.tabs[0]), [vec!["b", "a"], vec!["c"]]);
        assert_eq!(t.active.as_deref(), Some("a"));
    }

    #[test]
    fn tiles_detach_into_tabs_and_back() {
        let mut t = tabs(&["a", "b"]);
        add(&mut t, "c", Some("a"));
        let c = t.tabs[0].columns[1].tiles[0].id.clone();

        let id = apply(
            &mut t,
            TabOp::Detach {
                tile: c.clone(),
                before: Some("b".into()),
            },
        )
        .unwrap();
        assert_eq!(order(&t), ["a", id.as_str(), "b"]);
        assert_eq!(t.active.as_deref(), Some(id.as_str()));

        apply(&mut t, TabOp::SendTile { tile: c, tab: "b".into() });
        assert_eq!(order(&t), ["a", "b"]);
        assert_eq!(layout(&t.tabs[1]), [["b"], ["c"]]);
        // What stays behind fills its tab again.
        assert_eq!(t.tabs[0].columns[0].size, 1.0);
        assert_eq!(t.active.as_deref(), Some("b"));

        // A tab's only tile "detached" just moves the tab.
        apply(
            &mut t,
            TabOp::Detach {
                tile: "a-1".into(),
                before: None,
            },
        );
        assert_eq!(order(&t), ["b", "a"]);
    }

    #[test]
    fn resizing_needs_the_same_layout() {
        let mut t = tabs(&["a"]);
        add(&mut t, "c", Some("a"));
        let resize = |columns: Vec<f32>, rows: Vec<Vec<f32>>| TabOp::Resize {
            tab: "a".into(),
            columns,
            rows,
        };
        let share = |t: &Tabs| t.tabs[0].columns[0].size / t.tabs[0].columns[1].size;
        apply(&mut t, resize(vec![2.0, 1.0], vec![vec![1.0], vec![1.0]]));
        assert!((share(&t) - 2.0).abs() < 1e-5);
        apply(&mut t, resize(vec![9.0], vec![vec![1.0]]));
        assert!((share(&t) - 2.0).abs() < 1e-5);

        apply(
            &mut t,
            TabOp::Ratio {
                tile: "a-1".into(),
                ratio: 0.4,
            },
        );
        assert_eq!((t.tabs[0].columns[0].tiles[0].ratio, t.last_ratio), (0.4, 0.4));

        apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 0.8, muted: true });
        let tile = &t.tabs[0].columns[0].tiles[0];
        assert_eq!((tile.volume, tile.muted, t.last_volume), (0.8, true, 0.8));
        apply(&mut t, TabOp::Sound { tile: "a-1".into(), volume: 7.0, muted: false });
        assert_eq!(t.tabs[0].columns[0].tiles[0].volume, 1.0, "clamped");
    }

    #[test]
    fn feeds_and_filters() {
        let mut t = Tabs::default();
        apply(&mut t, TabOp::AddFeed { name: " Pings ".into() });
        let tile = &t.tabs[0].columns[0].tiles[0];
        assert_eq!((tile.feed.as_deref(), tile.show.player), (Some("Pings"), false));
        assert!(t.channel_ids().is_empty(), "feeds have no channel");
        let id = tile.id.clone();
        apply(&mut t, TabOp::Filters { tile: id, filters: vec!["f1".into(), "f1".into()] });
        assert_eq!(t.tabs[0].columns[0].tiles[0].filters, ["f1"]);
        let mut fresh = || "x".to_owned();
        assert!(t.apply(TabOp::AddFeed { name: " ".into() }, &mut fresh).is_err());
    }

    #[test]
    fn ops_parse_from_the_ui() {
        let op: TabOp = serde_json::from_str(r#"{"op":"moveTile","tile":"x","target":"y","zone":"left"}"#).unwrap();
        assert!(matches!(op, TabOp::MoveTile { zone: Zone::Left, .. }));
        let op: TabOp = serde_json::from_str(r#"{"op":"activate","tab":null}"#).unwrap();
        assert!(matches!(op, TabOp::Activate { tab: None }));
    }
}
