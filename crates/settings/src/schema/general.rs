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
    /// V101-10 Font files the user added, offered next to the bundled ones.
    pub fonts: Vec<CustomFont>,
    /// 2.1 Side the chat sits on, next to the player or a clip (1.19).
    pub chat_position: ChatPosition,
    /// 3.6, 2.7 Tooltips that explain the app.
    pub tooltips: bool,
    /// U2 Minimizing the main window hides it to the tray.
    pub minimize_to_tray: bool,
    /// V103-11 The mouse wheel glides in every scrollable area, as in Chatterino. The page
    /// does it, so a change applies at once.
    pub smooth_scroll: bool,
    /// U4
    pub streamer: StreamerMode,
}

impl Default for General {
    fn default() -> Self {
        Self {
            chat_font_family: String::new(),
            chat_font_size: 13,
            fonts: Vec::new(),
            chat_position: ChatPosition::Right,
            tooltips: true,
            minimize_to_tray: false,
            smooth_scroll: true,
            streamer: StreamerMode::default(),
        }
    }
}

impl Schema for General {
    fn sanitize(&mut self) {
        self.chat_font_size = self.chat_font_size.clamp(9, 32);
        self.chat_font_family = self.chat_font_family.trim().chars().take(100).collect();
        let mut names: Vec<String> = Vec::new();
        self.fonts.retain_mut(|f| {
            f.name = font_name(&f.name);
            let key = f.name.to_lowercase();
            let keep = !f.name.is_empty() && f.is_file() && !names.contains(&key);
            names.push(key);
            keep
        });
        self.fonts.truncate(MAX_FONTS);
        let apps = &mut self.streamer.apps;
        for a in apps.iter_mut() {
            *a = a.trim().chars().take(MAX_APP_CHARS).collect();
        }
        let mut seen: Vec<String> = Vec::new();
        apps.retain(|a| {
            let key = a.to_lowercase();
            let keep = !a.is_empty() && !seen.contains(&key);
            seen.push(key);
            keep
        });
        apps.truncate(MAX_APPS);
    }
}

const MAX_APPS: usize = 30;
const MAX_APP_CHARS: usize = 80;
const MAX_FONTS: usize = 20;
const MAX_FONT_NAME_CHARS: usize = 60;
const MAX_FONT_FILE_STEM: usize = 32;

/// V101-10 A font file the user added. The file sits in `fonts\` next to the settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct CustomFont {
    /// What the font is called in the list and in CSS.
    pub name: String,
    /// `<id>.<ext>` in the fonts folder.
    pub file: String,
}

impl CustomFont {
    /// Only a plain file name the app made: no paths, a known font type.
    pub fn is_file(&self) -> bool {
        let Some((stem, ext)) = self.file.rsplit_once('.') else {
            return false;
        };
        !stem.is_empty()
            && stem.len() <= MAX_FONT_FILE_STEM
            && stem.chars().all(|c| c.is_ascii_alphanumeric())
            && FONT_TYPES.contains(&ext)
    }
}

/// A font's name as CSS gets it: no quotes, braces or other characters that end a rule.
pub fn font_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | '\'' | '\\' | ';' | '{' | '}' | '<' | '>'))
        .collect();
    kept.trim().chars().take(MAX_FONT_NAME_CHARS).collect()
}

/// Font files a page can load.
pub const FONT_TYPES: [&str; 4] = ["ttf", "otf", "woff", "woff2"];

/// U4 Streamer mode hides what viewers of your stream shouldn't see.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct StreamerMode {
    pub when: StreamerWhen,
    /// Exe names that count as streaming for `Auto`.
    pub apps: Vec<String>,
    /// Link previews, thumbnails and cards.
    pub hide_link_previews: bool,
    /// Profile pictures on usercards.
    pub hide_avatars: bool,
    /// Timeouts, bans and deleted-message lines.
    pub hide_mod_actions: bool,
    /// Whispers in chat and their pop-ups.
    pub hide_whispers: bool,
    /// Windows notifications.
    pub no_notifications: bool,
    /// Mention, highlight and whisper sounds.
    pub mute_sounds: bool,
}

impl Default for StreamerMode {
    fn default() -> Self {
        Self {
            when: StreamerWhen::Auto,
            apps: DEFAULT_APPS.iter().map(|a| (*a).to_owned()).collect(),
            hide_link_previews: true,
            hide_avatars: true,
            hide_mod_actions: true,
            hide_whispers: true,
            no_notifications: true,
            mute_sounds: false,
        }
    }
}

/// Streaming and recording apps Chatterino also looks for, plus Streamlabs.
const DEFAULT_APPS: [&str; 8] = [
    "obs64.exe",
    "obs32.exe",
    "obs.exe",
    "Streamlabs OBS.exe",
    "PRISMLiveStudio.exe",
    "XSplit.Core.exe",
    "TwitchStudio.exe",
    "vMix64.exe",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StreamerWhen {
    Off,
    On,
    /// While one of the apps runs.
    #[default]
    Auto,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn font(name: &str, file: &str) -> CustomFont {
        CustomFont {
            name: name.into(),
            file: file.into(),
        }
    }

    #[test]
    fn custom_fonts_keep_plain_files_and_one_of_each_name() {
        let mut g = General {
            fonts: vec![
                font(" My \"Font\" ", "abc123.ttf"),
                font("my font", "def456.otf"),
                font("Path", "../evil.ttf"),
                font("Exe", "abc.exe"),
                font("", "aaa.woff2"),
                font("Web", "bbb.woff2"),
            ],
            ..General::default()
        };
        g.sanitize();
        assert_eq!(g.fonts, vec![font("My Font", "abc123.ttf"), font("Web", "bbb.woff2")]);
    }
}
