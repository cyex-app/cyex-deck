use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

const MAX_LAYOUTS: usize = 20;

/// Window positions and pinned state (1.51), plus saved layouts (U2).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Windows {
    pub main: Option<Geometry>,
    pub main_pinned: bool,
    /// Last state of each popped-out panel, keyed by panel route (e.g. `streammanager`).
    pub panels: BTreeMap<String, PanelWindow>,
    pub layouts: Vec<Layout>,
}

impl Schema for Windows {
    fn sanitize(&mut self) {
        self.layouts.retain(|l| !l.name.trim().is_empty());
        self.layouts.truncate(MAX_LAYOUTS);
    }
}

/// Position and inner size in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct PanelWindow {
    pub title: String,
    pub geometry: Option<Geometry>,
    pub pinned: bool,
}

/// A named snapshot of the main window and every open pop-out.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Layout {
    pub name: String,
    pub main: Option<Geometry>,
    pub panels: BTreeMap<String, PanelWindow>,
}
