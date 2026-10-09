//! Typed shape of every partition. Each feature phase adds its fields to its own file.
//! All structs use `#[serde(default)]`, so missing fields fall back to defaults.

mod accounts;
mod appearance;
mod chat;
mod commands;
mod general;
mod highlights;
mod hotkeys;
mod miner;
mod modview;
mod notifications;
mod player;
mod tabs;
mod windows;

pub use accounts::{Account, Accounts, TokenMeta, TokenSource};
pub use appearance::{Appearance, TabBarSide, TabDensity, TabUnread};
pub use chat::{
    BadgeToggles, Chat, DeletedStyle, GifMode, NameStyle, PaintToggles, PauseKey, Timestamps, UnlistedEmotes, UserNote,
};
pub use commands::Commands;
pub use general::{ChatPosition, General};
pub use highlights::Highlights;
pub use hotkeys::Hotkeys;
pub use miner::Miner;
pub use modview::Modview;
pub use notifications::Notifications;
pub use player::{Player, Quality};
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
