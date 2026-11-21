//! Shared types and helpers used across cyex crates.

pub mod app;
pub mod backoff;
pub mod color;
pub mod health;
pub mod login;
pub mod markdown;
pub mod number;
pub mod palette;
pub mod version;

/// V112-23 cyex.tv is "coming soon": its emotes, live socket, cosmetics, personal sets, sign-in and
/// commands are all in the code but nothing of it runs, asks the network or shows in chat while this
/// is false. The UI mirrors it as `CYEXTV_READY` in `ui/src/lib/cyextv.ts`. Flip both to `true`
/// to bring everything back unchanged.
pub const CYEXTV_READY: bool = false;

#[cfg(test)]
#[path = "../../chat/src/fuzz.rs"]
mod fuzz;
#[cfg(test)]
mod fuzz_tests;
