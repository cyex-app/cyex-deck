use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// Filled in by the phase that builds it (see spec 2.6).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Highlights {}

impl Schema for Highlights {}
