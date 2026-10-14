//! Twitch chat (1.8, 1.9): IRC connection, parsing, tokenizing, room state,
//! and what phase 8 adds on top: custom commands, filters, search, rules (highlights,
//! ignores, routing), near-duplicates and repeats; v1.04 mentions and look-alike folding.

pub mod automate;
pub mod build;
pub mod commands;
pub mod filter;
pub mod irc;
pub mod lookalike;
pub mod mentions;
pub mod model;
mod room;
pub mod rules;
pub mod search;
pub mod similar;
pub mod tokenize;

pub use room::{BUFFER, LineChange, LineHook, PendingSend, Room, RoomSnapshot};
