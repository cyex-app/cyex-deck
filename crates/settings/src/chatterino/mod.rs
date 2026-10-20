//! V1041-02 Tabs from a Chatterino layout: read the file ([`parse`]), look the channels
//! up ([`resolve`]), then work out what goes in ([`plan`]). Adding them is
//! [`Tabs::import`](crate::schema::Tabs::import). V105-16 Highlights from its
//! settings.json: [`parse_highlights`], [`plan_highlights`], [`apply_highlights`].

mod highlights;
mod layout;
mod plan;
mod resolve;

pub use highlights::{
    Found, HighlightImport, HighlightPreview, NotSettings, Skipped, apply as apply_highlights, parse as parse_highlights,
    plan as plan_highlights,
};
pub use layout::{Layout, LayoutColumn, LayoutSplit, LayoutTab, NotALayout, SplitKind, parse};
pub use plan::{ImportReport, NotChannel, Plan, plan};
pub use resolve::{BATCH, is_login, known, names, resolve};
