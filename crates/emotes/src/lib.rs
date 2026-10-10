//! Third-party emotes, badges and name paints (1.37-1.39): 7TV, BTTV and FFZ over HTTP,
//! live updates over their sockets, and the lookups chat uses (C7, C9, C11).

pub mod bttv;
pub mod cosmetics;
pub mod ffz;
pub mod index;
pub mod model;
pub mod seventv;
pub mod ws;

pub use cosmetics::{BadgeDef, BadgeRef, Cosmetics, Look, Looks};
pub use index::{Lookup, RoomEmotes};
pub use model::{Emote, EmoteSet, Scope};
