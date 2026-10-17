use std::collections::BTreeMap;

use cyex_core::color::{DEFAULT_ACCENT, Theme, normalize_accent};
use cyex_core::palette::KEYS;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Schema;
use super::builtin_themes::{self, builtin_theme, is_builtin_id};

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
    /// F16 (v1.04 batch 2b) A bar across the top or bottom: up to `tab_max_rows` rows and
    /// then a scrollbar, or every row.
    pub tab_rows: TabRows,
    /// F16 1 to 10.
    pub tab_max_rows: u8,
    /// F17 Hide tabs whose channels are all offline (the one on screen stays).
    pub tab_only_live: bool,
    /// U9 What lights up a tab in the background.
    pub tab_unread: TabUnread,
    /// U8 Themes made in the theme editor.
    pub themes: Vec<CustomTheme>,
    /// U8 The custom theme in use; empty for plain dark or light. `theme` follows its base.
    pub theme_id: String,
    /// App zoom in percent, one of `ZOOMS`. Overlays keep their own size.
    pub zoom: u16,
    /// V106-02 (Q1) Accent picked for a built-in theme, by its id; missing = the theme's own.
    pub accents: BTreeMap<String, String>,
    /// V106-02 Old "New from a preset" copies were moved to the built-ins (once).
    #[serde(default)]
    pub presets_moved: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: DEFAULT_ACCENT.into(),
            tab_density: TabDensity::Normal,
            tab_close_button: true,
            tab_bar_side: TabBarSide::Top,
            tab_rows: TabRows::Limit,
            tab_max_rows: DEFAULT_TAB_ROWS,
            tab_only_live: false,
            tab_unread: TabUnread::All,
            themes: Vec::new(),
            theme_id: String::new(),
            zoom: DEFAULT_ZOOM,
            accents: BTreeMap::new(),
            presets_moved: true,
        }
    }
}

/// F16 How many rows of tabs a bar shows before it scrolls.
pub const TAB_ROWS: std::ops::RangeInclusive<u8> = 1..=10;
pub const DEFAULT_TAB_ROWS: u8 = 3;

/// App zoom steps, in percent.
pub const ZOOMS: [u16; 5] = [100, 125, 150, 175, 200];
pub const DEFAULT_ZOOM: u16 = 100;

impl Schema for Appearance {
    fn sanitize(&mut self) {
        if !self.presets_moved {
            builtin_themes::move_old_presets(self);
            self.presets_moved = true;
        }
        // The nearest step, so a hand-edited 130 lands on 125 and 999 on 200.
        self.zoom = ZOOMS
            .into_iter()
            .min_by_key(|z| z.abs_diff(self.zoom))
            .unwrap_or(DEFAULT_ZOOM);
        self.tab_max_rows = self.tab_max_rows.clamp(*TAB_ROWS.start(), *TAB_ROWS.end());
        self.accent = normalize_accent(&self.accent).unwrap_or_else(|| DEFAULT_ACCENT.into());
        let mut seen = Vec::new();
        self.themes.retain_mut(|t| {
            t.sanitize();
            if is_builtin_id(&t.id) {
                let new = super::random_id();
                if self.theme_id == t.id {
                    self.theme_id = new.clone();
                }
                t.id = new;
            }
            let fresh = !t.id.is_empty() && !seen.contains(&t.id);
            seen.push(t.id.clone());
            fresh
        });
        self.themes.truncate(MAX_THEMES);
        self.accents
            .retain(|id, c| is_builtin_id(id) && normalize_accent(c).map(|n| *c = n).is_some());
        match self.active_theme().map(|t| t.base) {
            Some(base) => self.theme = base,
            None => self.theme_id.clear(),
        }
    }
}

impl Appearance {
    /// U8 The custom or built-in theme in use, if any (custom first).
    pub fn active_theme(&self) -> Option<&CustomTheme> {
        if self.theme_id.is_empty() {
            return None;
        }
        self.themes
            .iter()
            .find(|t| t.id == self.theme_id)
            .or_else(|| builtin_theme(&self.theme_id))
    }

    /// Adds a custom theme and switches to it; false (nothing changes) when the list is full.
    pub fn add_theme(&mut self, theme: CustomTheme) -> bool {
        if self.themes.len() >= MAX_THEMES {
            return false;
        }
        self.theme_id = theme.id.clone();
        self.themes.push(theme);
        true
    }

    /// The accent in use: a built-in's picked one or its own, a custom theme's, else the plain one.
    pub fn active_accent(&self) -> &str {
        match self.active_theme() {
            Some(t) if is_builtin_id(&t.id) => self.accents.get(&t.id).map_or(&t.accent, |a| a),
            Some(t) => &t.accent,
            None => &self.accent,
        }
    }
}

/// Most custom themes one can keep.
pub const MAX_THEMES: usize = 30;
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

