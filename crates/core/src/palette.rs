//! U8 Custom theme colors. The user picks a few base colors; hover, raised and line tints
//! are derived from them, and anything that would hurt readability is nudged, like the
//! accent (3.3): backgrounds stay dark (or light), text keeps its contrast.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use crate::color::{AccentTokens, BLACK, Rgb, Theme, WHITE, accent_tokens_on, contrast, push_until};

/// Colors a theme can set, as CSS variable names without the `--`.
pub const KEYS: [&str; 11] = [
    "bg", "surface", "elevated", "border", "text", "muted", "dim", "red", "yellow", "green", "cyan",
];

/// Dark themes keep their backgrounds at or below this luminance, light ones at or above
/// the light value, so name colors and the accent stay readable on them.
const DARK_MAX_LUMINANCE: f64 = 0.08;
const LIGHT_MIN_LUMINANCE: f64 = 0.55;

/// Contrast each color needs against every background, by key: body text, secondary
/// text, small print (timestamps), then the status colors used for icons and marks.
const TEXT_NEEDS: [(&str, f64); 7] = [
    ("text", 7.0),
    ("muted", 4.5),
    ("dim", 4.0),
    ("red", 3.0),
    ("yellow", 3.0),
    ("green", 3.0),
    ("cyan", 3.0),
];

/// The built-in colors. Keep in sync with `ui/src/design/tokens.css`.
pub fn builtin(theme: Theme, key: &str) -> Option<Rgb> {
    let hex = match (theme, key) {
        (Theme::Dark, "bg") => "#1e1f23",
        (Theme::Dark, "surface") => "#26272c",
        (Theme::Dark, "elevated") => "#2f3037",
        (Theme::Dark, "border") => "#34353c",
        (Theme::Dark, "text") => "#e8e8ea",
        (Theme::Dark, "muted") => "#b4b5bb",
        (Theme::Dark, "dim") => "#91939c",
        (Theme::Dark, "red") => "#ff6b6b",
        (Theme::Dark, "yellow") => "#ffb545",
        (Theme::Dark, "green") => "#33d68f",
        (Theme::Dark, "cyan") => "#4fd6ff",
        (Theme::Light, "bg") => "#f3f3f4",
        (Theme::Light, "surface") => "#ffffff",
        (Theme::Light, "elevated") => "#ffffff",
        (Theme::Light, "border") => "#dcdde1",
        (Theme::Light, "text") => "#1b1c20",
        (Theme::Light, "muted") => "#4f515a",
        (Theme::Light, "dim") => "#6b6d76",
        (Theme::Light, "red") => "#c1352f",
        (Theme::Light, "yellow") => "#9a5c05",
        (Theme::Light, "green") => "#0f8a55",
        (Theme::Light, "cyan") => "#0b6d8a",
        _ => return None,
    };
    Rgb::parse(hex)
}

/// Tints the built-in themes set by hand in `tokens.css`.
fn builtin_derived(theme: Theme, key: &str) -> &'static str {
    match (theme, key) {
        (Theme::Dark, "surface-hover") => "#2d2e34",
        (Theme::Dark, "elevated-2") => "#353640",
        (Theme::Dark, "border-strong") => "#43454d",
        (Theme::Dark, "row-hover") => "rgba(255, 255, 255, 0.035)",
        (Theme::Dark, "row-alt") => "rgba(255, 255, 255, 0.015)",
        (Theme::Dark, "overlay") => "rgba(14, 15, 18, 0.6)",
        (Theme::Light, "surface-hover") => "#f6f6f7",
        (Theme::Light, "elevated-2") => "#f4f4f6",
        (Theme::Light, "border-strong") => "#c4c6cc",
        (Theme::Light, "row-hover") => "rgba(20, 22, 30, 0.035)",
        (Theme::Light, "row-alt") => "rgba(20, 22, 30, 0.015)",
        (Theme::Light, "overlay") => "rgba(243, 243, 244, 0.7)",
        _ => "",
    }
}

fn overlay_alpha(theme: Theme) -> f64 {
    match theme {
        Theme::Dark => 0.6,
        Theme::Light => 0.7,
    }
}

/// Every CSS token a custom theme sets, ready for the page.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Palette {
    /// CSS variable name (without `--`) → value.
    pub tokens: BTreeMap<String, String>,
    /// Keys whose picked color was nudged to stay readable.
    pub adjusted: Vec<String>,
    /// The page background, also used to paint windows before the UI loads.
    pub bg: String,
    pub surface: String,
}

impl Palette {
    pub fn backgrounds(&self) -> [Rgb; 2] {
        let parse = |s: &str| Rgb::parse(s).unwrap_or(BLACK);
        [parse(&self.bg), parse(&self.surface)]
    }
}

