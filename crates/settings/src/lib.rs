//! Settings storage (2.8, 2.9, X6).
//!
//! One JSON file per [`Partition`] in the settings folder. The first save of each day
//! snapshots the files to `backups/yyyy-mm-dd/`; the newest [`BACKUP_DAYS`] snapshots
//! are kept. Partitions export to a file or a share code and import back after
//! validation against their schema.

mod backup;
mod clock;
mod error;
mod fsutil;
mod merge;
mod partition;
pub mod schema;
mod store;
mod transfer;

#[cfg(test)]
mod testutil;

pub use backup::BACKUP_DAYS;
pub use clock::{Clock, SystemClock};
pub use error::{Error, Result};
pub use partition::Partition;
pub use schema::Schema;
pub use store::{LoadReport, Store};
pub use transfer::{Bundle, suggested_file_name};
