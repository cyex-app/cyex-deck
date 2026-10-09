use cyex_core::color::{DEFAULT_ACCENT, Theme, normalize_accent};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Appearance {
    /// 3.3
    pub theme: Theme,
    /// 3.3, 3.4 `#rrggbb`. Derived tokens come from `cyex_core::color::accent_tokens`.
    pub accent: String,
    /// 1.24
    pub tab_density: TabDensity,
    /// 1.25 Middle click still closes a tab when this is off.
    pub tab_close_button: bool,
    /// U5
    pub tab_bar_side: TabBarSide,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: DEFAULT_ACCENT.into(),
            tab_density: TabDensity::Normal,
            tab_close_button: true,
            tab_bar_side: TabBarSide::Top,
        }
    }
}

impl Schema for Appearance {
    fn sanitize(&mut self) {
        self.accent = normalize_accent(&self.accent).unwrap_or_else(|| DEFAULT_ACCENT.into());
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TabDensity {
    Comfortable,
    #[default]
    Normal,
    Compact,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TabBarSide {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}
