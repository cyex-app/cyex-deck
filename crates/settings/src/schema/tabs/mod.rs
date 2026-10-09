//! Channel tabs (1.10, 1.15, 1.16, 1.17, 1.22, U3, U5). Channels are stored by user id;
//! their login and name are refreshed from Twitch (`resolve`). Every edit goes through
//! [`TabOp`], so all windows and the joined rooms follow the same list.

mod closed;
mod ops;
mod order;
mod resolve;

pub use closed::Closed;
pub use ops::{TabError, TabOp, Zone};
pub use resolve::Resolved;

use std::collections::HashSet;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// 1.22
const MAX_ALIAS_CHARS: usize = 60;
const MAX_GROUP_NAME_CHARS: usize = 40;
/// 3.11 Chat's share of a tile that also shows the stream.
pub const DEFAULT_RATIO: f32 = 0.28;
/// A player's starting volume, 0 to 1.
pub const DEFAULT_VOLUME: f32 = 0.5;
const RATIO_RANGE: (f32, f32) = (0.1, 0.9);
/// U3 Relative sizes of columns and tiles.
const SIZE_RANGE: (f32, f32) = (0.05, 20.0);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Tabs {
    pub tabs: Vec<Tab>,
    /// U5
    pub groups: Vec<TabGroup>,
    /// The tab the main window shows. `None` shows a fixed view such as home.
    pub active: Option<String>,
    /// 1.16 What the last added or edited tile showed. New tabs start from it.
    pub last_show: TileShow,
    /// 3.11 Last chat share set by the user. New tiles start from it.
    pub last_ratio: f32,
    /// Last volume set in a player. New tiles start from it.
    pub last_volume: f32,
}

impl Default for Tabs {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            groups: Vec::new(),
            active: None,
            last_show: TileShow::default(),
            last_ratio: DEFAULT_RATIO,
            last_volume: DEFAULT_VOLUME,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Tab {
    pub id: String,
    /// 1.22 Name set by the user. `None` shows the channel names.
    pub alias: Option<String>,
    /// U5
    pub group: Option<String>,
    /// U3 Left to right, each a stack of tiles.
    pub columns: Vec<Column>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Column {
    /// Width share within the tab.
    pub size: f32,
    pub tiles: Vec<Tile>,
}

impl Default for Column {
    fn default() -> Self {
        Self {
            size: 1.0,
            tiles: Vec::new(),
        }
    }
}

impl Column {
    fn of(tile: Tile, size: f32) -> Self {
        Self { size, tiles: vec![tile] }
    }
}

/// One channel inside a tab (U3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Tile {
    pub id: String,
    pub channel: TabChannel,
    /// 1.16
    pub show: TileShow,
    /// 1.17 Account user id. `None` reads anonymously.
    pub account: Option<String>,
    /// 3.11 Chat's share when the stream shows too.
    pub ratio: f32,
    /// Height share within its column.
    pub size: f32,
    /// The player's volume, 0 to 1, kept per tile so a muted stream stays muted.
    pub volume: f32,
    pub muted: bool,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            id: String::new(),
            channel: TabChannel::default(),
            show: TileShow::default(),
            account: None,
            ratio: DEFAULT_RATIO,
            size: 1.0,
            volume: DEFAULT_VOLUME,
            muted: false,
        }
    }
}

/// 1.15 Stored by user id. Login and name are the last ones Twitch reported.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct TabChannel {
    pub id: String,
    pub login: String,
    pub name: String,
}

/// 1.16 Stream and chat, just the stream, or just the chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct TileShow {
    pub player: bool,
    pub chat: bool,
}

impl Default for TileShow {
    fn default() -> Self {
        Self {
            player: true,
            chat: true,
        }
    }
}

/// U5 A named, colored run of tabs that can fold away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct TabGroup {
    pub id: String,
    pub name: String,
    pub color: GroupColor,
    pub collapsed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GroupColor {
    #[default]
    Grey,
    Blue,
    Red,
    Yellow,
    Green,
    Pink,
    Purple,
    Cyan,
}

