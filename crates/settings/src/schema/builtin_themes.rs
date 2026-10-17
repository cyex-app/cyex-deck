//! V106-02 The built-in themes beside plain dark and light, picked in the top theme
//! selector by id (`appearance.themeId`). Fixed: never edited, never in `themes`.
use std::sync::LazyLock;

use cyex_core::color::{DEFAULT_ACCENT, Theme};

use super::appearance::{Appearance, CustomTheme, Wallpaper};

pub const BUILTIN_IDS: [&str; 5] = ["midnight", "forest", "dusk", "ocean", "ember"];

fn theme(id: &str, name: &str, base: Theme, accent: &str, colors: &[(&str, &str)]) -> CustomTheme {
    CustomTheme {
        id: id.into(),
        name: name.into(),
        base,
        colors: colors
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        accent: accent.into(),
        wallpaper: Wallpaper::default(),
    }
}

static BUILTINS: LazyLock<Vec<CustomTheme>> = LazyLock::new(|| {
    vec![
        theme(
            "midnight",
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
            "forest",
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
            "dusk",
            "Dusk",
            Theme::Dark,
            "#d4679f",
            &[
                ("bg", "#18141c"),
                ("surface", "#1f1a25"),
                ("elevated", "#27212e"),
                ("border", "#2f2838"),
                ("text", "#ece6f1"),
                ("muted", "#b6acbf"),
                ("dim", "#958b9f"),
            ],
        ),
        theme(
            "ocean",
            "Ocean",
            Theme::Dark,
            "#1fb5cc",
            &[
                ("bg", "#0b1620"),
                ("surface", "#10202a"),
                ("elevated", "#162a34"),
                ("border", "#1d343f"),
                ("text", "#e1eef3"),
                ("muted", "#a1b8c0"),
                ("dim", "#7f98a1"),
            ],
        ),
        theme(
            "ember",
            "Ember",
            Theme::Dark,
            "#ec8a2c",
            &[
                ("bg", "#1a1613"),
                ("surface", "#221d19"),
                ("elevated", "#2a2420"),
                ("border", "#352d27"),
                ("text", "#f1e9e2"),
                ("muted", "#beb1a6"),
                ("dim", "#9b8e83"),
            ],
        ),
    ]
});

pub fn builtin_themes() -> &'static [CustomTheme] {
    &BUILTINS
}

pub fn builtin_theme(id: &str) -> Option<&'static CustomTheme> {
    BUILTINS.iter().find(|t| t.id == id)
}

pub fn is_builtin_id(id: &str) -> bool {
    BUILTIN_IDS.contains(&id)
}

/// Q2 The old "New from a preset" copies, each with the built-in id it moves to (`""` = dark).
pub(super) fn old_presets() -> [(CustomTheme, &'static str); 3] {
    [
        (
            theme(
                "",
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
            "midnight",
        ),
        (
            theme(
                "",
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
            "forest",
        ),
        (
            theme(
                "",
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
            "",
        ),
    ]
}

fn is_untouched_copy(t: &CustomTheme, p: &CustomTheme) -> bool {
    t.name == p.name
        && t.base == p.base
        && t.colors == p.colors
        && t.accent == p.accent
        && t.wallpaper.source.is_empty()
}

/// Q2 Removes copies nobody edited; the one in use moves to its built-in (Paper to dark).
pub(super) fn move_old_presets(a: &mut Appearance) {
    let olds = old_presets();
    let mut theme_id = std::mem::take(&mut a.theme_id);
    a.themes.retain(|t| {
        let Some((_, to)) = olds.iter().find(|(p, _)| is_untouched_copy(t, p)) else {
            return true;
        };
        if theme_id == t.id {
            theme_id = (*to).to_owned();
            if to.is_empty() {
                a.theme = Theme::Dark;
            }
        }
        false
    });
    a.theme_id = theme_id;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_core::palette;

    #[test]
    fn ids_match_the_list() {
        let ids: Vec<&str> = builtin_themes().iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, BUILTIN_IDS);
    }

    #[test]
    fn builtins_pass_unadjusted() {
        for t in builtin_themes() {
            let p = palette::palette(t.base, &t.colors);
            assert!(p.adjusted.is_empty(), "{}", t.id);
            assert_eq!(palette::accent(&p, t.base, &t.accent).accent, t.accent, "{}", t.id);
        }
    }

    #[test]
    fn builtins_survive_sanitize() {
        for t in builtin_themes() {
            let mut c = t.clone();
            c.sanitize();
            assert_eq!(&c, t);
        }
    }
}
