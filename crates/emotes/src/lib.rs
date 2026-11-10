//! Third-party emotes, badges and name paints (1.37-1.39): 7TV, BTTV and FFZ over HTTP,
//! live updates over their sockets, and the lookups chat uses (C7, C9, C11).

pub mod badges;
pub mod bttv;
pub mod capped;
pub mod cosmetics;
pub mod ffz;
pub mod index;
pub mod model;
pub mod seventv;

pub use capped::Capped;
pub use cosmetics::{BadgeDef, BadgeRef, Cosmetics, Look, Looks};
pub use index::{Lookup, RoomEmotes};
pub use model::{Emote, EmoteSet, Scope};

#[cfg(test)]
#[path = "../../chat/src/fuzz.rs"]
mod fuzz;
#[cfg(test)]
#[path = "../../chat/src/fuzz_json.rs"]
mod fuzz_json;
#[cfg(test)]
mod fuzz_tests;
