use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// 1.53 The mod view's layout: which widgets show, in what order, and whether dragging is
/// locked. One layout for every channel, like twitch.tv. Widget keys are the UI's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Modview {
    /// Widget keys in display order; empty means the UI's default order.
    pub order: Vec<String>,
    /// Widget keys the user removed from the grid.
    pub hidden: Vec<String>,
    /// When on, widgets can't be dragged or resized.
    pub locked: bool,
}

impl Schema for Modview {}
