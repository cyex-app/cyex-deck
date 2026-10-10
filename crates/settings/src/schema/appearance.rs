use std::collections::BTreeMap;

use cyex_core::color::{DEFAULT_ACCENT, Theme, normalize_accent};
use cyex_core::palette::KEYS;
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
    /// U9 What lights up a tab in the background.
    pub tab_unread: TabUnread,
    /// U8 Themes made in the theme editor.
    pub themes: Vec<CustomTheme>,
    /// U8 The custom theme in use; empty for plain dark or light. `theme` follows its base.
    pub theme_id: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: DEFAULT_ACCENT.into(),
            tab_density: TabDensity::Normal,
            tab_close_button: true,
            tab_bar_side: TabBarSide::Top,
            tab_unread: TabUnread::All,
            themes: Vec::new(),
            theme_id: String::new(),
        }
    }
}

impl Schema for Appearance {
    fn sanitize(&mut self) {
        self.accent = normalize_accent(&self.accent).unwrap_or_else(|| DEFAULT_ACCENT.into());
        let mut seen = Vec::new();
        self.themes.retain_mut(|t| {
            t.sanitize();
            let fresh = !t.id.is_empty() && !seen.contains(&t.id);
            seen.push(t.id.clone());
            fresh
        });
        self.themes.truncate(MAX_THEMES);
        match self.active_theme().map(|t| t.base) {
            Some(base) => self.theme = base,
            None => self.theme_id.clear(),
        }
    }
}

impl Appearance {
    /// U8 The custom theme in use, if any.
    pub fn active_theme(&self) -> Option<&CustomTheme> {
        if self.theme_id.is_empty() {
            return None;
        }
        self.themes.iter().find(|t| t.id == self.theme_id)
    }

    /// The accent in use: the custom theme's, else the plain one.
    pub fn active_accent(&self) -> &str {
        self.active_theme().map_or(&self.accent, |t| &t.accent)
    }
}

const MAX_THEMES: usize = 30;
const MAX_THEME_NAME_CHARS: usize = 40;
const MAX_ID_CHARS: usize = 32;
const MAX_URL_CHARS: usize = 2048;
/// U8 Highest dim and blur a wallpaper takes.
const MAX_DIM: u8 = 95;
const MAX_BLUR: u8 = 30;

/// U8 A theme from the theme editor: a few colors on top of dark or light, its own accent
/// and an optional wallpaper. Colors that would hurt readability are nudged when applied
/// (`cyex_core::palette`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct CustomTheme {
    pub id: String,
    pub name: String,
    pub base: Theme,
    /// `bg`, `text`, … (`cyex_core::palette::KEYS`) → `#rrggbb`. Missing ones use the base's.
    pub colors: BTreeMap<String, String>,
    pub accent: String,
    pub wallpaper: Wallpaper,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base: Theme::Dark,
            colors: BTreeMap::new(),
            accent: DEFAULT_ACCENT.into(),
            wallpaper: Wallpaper::default(),
        }
    }
}

impl CustomTheme {
    pub fn sanitize(&mut self) {
        self.id = self
            .id
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(MAX_ID_CHARS)
            .collect();
        self.name = self.name.trim().chars().take(MAX_THEME_NAME_CHARS).collect();
        if self.name.is_empty() {
            self.name = "My theme".into();
        }
        self.colors = std::mem::take(&mut self.colors)
            .into_iter()
            .filter(|(k, _)| KEYS.contains(&k.as_str()))
            .filter_map(|(k, v)| normalize_accent(&v).map(|v| (k, v)))
            .collect();
        self.accent = normalize_accent(&self.accent).unwrap_or_else(|| DEFAULT_ACCENT.into());
        let w = &mut self.wallpaper;
        w.source = w.source.trim().to_owned();
        if !(w.is_link() || w.is_file()) {
            w.source.clear();
        }
        w.dim = w.dim.min(MAX_DIM);
        w.blur = w.blur.min(MAX_BLUR);
    }
}

/// U8 A picture behind the window or the chat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Wallpaper {
    /// An https link, or a file name in the app's `wallpapers` folder. Empty for none.
    pub source: String,
    pub area: WallpaperArea,
    /// How much of the theme's background covers the picture, in percent.
    pub dim: u8,
    /// Blur in px.
    pub blur: u8,
    pub fit: WallpaperFit,
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            source: String::new(),
            area: WallpaperArea::Window,
            dim: 70,
            blur: 0,
            fit: WallpaperFit::Cover,
        }
    }
}

