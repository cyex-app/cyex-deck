//! Twitch API clients. One shared HTTP client; callers pass tokens in, nothing here
//! stores them. Also the third-party services chat uses: recent messages (C1),
//! Chatterino's link resolver (D8), translation (C17) and logs.cyex.app (1.36, its firehose
//! V104-02).

pub mod channel_input;
pub mod clients;
pub mod clock;
pub mod firehose;
pub mod giphy;
pub mod github;
mod error;
pub mod gql;
pub mod helix;
pub mod hermes;
mod http;
pub mod id;
pub mod logs;
pub mod pace;
pub mod random;
pub mod recent;
pub mod resolver;
pub mod spade;
pub mod translate;
pub mod uploader;
pub mod usher;
pub mod ws;

pub use error::{Error, Result};
pub use http::{Http, RateLimit};
