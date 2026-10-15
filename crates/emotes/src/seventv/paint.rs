//! 1.37 7TV name paints to CSS, built once per paint like 7tv-ext does. The UI puts the
//! declarations in one class per paint, on top of `background-clip: text` with the name's
//! own color underneath.

use std::fmt::Write;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A paint in the shape the EventAPI sends (v3). Today's paints are one flat layer;
/// `gradients` is read too in case several come back.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Paint {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    layer: Layer,
    #[serde(default)]
    gradients: Vec<Layer>,
    #[serde(default)]
    shadows: Vec<Shadow>,
    color: Option<i64>,
}

mod v4;
pub(crate) use v4::from_v4;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
struct Layer {
    function: Option<String>,
    #[serde(default)]
    repeat: bool,
    #[serde(default)]
    angle: f64,
    shape: Option<String>,
    image_url: Option<String>,
    #[serde(default)]
    stops: Vec<Stop>,
    /// `[x, y]` as 0-1.
    at: Option<Vec<f64>>,
    /// `[w, h]` as 0-1.
    size: Option<Vec<f64>>,
    canvas_repeat: Option<String>,
    /// V1051-03 v4 lists the still frame's URL; v3 never sets it.
    #[serde(default)]
    still_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Stop {
    at: f64,
    color: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct Shadow {
    #[serde(default)]
    x_offset: f64,
    #[serde(default)]
    y_offset: f64,
    #[serde(default)]
    radius: f64,
    color: i64,
}

/// What the UI gets per paint.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaintStyle {
    pub id: String,
    pub name: String,
    /// CSS declarations.
    pub css: String,
    /// E2 Declarations while the window isn't focused, for paints made of moving pictures.
    pub still: Option<String>,
}

impl Paint {
    pub fn style(&self) -> PaintStyle {
        PaintStyle {
            id: self.id.clone(),
            name: self.name.clone(),
            css: self.css(false),
            still: self.moves().then(|| self.css(true)),
        }
    }

    fn layers(&self) -> Vec<&Layer> {
        if self.gradients.is_empty() {
            self.layer
                .function
                .is_some()
                .then_some(&self.layer)
                .into_iter()
                .collect()
        } else {
            self.gradients.iter().collect()
        }
    }

    fn moves(&self) -> bool {
        self.layers().iter().any(|l| l.image().is_some())
    }

    fn css(&self, still: bool) -> String {
        let mut images = Vec::new();
        let mut positions = Vec::new();
        let mut sizes = Vec::new();
        let mut repeats = Vec::new();
        for l in self.layers() {
            let Some(image) = l.css_image(still) else { continue };
            images.push(image);
            positions.push(match l.at.as_deref() {
                Some([x, y]) => format!("{}% {}%", pct(*x), pct(*y)),
                _ => "0% 0%".into(),
            });
            sizes.push(match l.size.as_deref() {
                Some([w, h]) => format!("{}% {}%", pct(*w), pct(*h)),
                _ => "100% 100%".into(),
            });
            repeats.push(
                l.canvas_repeat
                    .clone()
                    .filter(|r| is_word(r))
                    .unwrap_or_else(|| "no-repeat".into()),
            );
        }
        let mut css = String::new();
        if let Some(c) = self.color {
            let _ = write!(css, "color:{};", rgba(c));
        }
        if !images.is_empty() {
            let _ = write!(
                css,
                "background-image:{};background-position:{};background-size:{};background-repeat:{};",
                images.join(","),
                positions.join(","),
                sizes.join(","),
                repeats.join(",")
            );
        }
        if !self.shadows.is_empty() {
            let shadows: Vec<String> = self
                .shadows
                .iter()
                .map(|s| {
                    format!(
                        "drop-shadow({}px {}px {}px {})",
                        num(s.x_offset),
                        num(s.y_offset),
                        num(s.radius),
                        rgba(s.color)
                    )
                })
                .collect();
            // V105-01 Shadows go in a variable, so the overlay's outline can join them (emotes.css).
            let _ = write!(css, "--paint-shadow:{};", shadows.join(" "));
        }
        css
    }
}

impl Layer {
    fn image(&self) -> Option<&str> {
        (self.function.as_deref() == Some("URL"))
            .then_some(self.image_url.as_deref())
            .flatten()
            .filter(|u| safe_url(u))
    }

