use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct General {
    /// 2.1 Chat font family. Empty uses the app font.
    pub chat_font_family: String,
    /// 2.1 Chat font size in px.
    pub chat_font_size: u8,
    /// 2.1 Side the chat sits on, next to the player or a clip (1.19).
    pub chat_position: ChatPosition,
    /// 3.6, 2.7 Tooltips that explain the app.
    pub tooltips: bool,
    /// U2 Minimizing the main window hides it to the tray.
    pub minimize_to_tray: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            chat_font_family: String::new(),
            chat_font_size: 13,
            chat_position: ChatPosition::Right,
            tooltips: true,
            minimize_to_tray: false,
        }
    }
}

impl Schema for General {
    fn sanitize(&mut self) {
        self.chat_font_size = self.chat_font_size.clamp(9, 32);
        self.chat_font_family = self.chat_font_family.trim().chars().take(100).collect();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChatPosition {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}
