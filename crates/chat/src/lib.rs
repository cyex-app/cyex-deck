//! Twitch chat (1.8, 1.9): IRC connection, parsing, tokenizing, room state, join budget,
//! and what phase 8 adds on top: custom commands, filters, search, rules (highlights,
//! ignores, routing), near-duplicates and repeats.

pub mod automate;
mod budget;
pub mod build;
pub mod commands;
pub mod filter;
pub mod irc;
pub mod model;
mod room;
pub mod rules;
pub mod search;
pub mod similar;
pub mod tokenize;

pub use budget::{ConnectionCount, JoinBudget, ROOM_LIMIT};
pub use room::{BUFFER, PendingSend, Room, RoomSnapshot};
