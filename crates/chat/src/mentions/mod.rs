//! V104-01, V104-02 Mentions: words or patterns that ping you, each with its own options,
//! in open chats and (global ones) in logs.cyex.app's firehose. One engine for every
//! regex flavor (`flavor`), look-alike folding from `crate::lookalike`.

pub mod flavor;
mod matcher;
mod peek;

pub use matcher::{Compiled, MentionCheck, MentionSet, check};
pub(crate) use matcher::{BACKTRACK_LIMIT, SIZE_LIMIT, short_error};
pub use peek::{Peek, peek};

/// The feed tab (F4) "show in Mentions" copies messages into.
pub const FEED: &str = "Mentions";