impl Tabs {
    pub fn tab(&self, id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn tiles(&self) -> impl Iterator<Item = &Tile> {
        self.tabs.iter().flat_map(Tab::tiles)
    }

    /// Every channel id in a tab, once.
    pub fn channel_ids(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.tiles()
            .filter(|t| seen.insert(t.channel.id.as_str()))
            .map(|t| t.channel.id.clone())
            .collect()
    }

    fn ids(&self) -> HashSet<&str> {
        let tabs = self.tabs.iter().map(|t| t.id.as_str());
        let tiles = self.tiles().map(|t| t.id.as_str());
        let groups = self.groups.iter().map(|g| g.id.as_str());
        tabs.chain(tiles).chain(groups).collect()
    }

    /// A tab, tile or group id nothing uses yet.
    fn fresh(&self, new_id: &mut dyn FnMut() -> String) -> String {
        let taken = self.ids();
        loop {
            let id = new_id();
            if !id.is_empty() && !taken.contains(id.as_str()) {
                return id;
            }
        }
    }
}

impl Tab {
    pub fn tiles(&self) -> impl Iterator<Item = &Tile> {
        self.columns.iter().flat_map(|c| c.tiles.iter())
    }

    /// 1.22 The alias, or the channel names.
    pub fn label(&self) -> String {
        match &self.alias {
            Some(alias) => alias.clone(),
            None => self.tiles().map(|t| t.channel.name.as_str()).collect::<Vec<_>>().join(", "),
        }
    }
}

/// Random id for a tab, tile or group. Not for secrets.
pub fn random_id() -> String {
    let mut h = RandomState::new().build_hasher();
    h.write_u8(0);
    format!("{:010x}", h.finish() & 0xff_ffff_ffff)
}

impl Schema for Tabs {
    fn sanitize(&mut self) {
        let mut seen = HashSet::new();
        self.groups.retain_mut(|g| {
            g.name = clip(&g.name, MAX_GROUP_NAME_CHARS);
            !g.id.is_empty() && seen.insert(g.id.clone())
        });
        let groups: HashSet<String> = seen;

        let mut tab_ids = HashSet::new();
        let mut tile_ids = HashSet::new();
        self.tabs.retain_mut(|t| {
            if t.id.is_empty() || !tab_ids.insert(t.id.clone()) {
                return false;
            }
            t.alias = t.alias.take().map(|a| clip(&a, MAX_ALIAS_CHARS)).filter(|a| !a.is_empty());
            if t.group.as_ref().is_some_and(|g| !groups.contains(g)) {
                t.group = None;
            }
            for column in &mut t.columns {
                column.size = clamp(column.size, SIZE_RANGE, 1.0);
                column.tiles.retain_mut(|tile| {
                    if tile.id.is_empty() || tile.channel.id.is_empty() || !tile_ids.insert(tile.id.clone()) {
                        return false;
                    }
                    sanitize_tile(tile);
                    true
                });
            }
            t.columns.retain(|c| !c.tiles.is_empty());
            normalize(t.columns.iter_mut().map(|c| &mut c.size));
            for column in &mut t.columns {
                normalize(column.tiles.iter_mut().map(|x| &mut x.size));
            }
            !t.columns.is_empty()
        });

        let used: HashSet<&str> = self.tabs.iter().filter_map(|t| t.group.as_deref()).collect();
        self.groups.retain(|g| used.contains(g.id.as_str()));
        order::group_together(&mut self.tabs);
        if self.active.as_ref().is_some_and(|a| self.tab(a).is_none()) {
            self.active = None;
        }
        self.last_ratio = clamp(self.last_ratio, RATIO_RANGE, DEFAULT_RATIO);
        self.last_volume = clamp(self.last_volume, (0.0, 1.0), DEFAULT_VOLUME);
        if !self.last_show.player && !self.last_show.chat {
            self.last_show.chat = true;
        }
    }
}

fn sanitize_tile(tile: &mut Tile) {
    if !tile.show.player && !tile.show.chat {
        tile.show.chat = true;
    }
    tile.ratio = clamp(tile.ratio, RATIO_RANGE, DEFAULT_RATIO);
    tile.size = clamp(tile.size, SIZE_RANGE, 1.0);
    tile.volume = clamp(tile.volume, (0.0, 1.0), DEFAULT_VOLUME);
    tile.channel.login = tile.channel.login.trim().to_ascii_lowercase();
    if tile.channel.name.trim().is_empty() {
        tile.channel.name = tile.channel.login.clone();
    }
    tile.account = tile.account.take().filter(|a| !a.is_empty());
}

/// Scales shares so they average 1. The UI uses them as flex-grow, where shares that
/// add up to less than 1 leave part of the space empty.
fn normalize<'a>(sizes: impl Iterator<Item = &'a mut f32>) {
    let mut sizes: Vec<&mut f32> = sizes.collect();
    let sum: f32 = sizes.iter().map(|s| **s).sum();
    if sizes.is_empty() || sum <= 0.0 {
        return;
    }
    let mean = sum / sizes.len() as f32;
    for s in &mut sizes {
        **s = clamp(**s / mean, SIZE_RANGE, 1.0);
    }
}

