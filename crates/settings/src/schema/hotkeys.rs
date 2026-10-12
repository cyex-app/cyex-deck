use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

/// S2 Rebound hotkeys only. Defaults live with the actions in `ui/src/lib/hotkeys.svelte.ts`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Hotkeys {
    /// Action id → combo like `Ctrl+Shift+K`. Empty combo = unbound.
    pub bindings: BTreeMap<String, String>,
}

impl Schema for Hotkeys {
    fn sanitize(&mut self) {
        self.bindings.retain(|action, _| !action.trim().is_empty());
    }
}
