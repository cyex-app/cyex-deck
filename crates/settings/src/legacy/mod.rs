//! V106-33 Shapes older versions wrote, read once and folded into today's partitions.
//! `mentions.json` (v1.04, v1.05) stays on disk untouched and is copied into backups.

pub(crate) mod fold;
pub(crate) mod mentions;

pub use fold::MergeReport;
pub(crate) use fold::{fold, needs_fold, Source};
