//! Twitch API clients. One shared HTTP client; callers pass tokens in, nothing here
//! stores them. Also the third-party services chat uses: recent messages (C1),
//! Chatterino's link resolver (D8) and our own (V110-U70), translation (C17) and logs.cyex.app
//! (1.36, its firehose V104-02).

mod call;
pub mod breaker;
pub mod channel_input;
pub mod clients;
pub mod clock;
mod download;
pub mod embed;
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
pub mod preview;
pub mod random;
pub mod recent;
pub mod resolver;
pub mod spade;
pub mod translate;
pub mod uploader;
pub mod usher;
pub mod ws;

#[cfg(test)]
#[path = "../../chat/src/fuzz.rs"]
mod fuzz;
#[cfg(test)]
#[path = "../../chat/src/fuzz_json.rs"]
mod fuzz_json;
#[cfg(test)]
mod fuzz_tests;

pub use call::{Call, Reply, Verb};
pub use download::DownloadError;
pub use error::{Error, Result, already_text, integrity_text};
pub use http::{BROWSER_AGENT, Http, RateLimit};