/// F16
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TabRows {
    /// Up to `tab_max_rows`, then a scrollbar.
    #[default]
    Limit,
    /// Every row, however many.
    All,
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
    fn zoom_defaults_and_snaps_to_a_step() {
        assert_eq!(Appearance::default().zoom, 100);
        let old: Appearance = serde_json::from_str(r#"{"tabDensity":"compact"}"#).unwrap();
        assert_eq!(old.zoom, 100);
        for (given, want) in [(0, 100), (99, 100), (130, 125), (162, 150), (175, 175), (999, 200)] {
            let mut a = Appearance { zoom: given, ..Appearance::default() };
            a.sanitize();
            assert_eq!(a.zoom, want, "{given}");
        }
    }

    #[test]
    fn tab_rows_are_clamped() {
        let mut a = Appearance {
            tab_max_rows: 0,
            ..Appearance::default()
        };
        a.sanitize();
        assert_eq!(a.tab_max_rows, 1);
        a.tab_max_rows = 40;
        a.sanitize();
        assert_eq!(a.tab_max_rows, 10);
        let old: Appearance = serde_json::from_str(r#"{"tabDensity":"compact"}"#).unwrap();
        assert_eq!(
            (old.tab_rows, old.tab_max_rows, old.tab_only_live),
            (TabRows::Limit, 3, false)
        );
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

    /// Untouched Midnight (a1), untouched Paper (b2), Forest with a changed bg (c3).
    fn old_copies(theme_id: &str, moved: bool) -> Appearance {
        let [(mut mid, _), (mut forest, _), (mut paper, _)] = builtin_themes::old_presets();
        mid.id = "a1".into();
        paper.id = "b2".into();
        forest.id = "c3".into();
        forest.colors.insert("bg".into(), "#000000".into());
        Appearance {
            themes: vec![mid, paper, forest],
            theme_id: theme_id.into(),
            theme: if theme_id == "b2" { Theme::Light } else { Theme::Dark },
            presets_moved: moved,
            ..Appearance::default()
        }
    }

    #[test]
    fn builtins_pick_by_id() {
        let mut a: Appearance = serde_json::from_str(r#"{"themeId":"dusk"}"#).unwrap();
        a.sanitize();
        assert_eq!(a.theme_id, "dusk");
        assert_eq!(a.theme, Theme::Dark);
        assert_eq!(a.active_accent(), "#d4679f");
    }

    #[test]
    fn old_preset_copies_move_once() {
        let old: Appearance = serde_json::from_str(r#"{"themeId":"x"}"#).unwrap();
        assert!(!old.presets_moved, "a file without the key still gets the move");
        let load = |theme_id: &str| {
            let mut a = old_copies(theme_id, false);
            a.sanitize();
            a
        };
        let a = load("b2");
        let ids: Vec<&str> = a.themes.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["c3"]);
        assert_eq!(a.theme_id, "");
        assert_eq!(a.theme, Theme::Dark);
        assert!(a.presets_moved);
        assert_eq!(load("a1").theme_id, "midnight");
    }

    #[test]
    fn moved_flag_stops_the_move() {
        let mut a = old_copies("b2", true);
        a.sanitize();
        assert_eq!(a.themes.len(), 3);
        assert_eq!(a.theme_id, "b2");
    }

    #[test]
    fn a_custom_theme_cannot_take_a_builtin_id() {
        let mut a = with_theme(CustomTheme {
            id: "ocean".into(),
            name: "Mine".into(),
            ..CustomTheme::default()
        });
        a.sanitize();
        assert_eq!(a.themes.len(), 1);
        assert_eq!(a.themes[0].id.len(), 10);
        assert_eq!(a.theme_id, a.themes[0].id, "the user's theme stays selected");
        assert_ne!(a.themes[0].id, "ocean");
        assert_eq!(builtin_theme("ocean").unwrap().name, "Ocean");
    }

    #[test]
    fn builtin_accents() {
        let mut a: Appearance =
            serde_json::from_str(r##"{"themeId":"dusk","accents":{"dusk":"#ABC","nope":"#ffffff","ember":"blue"}}"##)
                .unwrap();
        a.sanitize();
        assert_eq!(a.accents, BTreeMap::from([("dusk".into(), "#aabbcc".into())]));
        assert_eq!(a.active_accent(), "#aabbcc");
    }

    #[test]
    fn the_theme_list_is_capped() {
        let mut a = Appearance::default();
        for i in 0..MAX_THEMES {
            assert!(a.add_theme(CustomTheme { id: format!("t{i}"), ..CustomTheme::default() }));
        }
        let before = a.clone();
        assert!(!a.add_theme(CustomTheme { id: "extra".into(), ..CustomTheme::default() }));
        assert_eq!(a, before);
        assert_eq!(a.theme_id, format!("t{}", MAX_THEMES - 1));
    }

    #[test]
    fn new_install_has_nothing_to_move() {
        assert!(Appearance::default().presets_moved);
    }
}