    fn css_image(&self, still: bool) -> Option<String> {
        let f = self.function.as_deref()?;
        if f == "URL" {
            let url = self.image()?;
            let url = if still {
                match self.still_url.as_deref().filter(|u| safe_url(u)) {
                    Some(s) => s.to_owned(),
                    None => url.replacen("x.webp", "x_static.webp", 1),
                }
            } else {
                url.to_owned()
            };
            return Some(format!("url(\"{url}\")"));
        }
        let mut args = Vec::new();
        let name = match f {
            "LINEAR_GRADIENT" => {
                args.push(format!("{}deg", num(self.angle)));
                "linear-gradient"
            }
            "RADIAL_GRADIENT" => {
                args.push(match self.shape.as_deref() {
                    Some(s) if is_word(s) => s.to_owned(),
                    _ => "circle".into(),
                });
                "radial-gradient"
            }
            "CONIC_GRADIENT" => {
                args.push(format!("from {}deg", num(self.angle)));
                "conic-gradient"
            }
            _ => return None,
        };
        if self.stops.is_empty() {
            return None;
        }
        for s in &self.stops {
            args.push(format!("{} {}%", rgba(s.color), pct(s.at)));
        }
        let prefix = if self.repeat { "repeating-" } else { "" };
        Some(format!("{prefix}{name}({})", args.join(",")))
    }
}

/// 7TV's packed RGBA.
fn rgba(n: i64) -> String {
    let n = n as u32;
    let a = f64::from(n & 255) / 255.0;
    format!(
        "rgba({},{},{},{})",
        n >> 24 & 255,
        n >> 16 & 255,
        n >> 8 & 255,
        num(a)
    )
}

fn pct(x: f64) -> String {
    num(x * 100.0)
}

/// Up to 3 decimals, no trailing zeros.
fn num(x: f64) -> String {
    if !x.is_finite() {
        return "0".into();
    }
    let s = format!("{:.3}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_owned() }
}

fn is_word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
}

/// Only 7TV's CDN, and nothing that could close the CSS string.
pub(crate) fn safe_url(u: &str) -> bool {
    u.starts_with("https://cdn.7tv.app/") && !u.contains(['"', '\\', '(', ')', ';', '<', '>', '\n'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seventv::events::{Event, Frame, parse};

    fn paint(name: &str) -> Paint {
        let path = format!("{}/tests/fixtures/7tv/{name}", env!("CARGO_MANIFEST_DIR"));
        match parse(&std::fs::read_to_string(path).unwrap()) {
            Frame::Event(e) => match *e {
                Event::Paint(p) => p,
                e => panic!("{e:?}"),
            },
            f => panic!("{f:?}"),
        }
    }

    #[test]
    fn colors_unpack() {
        assert_eq!(rgba(-525135617), "rgba(224,179,16,1)");
        assert_eq!(rgba(255), "rgba(0,0,0,1)");
        assert_eq!(num(0.15 * 100.0), "15");
    }

    #[test]
    fn linear_paint() {
        let s = paint("eventapi_cosmetic_paint.json").style();
        assert!(
            s.css.starts_with("background-image:linear-gradient(90deg,"),
            "{}",
            s.css
        );
        assert!(s.css.contains("background-size:100% 100%"));
        assert!(s.css.contains("--paint-shadow:drop-shadow(0px 0px 0.1px "));
        assert!(!s.css.contains("filter:"));
        assert!(s.still.is_none());
    }

    #[test]
    fn radial_paint() {
        let s = paint("eventapi_cosmetic_paint_radial.json").style();
        assert!(s.css.contains("radial-gradient("), "{}", s.css);
    }

    #[test]
    fn picture_paint_has_a_still_version() {
        let s = paint("eventapi_cosmetic_paint_url.json").style();
        assert!(s.css.contains("url(\"https://cdn.7tv.app/paint/"), "{}", s.css);
        assert!(s.still.unwrap().contains("1x_static.webp"));
    }

    #[test]
    fn foreign_urls_are_dropped() {
        let p: Paint = serde_json::from_str(
            r#"{"id":"x","name":"x","function":"URL","image_url":"https://evil.example/a.webp","stops":[],"shadows":[]}"#,
        )
        .unwrap();
        assert_eq!(p.style().css, "");
    }
}