/// Builds the palette for `base` with the user's `colors` (`key` → `#rrggbb`) on top.
pub fn palette(base: Theme, colors: &BTreeMap<String, String>) -> Palette {
    let mut picked: BTreeMap<&str, Rgb> = BTreeMap::new();
    for key in KEYS {
        let builtin = builtin(base, key).unwrap_or(BLACK);
        let color = colors.get(key).and_then(|c| Rgb::parse(c)).unwrap_or(builtin);
        picked.insert(key, color);
    }
    let mut adjusted = Vec::new();

    // Backgrounds stay on their side of the scale.
    let (limit_dark, toward) = match base {
        Theme::Dark => (true, BLACK),
        Theme::Light => (false, WHITE),
    };
    for key in ["bg", "surface", "elevated"] {
        let c = picked[key];
        let fixed = push_until(c, toward, |c| {
            if limit_dark {
                c.luminance() <= DARK_MAX_LUMINANCE
            } else {
                c.luminance() >= LIGHT_MIN_LUMINANCE
            }
        });
        if fixed != c {
            picked.insert(key, fixed);
            adjusted.push(key.to_owned());
        }
    }

    // Text keeps its contrast on every background.
    let backgrounds = [picked["bg"], picked["surface"], picked["elevated"]];
    let away = base.away_from_background();
    for (key, needs) in TEXT_NEEDS {
        let c = picked[key];
        let fixed = push_until(c, away, |c| backgrounds.iter().all(|b| contrast(c, *b) >= needs));
        if fixed != c {
            picked.insert(key, fixed);
            adjusted.push(key.to_owned());
        }
    }

    let text = picked["text"];
    let mut tokens: BTreeMap<String, String> = picked
        .iter()
        .map(|(k, c)| ((*k).to_owned(), c.to_hex()))
        .collect();
    // Tints follow their colors; untouched ones keep the built-in values exactly.
    let untouched = |keys: &[&str]| keys.iter().all(|k| Some(picked[k]) == builtin(base, k));
    let derived = [
        (
            "surface-hover",
            &["surface", "text"][..],
            picked["surface"].mix(text, 0.04).to_hex(),
        ),
        (
            "elevated-2",
            &["elevated", "text"][..],
            picked["elevated"].mix(text, 0.05).to_hex(),
        ),
        (
            "border-strong",
            &["border", "text"][..],
            picked["border"].mix(text, 0.12).to_hex(),
        ),
        ("row-hover", &["text"][..], text.rgba(0.035)),
        ("row-alt", &["text"][..], text.rgba(0.015)),
        ("overlay", &["bg"][..], picked["bg"].rgba(overlay_alpha(base))),
    ];
    for (key, from, value) in derived {
        let value = if untouched(from) {
            builtin_derived(base, key).to_owned()
        } else {
            value
        };
        tokens.insert(key.to_owned(), value);
    }

    Palette {
        bg: picked["bg"].to_hex(),
        surface: picked["surface"].to_hex(),
        tokens,
        adjusted,
    }
}

/// The accent tokens for a custom palette.
pub fn accent(palette: &Palette, base: Theme, accent: &str) -> AccentTokens {
    accent_tokens_on(accent, base, palette.backgrounds())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn builtin_colors_pass_unchanged() {
        for base in [Theme::Dark, Theme::Light] {
            let p = palette(base, &BTreeMap::new());
            assert!(p.adjusted.is_empty(), "{base:?}: {:?}", p.adjusted);
            for key in KEYS {
                assert_eq!(p.tokens[key], builtin(base, key).unwrap().to_hex());
            }
            assert_eq!(p.tokens["surface-hover"], builtin_derived(base, "surface-hover"));
            assert_eq!(p.tokens["overlay"], builtin_derived(base, "overlay"));
        }
    }

    #[test]
    fn picked_colors_are_used() {
        let p = palette(Theme::Dark, &colors(&[("bg", "#0e0f13"), ("text", "#ffffff")]));
        assert_eq!(p.bg, "#0e0f13");
        assert_eq!(p.tokens["text"], "#ffffff");
        assert!(p.adjusted.is_empty());
        assert_ne!(p.tokens["surface-hover"], builtin_derived(Theme::Dark, "surface-hover"));
        assert_ne!(
            p.tokens["border-strong"],
            builtin_derived(Theme::Dark, "border-strong")
        );
    }

    #[test]
    fn dark_backgrounds_stay_dark() {
        let p = palette(Theme::Dark, &colors(&[("surface", "#d0d0d0")]));
        assert!(p.adjusted.contains(&"surface".to_owned()));
        assert!(Rgb::parse(&p.surface).unwrap().luminance() <= DARK_MAX_LUMINANCE);
    }

    #[test]
    fn light_backgrounds_stay_light() {
        let p = palette(Theme::Light, &colors(&[("bg", "#202020")]));
        assert!(Rgb::parse(&p.bg).unwrap().luminance() >= LIGHT_MIN_LUMINANCE);
    }

    #[test]
    fn unreadable_text_is_nudged() {
        let p = palette(Theme::Dark, &colors(&[("text", "#333333"), ("dim", "#26272c")]));
        assert!(p.adjusted.contains(&"text".to_owned()));
        let text = Rgb::parse(&p.tokens["text"]).unwrap();
        assert!(contrast(text, Rgb::parse(&p.bg).unwrap()) >= 7.0);
        let dim = Rgb::parse(&p.tokens["dim"]).unwrap();
        assert!(contrast(dim, Rgb::parse(&p.surface).unwrap()) >= 4.0);
    }

    #[test]
    fn junk_falls_back_to_builtin() {
        let p = palette(Theme::Light, &colors(&[("bg", "nope"), ("unknown", "#ff0000")]));
        assert_eq!(p.bg, "#f3f3f4");
        assert!(!p.tokens.contains_key("unknown"));
    }

    #[test]
    fn accent_reads_on_custom_backgrounds() {
        let p = palette(
            Theme::Light,
            &colors(&[("bg", "#f4ecd8"), ("surface", "#fbf6ea")]),
        );
        let a = accent(&p, Theme::Light, "#ffd400");
        let text = Rgb::parse(&a.accent_text).unwrap();
        assert!(contrast(text, Rgb::parse(&p.bg).unwrap()) >= 4.5);
    }
}
