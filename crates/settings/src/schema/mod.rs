//! Typed shape of every partition. Each feature phase adds its fields to its own file.
//! All structs use `#[serde(default)]`, so missing fields fall back to defaults.

mod accounts;
mod appearance;
mod browse;
mod chat;
mod commands;
mod emotes;
mod general;
mod highlights;
mod hotkeys;
mod miner;
mod modview;
mod notifications;
mod player;
mod plugins;
mod streammanager;
mod tabs;
mod windows;

pub use accounts::{Account, Accounts, TokenMeta, TokenSource};
pub use appearance::{
    Appearance, CustomTheme, TabBarSide, TabDensity, TabUnread, Wallpaper, WallpaperArea, WallpaperFit, preset_themes,
};
pub use browse::{Browse, HomeLayout, LiveSort, OfflineSort};
pub use chat::{
    BadgeToggles, Chat, DeletedStyle, GifMode, LinkThumbs, NameStyle, Overlay, PaintToggles, PauseKey,
    Timestamps, UnlistedEmotes, UserNote,
};
pub use commands::{Access, Automation, Commands, SavedTimer, TriggerMatch};
pub use emotes::{EmoteKey, EmoteSource, EmoteUse, Emotes};
pub use general::{ChatPosition, General, StreamerMode, StreamerWhen};
pub use highlights::{
    Alert, AppRule, BadgeRule, Blocked, BlockedShow, BuiltIn, FilterDef, HiddenUser, Highlights, IgnoreRule, PhraseRule,
    SentFrom, Similarity, UserRule,
};
pub use hotkeys::Hotkeys;
pub use miner::{BetFilter, Bets, ChatPresence, DelayMode, FilterBy, FilterCmp, MAX_FIRST, MineSource, Miner, Rule, Strategy};
pub use modview::Modview;
pub use notifications::Notifications;
pub use player::{Player, Quality};
pub use plugins::{PluginEntry, Plugins};
pub use streammanager::{ActivityFilters, Obs, StreamManager};
pub use tabs::{
    Closed, Column, GroupColor, Resolved, Tab, TabChannel, TabError, TabGroup, TabOp, Tabs, Tile, TileShow, Zone, random_id,
};
pub use windows::{Geometry, Layout, PanelWindow, Windows};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait Schema: Serialize + DeserializeOwned + Default {
    /// Clamp or repair values serde accepted but the app can't use.
    fn sanitize(&mut self) {}
}
