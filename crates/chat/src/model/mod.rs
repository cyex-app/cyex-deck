//! Types the UI renders. Exported to TypeScript by ts-rs.

mod fragment;
mod message;
mod notice;
mod role;
mod room;

pub use fragment::{Act, EmoteLook, Fragment, Layer, LinkTarget, Provider, SpanColor, SpanStyle, mods, plain};
pub use message::{
    Badge, ChatMessage, Client, Flag, Highlight, HighlightKind, Meta, NameColor, PluginTag, Reply, RewardRef, Sender,
    SharedSource, Translation,
};
pub use notice::{Notice, NoticeKind};
pub(crate) use role::MOD_SETS;
pub use role::ModRole;
pub use room::{ConnStatus, Pinned, RoomEvent, RoomState, SelfState};