impl Wallpaper {
    /// An https link that's safe inside CSS `url("…")`.
    pub fn is_link(&self) -> bool {
        self.source.len() <= MAX_URL_CHARS
            && self.source.starts_with("https://")
            && !self
                .source
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '(' | ')' | '\\'))
    }

    /// A file the app copied in: `<letters and digits>.<png|jpg|jpeg|webp|gif>`.
    pub fn is_file(&self) -> bool {
        let Some((stem, ext)) = self.source.rsplit_once('.') else {
            return false;
        };
        !stem.is_empty()
            && stem.len() <= MAX_ID_CHARS
            && stem.chars().all(|c| c.is_ascii_alphanumeric())
            && matches!(ext, "png" | "jpg" | "jpeg" | "webp" | "gif")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WallpaperArea {
    /// Behind home, tabs and chat; cards and menus stay solid.
    #[default]
    Window,
    /// Behind chat messages only.
    Chat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WallpaperFit {
    #[default]
    Cover,
    Contain,
    Tile,
}

/// U8 Ready-made themes to start from.
pub fn preset_themes() -> Vec<CustomTheme> {
    let theme = |name: &str, base: Theme, accent: &str, colors: &[(&str, &str)]| CustomTheme {
        id: String::new(),
        name: name.into(),
        base,
        colors: colors
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        accent: accent.into(),
        wallpaper: Wallpaper::default(),
    };
    vec![
        theme(
            "Midnight",
            Theme::Dark,
            DEFAULT_ACCENT,
            &[
                ("bg", "#0e0f13"),
                ("surface", "#15171c"),
                ("elevated", "#1c1f26"),
                ("border", "#23262e"),
                ("text", "#e6e8ee"),
                ("muted", "#a9adb8"),
                ("dim", "#868b97"),
            ],
        ),
        theme(
            "Forest",
            Theme::Dark,
            "#2fb37a",
            &[
                ("bg", "#151b18"),
                ("surface", "#1b231f"),
                ("elevated", "#222b26"),
                ("border", "#2a342e"),
                ("text", "#e3ebe6"),
                ("muted", "#a8b5ad"),
                ("dim", "#86948b"),
            ],
        ),
        theme(
            "Paper",
            Theme::Light,
            "#b5541c",
            &[
                ("bg", "#f4efe6"),
                ("surface", "#fbf8f2"),
                ("elevated", "#fffdf8"),
                ("border", "#e2dacb"),
                ("text", "#2a2520"),
                ("muted", "#5c544b"),
                ("dim", "#71685d"),
            ],
        ),
    ]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TabUnread {
    /// New messages, mentions stronger.
    #[default]
    All,
    Mentions,
    Off,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_theme(t: CustomTheme) -> Appearance {
        Appearance {
            theme_id: t.id.clone(),
            themes: vec![t],
            ..Appearance::default()
        }
    }

    #[test]
    fn active_theme_sets_the_base() {
        let mut a = with_theme(CustomTheme {
            id: "abc".into(),
            base: Theme::Light,
            accent: "#ff0000".into(),
            ..CustomTheme::default()
        });
        a.sanitize();
        assert_eq!(a.theme, Theme::Light);
        assert_eq!(a.active_accent(), "#ff0000");
        a.theme_id = "gone".into();
        a.sanitize();
        assert!(a.theme_id.is_empty());
        assert_eq!(a.active_accent(), DEFAULT_ACCENT);
    }

    #[test]
    fn themes_are_cleaned() {
        let mut a = with_theme(CustomTheme {
            id: "a-b c".into(),
            name: "   ".into(),
            colors: BTreeMap::from([
                ("bg".into(), "#ABC".into()),
                ("text".into(), "blue".into()),
                ("nope".into(), "#ffffff".into()),
            ]),
            ..CustomTheme::default()
        });
        a.themes.push(CustomTheme {
            id: "abc".into(),
            ..CustomTheme::default()
        });
        a.sanitize();
        assert_eq!(a.themes.len(), 1, "same id twice keeps the first");
        let t = &a.themes[0];
        assert_eq!(t.id, "abc");
        assert_eq!(t.name, "My theme");
        assert_eq!(t.colors, BTreeMap::from([("bg".into(), "#aabbcc".into())]));
    }

    #[test]
    fn wallpaper_sources() {
        let w = |s: &str| Wallpaper {
            source: s.into(),
            ..Wallpaper::default()
        };
        assert!(w("https://example.com/a.png").is_link());
        assert!(!w("https://example.com/a.png\") ; x").is_link());
        assert!(!w("http://example.com/a.png").is_link());
        assert!(w("9f3a2b.webp").is_file());
        assert!(!w("../x.png").is_file());
        assert!(!w("a.exe").is_file());
    }

    #[test]
    fn presets_survive_sanitize() {
        for mut t in preset_themes() {
            t.id = "x".into();
            let before = t.clone();
            t.sanitize();
            assert_eq!(t, before);
        }
    }
}
