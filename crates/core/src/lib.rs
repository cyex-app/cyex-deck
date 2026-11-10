//! Shared types and helpers used across cyex crates.

pub mod app;
pub mod color;
pub mod health;
pub mod login;
pub mod markdown;
pub mod number;
pub mod palette;
pub mod version;

#[cfg(test)]
#[path = "../../chat/src/fuzz.rs"]
mod fuzz;
#[cfg(test)]
mod fuzz_tests;
