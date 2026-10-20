//! Types the UI renders. Exported to TypeScript by ts-rs.

mod fragment;
mod message;
mod notice;
mod quote;
mod role;
mod room;

pub use fragment::{Act, EmoteLook, Fragment, Layer, LinkTarget, Provider, SpanColor, SpanStyle, mods, plain};
pub use message::{
    Badge, ChatMessage, Client, Flag, Gone, GoneHow, Highlight, HighlightKind, Meta, NameColor, PluginTag, Reply,
    RewardRef, Sender, SharedSource, Translation,
};
pub use notice::{Notice, NoticeKind};
pub use quote::{QUOTE_MAX, quote};
pub(crate) use role::MOD_SETS;
pub use role::ModRole;
pub use room::{ConnStatus, Pinned, RoomEvent, RoomState, SelfState};
