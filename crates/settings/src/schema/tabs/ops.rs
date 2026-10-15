//! Every change to the tab list (1.16-1.18, 1.22, U3, U5). Windows send a [`TabOp`];
//! the result is saved and sent to every window.

use std::fmt;

use serde::Deserialize;
use ts_rs::TS;

use super::order::neighbor;
use super::{ChatPosition, Column, GroupColor, PanelNode, Schema, Tab, TabChannel, TabGroup, Tabs, Tile, TileShow};

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
    /// V102-23 Where a tile's chat sits next to its stream. `None` follows Settings (2.1),
    /// which this never changes.
    ChatSide { tile: String, side: Option<ChatPosition> },
    /// V102-26 The mod view's share of a tile's stream area, its divider dragged. `None`
    /// goes back to the default.
    ModviewShare { tile: String, share: Option<f32> },
    /// V103-05 A tile's panels as arranged by hand. `None` goes back to the default.
    Layout { tile: String, layout: Option<PanelNode> },
    /// V102-23 Every chat in the tab back where Settings puts it, V102-26 every mod view
    /// divider back to its default, V103-05 every hand-made arrangement gone.
    ResetLayout { tab: String },
    /// The tile's player volume; V105-10 with the boost.
    Sound { tile: String, volume: f32, muted: bool, boost: bool },
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
                    boost: false,
                    feed: None,
                    filters: Vec::new(),
                    chat_side: None,
                    modview_share: None,
                    layout: None,
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
                        boost: false,
                        feed: None,
                        filters: Vec::new(),
                        chat_side: None,
                        modview_share: None,
                        layout: None,
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
            TabOp::ChatSide { tile, side } => {
                let at = self.locate(&tile)?;
                self.tile_mut(at).chat_side = side;
                Ok(None)
            }
            TabOp::ModviewShare { tile, share } => {
                let at = self.locate(&tile)?;
                self.tile_mut(at).modview_share = share;
                Ok(None)
            }
            TabOp::Layout { tile, layout } => {
                let at = self.locate(&tile)?;
                self.tile_mut(at).layout = layout;
                Ok(None)
            }
            TabOp::ResetLayout { tab } => {
                let i = self.tab_index(&tab)?;
                for column in &mut self.tabs[i].columns {
                    for t in &mut column.tiles {
                        t.chat_side = None;
                        t.modview_share = None;
                        t.layout = None;
                    }
                }
                Ok(None)
            }
            TabOp::Sound { tile, volume, muted, boost } => {
                let at = self.locate(&tile)?;
                let t = self.tile_mut(at);
                (t.volume, t.muted, t.boost) = (volume, muted, boost);
                // V105-10 New tiles start at 100 % at most, never boosted.
                self.last_volume = volume.min(1.0);
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
mod tests;
