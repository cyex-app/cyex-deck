//! Typed shape of every partition. Each feature phase adds its fields to its own file.
//! All structs use `#[serde(default)]`, so missing fields fall back to defaults.

mod accounts;
mod appearance;
mod browse;
mod builtin_themes;
mod chat;
mod combos;
mod commands;
mod emotes;
mod general;
mod highlights;
mod hotkeys;
mod miner;
mod modview;
mod notifications;
mod panels;
mod player;
mod plugins;
mod sound;
mod streammanager;
mod tabs;
mod uploader;
mod windows;

pub use accounts::{Account, Accounts, SevenTvConsent, TokenMeta, TokenSource};
pub use appearance::{
    Appearance, CustomTheme, MAX_THEMES, TabBarSide, TabDensity, TabRows, TabUnread, Wallpaper, WallpaperArea, WallpaperFit,
};
pub use builtin_themes::{builtin_theme, builtin_themes, is_builtin_id};
pub use browse::{Browse, HomeLayout, LiveSort, OfflineSort};
pub use chat::{
    BadgeToggles, Chat, DeletedStyle, GifMode, LinkThumbs, ModButtons, NameStyle, Overlay, PaintToggles,
    PauseKey, Timestamps, UnlistedEmotes, UserNote,
};
pub use combos::{
    Combo, ComboDone, ComboError, ComboItem, ComboOp, Combos, MAX_COMBOS, MAX_NAME, NameError, WORDS, clean_name,
};
pub use commands::{Access, Automation, CommandRow, Commands, SavedTimer, TriggerMatch};
pub use emotes::{EmoteKey, EmoteSource, EmoteUse, Emotes};
pub use general::{ChatPosition, FONTS, General, font_or_default, StreamerMode, StreamerWhen};
pub(crate) use highlights::{MAX_GLOBAL, MAX_RULES, MAX_WORDS};
pub use highlights::{
    Alert, AppRule, BadgeRule, Blocked, BlockedShow, BuiltIn, FilterDef, HiddenUser, Highlights, IgnoreRule, PhraseRule,
    RegexFlavor, SOUND_TYPES, SentFrom, Similarity, UserRule, is_sound_file,
};
pub use hotkeys::Hotkeys;
pub use miner::{BetFilter, Bets, ChatPresence, DelayMode, FilterBy, FilterCmp, MAX_FIRST, MineSource, Miner, Rule, Strategy};
pub use modview::Modview;
pub use notifications::Notifications;
pub use panels::{PanelNode, SplitDir, sanitize as sanitize_panels};
pub use player::{Corner, Player, Quality};
pub use plugins::{PluginEntry, Plugins};
pub use sound::{OutputDevice, Sound};
pub use streammanager::{ActivityFilters, ModActionFilters, Obs, StreamManager};
pub use tabs::{
    Closed, Column, FeedKind, GroupColor, ImportColumn, ImportTab, Resolved, Tab, TabChannel, TabError, TabGroup, TabOp, Tabs, Tile, TileShow, Zone,
    random_id,
};
pub use uploader::Uploader;
pub use windows::{Geometry, Layout, PanelWindow, Windows};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait Schema: Serialize + DeserializeOwned + Default {
    /// Clamp or repair values serde accepted but the app can't use.
    fn sanitize(&mut self) {}
}
