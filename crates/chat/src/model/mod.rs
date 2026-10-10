//! Types the UI renders. Exported to TypeScript by ts-rs.

mod fragment;
mod message;
mod notice;
mod room;

pub use fragment::{EmoteLook, Fragment, Layer, LinkTarget, Provider, mods, plain};
pub use message::{Badge, ChatMessage, Flag, NameColor, Reply, RewardRef, Sender, SharedSource};
pub use notice::{Notice, NoticeKind};
pub use room::{ConnStatus, Pinned, RoomEvent, RoomState, SelfState};
