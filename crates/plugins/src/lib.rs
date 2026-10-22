//! S5 Lua plugins: a folder with `info.json` and `init.lua`, run in Lua 5.4 on a thread of
//! its own, with only the permissions the user agreed to. The app implements [`Host`].

mod api;
mod convert;
pub mod fetch;
pub mod host;
pub mod manifest;
pub mod perm;
mod repl;
mod runtime;
mod sandbox;
pub mod socket;
mod state;

#[cfg(test)]
mod tests;

pub use convert::color;
pub use host::{
    Account, Change, ChannelInfo, ChatEvent, Completion, CompletionRequest, Emit, Flags, Host, HttpRequest, HttpResponse,
    Layout, LayoutTab, LayoutTile, Level, Line, LineLink, LiveChange, NameUser, NewLine, Piece, PieceColor, PieceStyle,
    ReplyTo, RoomModes, Sender, Sent, SocketEvent, SocketOut, SocketRequest, StreamStatus,
};
pub use manifest::{Field, FieldKind, Manifest, Permission};
pub use perm::{Granted, check_url};
pub use state::{SEND_LIMIT, SEND_WINDOW};
pub use runtime::{CALLBACK, Config, Event, Exports, LINE, MenuExport, MenuPlace, Plugin, QUICK, Reply};
