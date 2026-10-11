//! EventSub over a websocket (phase 10): the session (`session`), its frames (`frame`),
//! what to subscribe to for a channel (`subs`) and the events, typed (`events`).
//! Subscribing goes through Helix (`cyex_twitch::helix::eventsub`). The same activity
//! from Twitch's dashboard topics on Hermes is read in `alerts` (D30).

pub mod alerts;
pub mod frame;
pub mod session;
pub mod subs;
pub mod events;
