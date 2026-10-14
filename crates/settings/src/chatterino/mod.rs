//! V1041-02 Tabs from a Chatterino layout: read the file ([`parse`]), look the channels
//! up ([`resolve`]), then work out what goes in ([`plan`]). Adding them is
//! [`Tabs::import`](crate::schema::Tabs::import).

mod layout;
mod plan;
mod resolve;

pub use layout::{Layout, LayoutTab, NotALayout, SplitKind, parse};
pub use plan::{ImportReport, NotChannel, Plan, plan};
pub use resolve::{BATCH, is_login, names, resolve};
