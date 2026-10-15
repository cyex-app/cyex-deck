//! Accounts and sign-in (spec 01): Device Code sign-in, pasted tokens, token storage in
//! the Windows Credential Manager, validation and refresh, capability gating (1.54).
//! No Tauri here; `app/src/auth` wires it up.

pub mod caps;
mod chat;
pub mod device;
mod error;
pub mod inspect;
pub mod keeper;
mod kind;
pub mod paste;
pub mod roster;
pub mod scopes;
pub mod secrets;
mod seventv;
mod vault;
mod view;

pub use caps::Feature;
pub use chat::{chat_token, chat_token_kind};
pub use error::AuthError;
pub use kind::TokenKind;
pub use secrets::Secret;
pub use seventv::{SevenTvToken, SevenTvTokens, date as seventv_date};
pub use vault::{TokenState, Vault};
pub use view::{AccountView, SevenTvView, TokenView, views};
