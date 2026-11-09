use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

const MAX_LAYOUTS: usize = 20;
/// V110-F82 Entries kept in a `panels` map, and the longest title or layout name.
const MAX_PANELS: usize = 200;
const MAX_NAME: usize = 128;

fn clip(s: &mut String, max: usize) {
    if s.chars().count() > max {
        *s = s.chars().take(max).collect();
    }
}

/// V110-F82 Every `tab/<id>`, `chat/<id>` and `modview/<login>` ever popped out stayed forever.
/// Fixed routes (no `/`) are kept first, then the rest in key order.
fn trim_panels(panels: &mut BTreeMap<String, PanelWindow>) {
    if panels.len() > MAX_PANELS {
        let mut keys: Vec<String> = panels.keys().cloned().collect();
        keys.sort_by_key(|k| k.contains('/'));
        for k in keys.into_iter().skip(MAX_PANELS) {
            panels.remove(&k);
        }
    }
    for w in panels.values_mut() {
        clip(&mut w.title, MAX_NAME);
    }
}

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
        trim_panels(&mut self.panels);
        for l in &mut self.layouts {
            clip(&mut l.name, MAX_NAME);
            trim_panels(&mut l.panels);
        }
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
    use super::*;

    #[test]
    fn v110_f82_panels_and_names_are_capped() {
        let mut w = Windows::default();
        w.panels.insert("streammanager".into(), PanelWindow::default());
        for i in 0..500 {
            w.panels.insert(format!("tab/{i:04}"), PanelWindow { title: "t".repeat(5000), ..Default::default() });
        }
        w.layouts.push(Layout { name: "n".repeat(5000), ..Default::default() });
        w.sanitize();
        assert_eq!(w.panels.len(), MAX_PANELS);
        assert!(w.panels.contains_key("streammanager"), "fixed routes stay");
        assert!(w.panels.values().all(|p| p.title.chars().count() <= MAX_NAME));
        assert_eq!(w.layouts[0].name.chars().count(), MAX_NAME);
        let again = w.clone();
        w.sanitize();
        assert_eq!(w, again);
    }

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
