//! Twitch IRC over TLS (ARCHITECTURE §8): line parsing, outgoing lines, rate limits and
//! the connection task.

mod client;
mod limiter;
mod line;
pub mod write;

pub use client::{Anonymous, BoxStream, Client, Connect, Credentials, Event, LinkStatus, Login, tls};
pub use limiter::Window;
pub use line::{Line, escape, unescape};