fn clamp(v: f32, (lo, hi): (f32, f32), fallback: f32) -> f32 {
    if v.is_finite() { v.clamp(lo, hi) } else { fallback }
}

fn clip(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect::<String>().trim_end().to_owned()
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub fn tile(id: &str, channel: &str) -> Tile {
        Tile {
            id: id.into(),
            channel: TabChannel {
                id: format!("u{channel}"),
                login: channel.into(),
                name: channel.to_uppercase(),
            },
            ..Tile::default()
        }
    }

    pub fn tab(id: &str, channel: &str) -> Tab {
        Tab {
            id: id.into(),
            columns: vec![Column::of(tile(&format!("{id}-1"), channel), 1.0)],
            ..Tab::default()
        }
    }

    /// Ids "n1", "n2", … for new things.
    pub fn counter() -> impl FnMut() -> String {
        let mut n = 0;
        move || {
            n += 1;
            format!("n{n}")
        }
    }

    pub fn order(tabs: &Tabs) -> Vec<&str> {
        tabs.tabs.iter().map(|t| t.id.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[test]
    fn labels_use_the_alias_or_channel_names() {
        let mut t = tab("a", "xqc");
        assert_eq!(t.label(), "XQC");
        t.columns.push(Column::of(tile("x", "lirik"), 1.0));
        assert_eq!(t.label(), "XQC, LIRIK");
        t.alias = Some("🎮 games".into());
        assert_eq!(t.label(), "🎮 games");
    }

    #[test]
    fn sanitize_repairs_bad_values() {
        let mut bad = tab("a", "xqc");
        bad.alias = Some("   ".into());
        bad.group = Some("missing".into());
        let t = &mut bad.columns[0].tiles[0];
        t.show = TileShow {
            player: false,
            chat: false,
        };
        t.ratio = f32::NAN;
        t.size = 500.0;
        t.channel.login = " XQC ".into();
        t.channel.name.clear();
        let mut empty = tab("b", "x");
        empty.columns[0].tiles.clear();
        let mut tabs = Tabs {
            tabs: vec![bad, tab("a", "dupe"), empty],
            groups: vec![TabGroup {
                id: "g".into(),
                name: "unused".into(),
                ..TabGroup::default()
            }],
            active: Some("b".into()),
            ..Tabs::default()
        };
        tabs.sanitize();

        assert_eq!(order(&tabs), ["a"]);
        let t = &tabs.tabs[0];
        assert_eq!((t.alias.as_deref(), t.group.as_deref()), (None, None));
        let tile = &t.columns[0].tiles[0];
        assert!(tile.show.chat);
        // Clamped, then scaled back to a share of 1 as the only tile.
        assert_eq!((tile.ratio, tile.size), (DEFAULT_RATIO, 1.0));
        assert_eq!((tile.channel.login.as_str(), tile.channel.name.as_str()), ("xqc", "xqc"));
        assert!(tabs.groups.is_empty());
        assert_eq!(tabs.active, None);
    }

    #[test]
    fn random_ids_are_short_and_differ() {
        let (a, b) = (random_id(), random_id());
        assert_eq!(a.len(), 10);
        assert_ne!(a, b);
    }

    #[test]
    fn stored_tabs_round_trip() {
        let mut tabs = Tabs {
            tabs: vec![tab("a", "xqc")],
            ..Tabs::default()
        };
        tabs.tabs[0].columns[0].tiles[0].account = Some("123".into());
        let json = serde_json::to_value(&tabs).unwrap();
        assert_eq!(json["tabs"][0]["columns"][0]["tiles"][0]["channel"]["id"], "uxqc");
        assert_eq!(serde_json::from_value::<Tabs>(json).unwrap(), tabs);
    }
}
