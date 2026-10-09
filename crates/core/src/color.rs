//! Accent color (3.3, 3.4). Every accent token is derived from the one color the user
//! picks, then nudged until it meets WCAG contrast, so the accent never hurts readability.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const DEFAULT_ACCENT: &str = "#145aff";

/// Fills (buttons, toggles, active markers) need 3:1 against the page (WCAG 1.4.11).
const FILL_CONTRAST: f64 = 3.0;
/// Accent-colored text needs 4.5:1 against every surface it sits on (WCAG 1.4.3).
const TEXT_CONTRAST: f64 = 4.5;

const WHITE: Rgb = Rgb::new(0xff, 0xff, 0xff);
const BLACK: Rgb = Rgb::new(0, 0, 0);
const DARK_INK: Rgb = Rgb::new(0x0e, 0x0f, 0x12);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    /// Page background and raised surface. Keep in sync with `ui/src/design/tokens.css`.
    fn backgrounds(self) -> [Rgb; 2] {
        match self {
            Theme::Dark => [Rgb::new(0x1e, 0x1f, 0x23), Rgb::new(0x26, 0x27, 0x2c)],
            Theme::Light => [Rgb::new(0xf3, 0xf3, 0xf4), WHITE],
        }
    }

    /// Direction that adds contrast against this theme's backgrounds.
    fn away_from_background(self) -> Rgb {
        match self {
            Theme::Dark => WHITE,
            Theme::Light => BLACK,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#rgb` or `#rrggbb` (the `#` is optional).
    pub fn parse(input: &str) -> Option<Self> {
        let hex = input.trim().trim_start_matches('#');
        let digits: Vec<u8> = hex
            .chars()
            .map(|c| c.to_digit(16).map(|d| d as u8))
            .collect::<Option<_>>()?;
        match digits.as_slice() {
            [r, g, b] => Some(Self::new(r * 17, g * 17, b * 17)),
            [r1, r2, g1, g2, b1, b2] => Some(Self::new(r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2)),
            _ => None,
        }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    fn rgba(self, alpha: f64) -> String {
        format!("rgba({}, {}, {}, {alpha})", self.r, self.g, self.b)
    }

    /// WCAG relative luminance.
    pub fn luminance(self) -> f64 {
        fn channel(c: u8) -> f64 {
            let c = f64::from(c) / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// Linear blend toward `other`; `t` = 0 keeps self, 1 gives other.
    pub fn mix(self, other: Rgb, t: f64) -> Rgb {
        let lerp = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Rgb::new(
            lerp(self.r, other.r),
            lerp(self.g, other.g),
            lerp(self.b, other.b),
        )
    }
}

/// WCAG contrast ratio, 1 to 21.
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// CSS values for every accent use in the UI.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccentTokens {
    /// Fills: primary buttons, toggles, live markers.
    pub accent: String,
    pub accent_hover: String,
    /// Text and icons on top of an accent fill.
    pub accent_ink: String,
    /// Accent-colored text on page backgrounds.
    pub accent_text: String,
    /// Tints for selected rows and active items.
    pub accent_dim: String,
    pub accent_wash: String,
    /// Focus rings and outlined states.
    pub accent_border: String,
}

/// Derives all accent tokens from the user's color. Invalid input falls back to the default.
pub fn accent_tokens(input: &str, theme: Theme) -> AccentTokens {
    let base = Rgb::parse(input).unwrap_or(Rgb::new(0x14, 0x5a, 0xff));
    let [page, surface] = theme.backgrounds();
    let away = theme.away_from_background();

    let fill = push_until(base, away, |c| contrast(c, page) >= FILL_CONTRAST);
    let text = push_until(base, away, |c| {
        contrast(c, page) >= TEXT_CONTRAST && contrast(c, surface) >= TEXT_CONTRAST
    });
    let hover = match theme {
        Theme::Dark => fill.mix(WHITE, 0.15),
        Theme::Light => fill.mix(BLACK, 0.12),
    };
    let ink = if contrast(fill, WHITE) >= contrast(fill, DARK_INK) {
        WHITE
    } else {
        DARK_INK
    };
    let tint = match theme {
        Theme::Dark => fill,
        Theme::Light => text,
    };

    AccentTokens {
        accent: fill.to_hex(),
        accent_hover: hover.to_hex(),
        accent_ink: ink.to_hex(),
        accent_text: text.to_hex(),
        accent_dim: tint.rgba(0.14),
        accent_wash: tint.rgba(0.22),
        accent_border: tint.rgba(0.55),
    }
}

/// Normalizes a user-entered accent to `#rrggbb`, or `None` if it isn't a color.
pub fn normalize_accent(input: &str) -> Option<String> {
    Rgb::parse(input).map(Rgb::to_hex)
}

/// C8 Name colors in chat: nudged toward the theme's text color until they read at 4.5:1
/// on the page and on raised surfaces, so dark names stay visible in dark mode.
pub fn readable_name(color: Rgb, theme: Theme) -> Rgb {
    let [page, surface] = theme.backgrounds();
    push_until(color, theme.away_from_background(), |c| {
        contrast(c, page) >= TEXT_CONTRAST && contrast(c, surface) >= TEXT_CONTRAST
    })
}

/// Blends `color` toward `target` in 5% steps until `ok` holds.
fn push_until(color: Rgb, target: Rgb, ok: impl Fn(Rgb) -> bool) -> Rgb {
    (0..=20)
        .map(|step| color.mix(target, f64::from(step) * 0.05))
        .find(|c| ok(*c))
        .unwrap_or(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(hex: &str) -> Rgb {
        Rgb::parse(hex).unwrap()
    }

    #[test]
    fn parses_short_and_long_hex() {
        assert_eq!(Rgb::parse("#abc"), Some(Rgb::new(0xaa, 0xbb, 0xcc)));
        assert_eq!(Rgb::parse("145AFF"), Some(Rgb::new(0x14, 0x5a, 0xff)));
        assert_eq!(Rgb::parse("#12345"), None);
        assert_eq!(Rgb::parse("#gggggg"), None);
        assert_eq!(normalize_accent(" #ABC "), Some("#aabbcc".into()));
    }

    #[test]
    fn contrast_matches_wcag_extremes() {
        assert!((contrast(WHITE, BLACK) - 21.0).abs() < 0.01);
        assert!((contrast(WHITE, WHITE) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_accent_is_kept_as_fill_in_both_themes() {
        for theme in [Theme::Dark, Theme::Light] {
            let t = accent_tokens(DEFAULT_ACCENT, theme);
            assert_eq!(t.accent, DEFAULT_ACCENT, "{theme:?}");
            assert_eq!(t.accent_ink, "#ffffff");
        }
    }

    #[test]
    fn accent_text_is_readable_on_every_surface() {
        for theme in [Theme::Dark, Theme::Light] {
            for input in [
                DEFAULT_ACCENT,
                "#ffff00",
                "#1e1f23",
                "#000080",
                "#ffffff",
                "#000000",
            ] {
                let t = accent_tokens(input, theme);
                for bg in theme.backgrounds() {
                    let c = contrast(rgb(&t.accent_text), bg);
                    assert!(c >= TEXT_CONTRAST, "{input} {theme:?}: text {c:.2}");
                }
            }
        }
    }

    #[test]
    fn fills_never_disappear_into_the_page() {
        for theme in [Theme::Dark, Theme::Light] {
            for input in ["#1e1f23", "#f3f3f4", "#ffff00", "#000080"] {
                let t = accent_tokens(input, theme);
                let c = contrast(rgb(&t.accent), theme.backgrounds()[0]);
                assert!(c >= FILL_CONTRAST, "{input} {theme:?}: fill {c:.2}");
            }
        }
    }

    #[test]
    fn ink_picks_the_readable_side() {
        assert_eq!(
            accent_tokens("#ffff00", Theme::Dark).accent_ink,
            DARK_INK.to_hex()
        );
        assert_eq!(accent_tokens("#145aff", Theme::Dark).accent_ink, "#ffffff");
    }

    #[test]
    fn name_colors_stay_readable_and_keep_their_hue() {
        for theme in [Theme::Dark, Theme::Light] {
            for input in ["#0000ff", "#000000", "#ffffff", "#ff0000", "#1e1f23", "#9acd32"] {
                let c = readable_name(rgb(input), theme);
                for bg in theme.backgrounds() {
                    assert!(contrast(c, bg) >= TEXT_CONTRAST, "{input} {theme:?}");
                }
            }
        }
        // Already readable: unchanged.
        assert_eq!(readable_name(rgb("#ff69b4"), Theme::Dark), rgb("#ff69b4"));
    }

    #[test]
    fn invalid_input_falls_back_to_default() {
        assert_eq!(
            accent_tokens("nope", Theme::Dark),
            accent_tokens(DEFAULT_ACCENT, Theme::Dark)
        );
    }
}
