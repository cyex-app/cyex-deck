//! Twitch API clients. One shared HTTP client; callers pass tokens in, nothing here
//! stores them. Also the two third-party services chat uses: recent messages (C1) and
//! Chatterino's link resolver (D8).

pub mod clients;
mod error;
pub mod gql;
pub mod helix;
mod http;
pub mod id;
mod random;
pub mod recent;
pub mod resolver;

pub use error::{Error, Result};
pub use http::Http;
