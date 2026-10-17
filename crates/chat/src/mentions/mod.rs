//! V104-01, V104-02, V106-33 Word rules (Highlights words, global mentions): words or
//! patterns that light up a message or ping you, each with its own options, in open chats
//! and (global ones) in logs.cyex.app's firehose. One engine for every regex flavor (`flavor`), look-alike folding from `crate::lookalike`.

pub mod flavor;
mod matcher;
mod peek;

pub use matcher::{Compiled, MentionCheck, MentionSet, check, check_all};
pub(crate) use matcher::{BACKTRACK_LIMIT, SIZE_LIMIT, short_error};
pub use peek::{Peek, peek};

/// The feed tab (F4) "show in Mentions" copies messages into.
pub const FEED: &str = "Mentions";
