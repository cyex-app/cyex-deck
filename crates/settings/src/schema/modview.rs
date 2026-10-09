use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// Filled in by the phase that builds it (see spec 09).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Modview {}

impl Schema for Modview {}
