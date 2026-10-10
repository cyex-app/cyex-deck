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
mod state;

#[cfg(test)]
mod tests;

pub use convert::color;
pub use host::{
    Account, Change, ChannelInfo, Completion, CompletionRequest, Flags, Host, HttpRequest, HttpResponse,
    Level, Line, ReplyTo, RoomModes, Sender, Sent, StreamStatus,
};
pub use manifest::{Field, FieldKind, Manifest, Permission};
pub use perm::{Granted, check_url};
pub use runtime::{CALLBACK, Config, Event, Exports, LINE, Plugin, QUICK, Reply};
