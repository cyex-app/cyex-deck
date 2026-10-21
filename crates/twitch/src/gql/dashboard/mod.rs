//! 1.35 The stream manager over GQL, the way Twitch's creator dashboard runs it (V101-28,
//! D30). Every shape was checked against the live schema on 2026/10/11; no write was run.
//! Answers reuse the Helix types, so the UI doesn't care which API ran. V107-R41 every
//! call goes through one [`Channel`] and one refusal rule (`channel`).

pub mod ads;
mod channel;
pub mod community;
pub mod content;
pub mod feed;
pub mod info;
#[cfg(test)]
pub(crate) mod mock;
pub mod modes;
pub mod modset;
pub mod raids;
pub mod rewards;
pub mod roles;
pub mod stats;
pub mod together;
pub mod votes;

pub use channel::{Channel, NOT_DONE};
