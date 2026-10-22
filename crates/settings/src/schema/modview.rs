use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{ModActionFilters, PanelNode, Schema, sanitize_panels};

/// 1.53 The mod view's layout: where its panels are, and whether moving them is locked.
/// One layout for every channel, like twitch.tv. Panel keys are the UI's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Modview {
    /// Before V103-06: widget keys in display order. Only read while `layout` is unset.
    pub order: Vec<String>,
    /// Before V103-06: widget keys the user removed. Only read while `layout` is unset.
    pub hidden: Vec<String>,
    /// When on, panels can't be dragged or resized.
    pub locked: bool,
    /// V103-06 The panels as arranged; the ones missing wait in the closed-panels corner.
    /// `None` is the UI's default.
    #[ts(optional = nullable)]
    pub layout: Option<PanelNode>,
    /// V103-15 Which mod actions its "Mod actions" panel shows.
    pub mod_actions: ModActionFilters,
    /// V103-22 The chat panel brought back (`true`) or closed (`false`) by hand: it stays so
    /// until "Reset layout". `None` follows the channel's regular chat.
    #[ts(optional = nullable)]
    pub chat_kept: Option<bool>,
    /// V107-R29 Panels popped out of the layout, by `<login>/<panel>`: the layout as it was
    /// then, so closing the window puts the panel back in its exact place. A panel popped out
    /// from the closed-panels row has no entry: it goes back there.
    /// V107-Mg Per channel: the layout stays shared and keeps the panel, so other channels'
    /// views still show it.
    pub popped: BTreeMap<String, PanelNode>,
}

impl Schema for Modview {
    fn sanitize(&mut self) {
        self.layout = sanitize_panels(self.layout.take());
        self.popped = std::mem::take(&mut self.popped)
            .into_iter()
            .filter_map(|(k, v)| sanitize_panels(Some(v)).map(|v| (k, v)))
            .collect();
    }
}
