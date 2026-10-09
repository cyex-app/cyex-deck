//! Twitch API clients. One shared HTTP client; callers pass tokens in, nothing here
//! stores them. Also the two third-party services chat uses: recent messages (C1) and
//! Chatterino's link resolver (D8).

pub mod channel_input;
pub mod clients;
mod error;
pub mod gql;
pub mod helix;
mod http;
pub mod id;
pub mod random;
pub mod recent;
pub mod resolver;
pub mod usher;

pub use error::{Error, Result};
pub use http::Http;
