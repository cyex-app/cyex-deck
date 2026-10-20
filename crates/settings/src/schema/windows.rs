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

/// Position and inner size in logical pixels, at `scale`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    /// W1 The scale the position was saved at, so `x * scale` is physical px: logical
    /// ranges of monitors with different scales overlap. `None` in saves before v1.07.
    #[ts(optional = nullable)]
    pub scale: Option<f64>,
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

#[cfg(test)]
mod tests {
    use super::Geometry;

    #[test]
    fn old_saves_have_no_scale_and_new_ones_keep_it() {
        let old: Geometry =
            serde_json::from_str(r#"{"x":1300,"y":0,"width":800,"height":600,"maximized":false}"#).unwrap();
        assert_eq!(old.scale, None);
        assert_eq!((old.x, old.width), (1300, 800));
        let new = Geometry { scale: Some(1.5), ..old };
        let json = serde_json::to_string(&new).unwrap();
        assert!(json.contains(r#""scale":1.5"#), "{json}");
        assert_eq!(serde_json::from_str::<Geometry>(&json).unwrap(), new);
    }
}
