//! cyex.tv cosmetics (V112-10..11): the catalog of paints and badges (`GET /v1/cosmetics`) and
//! which of them chatters wear (`GET /v1/cosmetics/users`, at most 100 at a time). Only the fields
//! the deck draws are read. Everything that reaches CSS or a picture address is checked first:
//! colours are `#RRGGBB` or `#RRGGBBAA`, pictures come over https from cyex.tv's own hosts,
//! stops are 2 to 10 at 0 to 1, and shadows are capped like the browser extension's.

use cyex_chat::model::Provider;
use cyex_twitch::pace::{Lane, Opts, Service};
use cyex_twitch::{Http, Result};
use serde::Deserialize;
use serde_json::Value;

use super::api::{MAX_LOOKUP, base};
use crate::cosmetics::BadgeDef;
use crate::seventv::paint::PaintStyle;

/// A cyex.tv paint's id in the deck: the catalog id behind this, so it can't meet a 7TV id and
/// stays a valid CSS class name.
pub const PAINT_PREFIX: &str = "ctv";
/// How often the catalog is read again.
pub const REFRESH_SECS: u64 = 300;

const MAX_STOPS: usize = 10;
/// The extension's caps (a cyex.tv paint may carry up to 4 shadows of up to 16 px).
const MAX_SHADOWS: usize = 3;
const MAX_OFFSET: f64 = 8.0;
const MAX_BLUR: f64 = 8.0;
const MAX_ID: usize = 36;

/// V112-10 The catalog as cyex.tv sent it.
pub async fn catalog_raw(http: &Http, lane: Lane) -> Result<Value> {
    let opts = Opts { lane, quick: true, ..Opts::default() };
    http.get_json_paced_with(Service::Cyextv, opts, &format!("{}/cosmetics", base())).await
}

/// V112-10 What up to [`MAX_LOOKUP`] Twitch users wear, as cyex.tv sent it. Users with nothing
/// are not listed.
pub async fn users_raw(http: &Http, user_ids: &[String], lane: Lane) -> Result<Value> {
    let ids = user_ids[..user_ids.len().min(MAX_LOOKUP)].join(",");
    let opts = Opts { lane, quick: true, ..Opts::default() };
    http.get_json_paced_with(
        Service::Cyextv,
        opts,
        &format!("{}/cosmetics/users?platform=twitch&ids={ids}", base()),
    )
    .await
}

/// Every drawable paint and badge.
#[derive(Debug, Default, PartialEq)]
pub struct Catalog {
    pub paints: Vec<PaintStyle>,
    pub badges: Vec<BadgeDef>,
}

/// One chatter's pick, as catalog ids. `None`: nothing of that kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wear {
    pub user: String,
    pub badge: Option<String>,
    pub paint: Option<String>,
}

/// The deck's paint id for a cyex.tv paint id.
pub fn deck_paint_id(id: &str) -> String {
    format!("{PAINT_PREFIX}{id}")
}

#[derive(Deserialize)]
struct RawCatalog {
    #[serde(default)]
    badges: Vec<Value>,
    #[serde(default)]
    paints: Vec<Value>,
}

#[derive(Deserialize)]
struct RawBadge {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    tooltip: String,
    image: Option<RawImage>,
}

#[derive(Deserialize)]
struct RawImage {
    #[serde(default)]
    host: String,
    #[serde(default)]
    files: Vec<RawFile>,
    #[serde(default)]
    animated: bool,
    #[serde(default = "yes")]
    ready: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct RawFile {
    name: String,
}

#[derive(Deserialize)]
struct RawPaint {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    angle: f64,
    #[serde(default)]
    repeat: bool,
    #[serde(default)]
    shape: String,
    color: Option<String>,
    #[serde(default)]
    stops: Vec<RawStop>,
    #[serde(default)]
    shadows: Vec<RawShadow>,
    image: Option<RawImage>,
}

#[derive(Deserialize)]
struct RawStop {
    #[serde(default)]
    at: f64,
    #[serde(default)]
    color: String,
}

#[derive(Deserialize)]
struct RawShadow {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    #[serde(default)]
    radius: f64,
    #[serde(default)]
    color: String,
}

#[derive(Deserialize)]
struct RawUsers {
    #[serde(default)]
    users: Vec<RawWear>,
}

#[derive(Deserialize)]
struct RawWear {
    platform_user_id: String,
    badge_id: Option<String>,
    paint_id: Option<String>,
}

/// V112-10 The catalog. One entry that can't be read or drawn is left out; a broken answer is an
/// error.
pub fn parse_catalog(v: &Value) -> serde_json::Result<Catalog> {
    let raw = RawCatalog::deserialize(v)?;
    let paints = raw
        .paints
        .into_iter()
        .filter_map(|p| RawPaint::deserialize(p).ok())
        .filter_map(|p| p.style())
        .collect();
    let badges = raw
        .badges
        .into_iter()
        .filter_map(|b| RawBadge::deserialize(b).ok())
        .filter_map(|b| b.def())
        .collect();
    Ok(Catalog { paints, badges })
}

/// V112-10 Who wears what. A user without an id or with an odd one is dropped.
pub fn parse_users(v: &Value) -> serde_json::Result<Vec<Wear>> {
    let raw = RawUsers::deserialize(v)?;
    Ok(raw
        .users
        .into_iter()
        .filter(|u| !u.platform_user_id.is_empty() && u.platform_user_id.bytes().all(|b| b.is_ascii_digit()))
        .map(|u| Wear {
            user: u.platform_user_id,
            badge: u.badge_id.filter(|i| safe_id(i)),
            paint: u.paint_id.filter(|i| safe_id(i)),
        })
        .collect())
}

/// A catalog id that is also a safe CSS class part.
fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_ID && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `#RRGGBB` or `#RRGGBBAA`, lower-cased.
fn color(c: &str) -> Option<String> {
    let h = c.strip_prefix('#')?;
    (matches!(h.len(), 6 | 8) && h.bytes().all(|b| b.is_ascii_hexdigit())).then(|| c.to_ascii_lowercase())
}

/// Only cyex.tv's own hosts over https, with nothing that could close a CSS string or url().
fn safe_url(u: &str) -> bool {
    (u.starts_with("https://cdn.cyex.tv/") || u.starts_with("https://api.cyex.tv/"))
        && u.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~/%:".contains(&b))
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { lo }
}

/// Up to 2 decimals, no trailing zeros.
fn num(x: f64) -> String {
    let s = format!("{x:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".into() } else { s.to_owned() }
}

impl RawImage {
    /// `{host}/{file}` if the picture is made and has that file.
    fn url(&self, file: &str) -> Option<String> {
        if !self.ready || !self.files.iter().any(|f| f.name == file) {
            return None;
        }
        let url = format!("{}/{file}", self.host.trim_end_matches('/'));
        safe_url(&url).then_some(url)
    }
}

impl RawBadge {
    fn def(self) -> Option<BadgeDef> {
        if !safe_id(&self.id) {
            return None;
        }
        let img = self.image?;
        let at = |scale: &str| img.url(&format!("{scale}.webp"));
        // A missing size borrows its nearest neighbour; no 1x at all, no badge.
        let one = at("1x").or_else(|| at("2x")).or_else(|| at("4x"))?;
        let two = at("2x").unwrap_or_else(|| one.clone());
        let four = at("4x").unwrap_or_else(|| two.clone());
        let name = [self.tooltip.trim(), self.name.trim()]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or("Badge");
        let still = |scale: &str| img.animated.then(|| img.url(&format!("{scale}_static.webp"))).flatten();
        Some(BadgeDef {
            provider: Provider::Cyextv,
            id: self.id,
            title: format!("{name} · cyex.tv"),
            image1x: one,
            image2x: two,
            image_large: Some(four.clone()),
            image4x: four,
            still_large: still("4x"),
            still: still("1x"),
            color: None,
            replaces: None,
        })
    }
}

impl RawPaint {
    fn style(self) -> Option<PaintStyle> {
        if !safe_id(&self.id) {
            return None;
        }
        let fallback = self.color.as_deref().and_then(color);
        let shadows = self.shadows();
        let (image, still_image) = match self.kind.as_str() {
            "image" => {
                let img = self.image.as_ref()?;
                let pic = img.url("1x.webp");
                let still = img.animated.then(|| img.url("1x_static.webp")).flatten();
                (pic.map(|u| (u, img.animated)), still)
            }
            _ => (None, None),
        };
        let gradient = self.gradient();
        let tile = self.repeat;
        // What goes in a rule: the plain colour, then the picture or gradient, then the shadows.
        let rule = |bg: Option<String>| {
            let mut css = String::new();
            if let Some(c) = &fallback {
                css.push_str(&format!("color:{c};"));
            }
            if let Some(bg) = bg {
                css.push_str(&format!("background-image:{bg};background-position:center;"));
                css.push_str(if bg.starts_with("url(") {
                    if tile {
                        "background-size:auto 100%;background-repeat:repeat;"
                    } else {
                        "background-size:cover;background-repeat:no-repeat;"
                    }
                } else {
                    "background-size:100% 100%;background-repeat:no-repeat;"
                });
            }
            if !shadows.is_empty() {
                css.push_str(&format!("--paint-shadow:{};", shadows.join(" ")));
            }
            css
        };
        let url = |u: &str| format!("url(\"{u}\")");
        let (css, still) = match &image {
            Some((pic, animated)) => (
                rule(Some(url(pic))),
                animated.then(|| rule(still_image.as_deref().map(url))),
            ),
            None => (rule(gradient), None),
        };
        // Nothing to draw and no colour: the name stays as it is.
        if css.is_empty() {
            return None;
        }
        Some(PaintStyle { id: deck_paint_id(&self.id), name: self.name, css, still })
    }

    /// The gradient as a CSS image, or `None` (fewer than 2 valid stops, an image paint, or an
    /// unknown kind: the plain colour then).
    fn gradient(&self) -> Option<String> {
        let rep = if self.repeat { "repeating-" } else { "" };
        let head = match self.kind.as_str() {
            "linear_gradient" => format!("linear-gradient({}deg", num(clamp(self.angle, 0.0, 360.0))),
            "radial_gradient" => {
                let shape = if self.shape == "ellipse" { "ellipse" } else { "circle" };
                format!("radial-gradient({shape}")
            }
            _ => return None,
        };
        let mut last = 0.0_f64;
        let stops: Vec<String> = self
            .stops
            .iter()
            .take(MAX_STOPS)
            .filter_map(|s| {
                let c = color(&s.color)?;
                last = last.max(clamp(s.at, 0.0, 1.0));
                Some(format!("{c} {}%", num(last * 100.0)))
            })
            .collect();
        (stops.len() >= 2).then(|| format!("{rep}{head},{})", stops.join(",")))
    }

    fn shadows(&self) -> Vec<String> {
        self.shadows
            .iter()
            .take(MAX_SHADOWS)
            .filter_map(|s| {
                let c = color(&s.color)?;
                Some(format!(
                    "drop-shadow({}px {}px {}px {c})",
                    num(clamp(s.x, -MAX_OFFSET, MAX_OFFSET)),
                    num(clamp(s.y, -MAX_OFFSET, MAX_OFFSET)),
                    num(clamp(s.radius, 0.0, MAX_BLUR)),
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/cyextv/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn paint<'a>(c: &'a Catalog, id: &str) -> &'a PaintStyle {
        c.paints.iter().find(|p| p.id == deck_paint_id(id)).unwrap_or_else(|| panic!("{id}"))
    }

    #[test]
    fn catalog_reads_paints_and_badges() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        let ids: Vec<_> = c.paints.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["ctv01LINEAR", "ctv01RADIAL", "ctv01REPEAT", "ctv01IMAGE", "ctv01ANIMATED", "ctv01FALLBACK", "ctv01SHADOWS"]);
        assert_eq!(c.badges.len(), 2, "a badge on another host and one still being made are left out");
    }

    #[test]
    fn linear_paint_points_up_at_zero() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        let p = paint(&c, "01LINEAR");
        assert_eq!(
            p.css,
            "color:#ff0000;background-image:linear-gradient(90deg,#ff0000 0%,#00ff0080 50%,#0000ff 100%);\
             background-position:center;background-size:100% 100%;background-repeat:no-repeat;"
        );
        assert!(p.still.is_none());
    }

    #[test]
    fn radial_and_repeating() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        assert!(paint(&c, "01RADIAL").css.contains("background-image:radial-gradient(ellipse,#111111 0%,#eeeeee 100%)"));
        assert!(paint(&c, "01REPEAT").css.contains("background-image:repeating-linear-gradient(0deg,"));
    }

    #[test]
    fn picture_paints_tile_or_cover_and_have_a_still() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        let p = paint(&c, "01IMAGE");
        assert!(p.css.contains("background-image:url(\"https://cdn.cyex.tv/cosmetics/paints/abc/1x.webp\")"));
        assert!(p.css.contains("background-size:cover"));
        assert!(p.still.is_none(), "not animated");
        let a = paint(&c, "01ANIMATED");
        assert!(a.css.contains("background-repeat:repeat"));
        let still = a.still.as_deref().unwrap();
        assert!(still.contains("1x_static.webp"), "{still}");
        assert!(!a.css.contains("static"));
    }

    #[test]
    fn too_few_stops_fall_back_to_the_plain_colour() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        assert_eq!(paint(&c, "01FALLBACK").css, "color:#abcdef;");
    }

    #[test]
    fn shadows_are_capped() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        let css = &paint(&c, "01SHADOWS").css;
        assert!(
            css.contains("--paint-shadow:drop-shadow(8px -8px 8px #000000) drop-shadow(1.5px 0px 0px #ffffff80) drop-shadow(0px 0px 2px #123456);"),
            "{css}"
        );
    }

    #[test]
    fn bad_values_never_reach_css() {
        let v = json!({
            "paints": [
                { "id": "bad", "name": "x", "kind": "linear_gradient", "angle": 10, "color": "red;}",
                  "stops": [{"at":0,"color":"url(x)"},{"at":1,"color":"#fff"}] },
                { "id": "ok1", "name": "x", "kind": "image", "color": "#111111",
                  "image": { "host": "http://cdn.cyex.tv/a", "ready": true, "files": [{"name":"1x.webp"}] } },
                { "id": "ok2", "name": "x", "kind": "image", "color": "#111111",
                  "image": { "host": "https://cdn.cyex.tv.evil.test/a", "ready": true, "files": [{"name":"1x.webp"}] } },
                { "id": "ok3", "name": "x", "kind": "image", "color": "#111111",
                  "image": { "host": "https://cdn.cyex.tv/a\")", "ready": true, "files": [{"name":"1x.webp"}] } },
                { "id": "a b", "name": "x", "kind": "linear_gradient", "color": "#111111" }
            ],
            "badges": []
        });
        let c = parse_catalog(&v).unwrap();
        // The first has no valid colour and fewer than 2 valid stops: nothing to draw.
        let ids: Vec<_> = c.paints.iter().map(|p| (p.id.as_str(), p.css.as_str())).collect();
        assert_eq!(ids, [("ctvok1", "color:#111111;"), ("ctvok2", "color:#111111;"), ("ctvok3", "color:#111111;")]);
    }

    #[test]
    fn stops_are_cut_and_kept_in_range() {
        let stops: Vec<Value> = (0..14).map(|i| json!({"at": f64::from(i) / 5.0 - 1.0, "color": "#112233"})).collect();
        let v = json!({ "paints": [{ "id": "p", "name": "x", "kind": "linear_gradient", "angle": 900, "stops": stops }] });
        let css = parse_catalog(&v).unwrap().paints.remove(0).css;
        assert!(css.starts_with("background-image:linear-gradient(360deg,"), "{css}");
        assert_eq!(css.matches("#112233").count(), 10);
        assert!(css.contains("#112233 0%,#112233 0%"), "never below 0: {css}");
        assert!(!css.contains(" -"), "{css}");
    }

    #[test]
    fn badges_have_three_sizes_and_the_provider_in_the_tip() {
        let c = parse_catalog(&fixture("cosmetics.json")).unwrap();
        let b = &c.badges[0];
        assert_eq!(b.provider, Provider::Cyextv);
        assert_eq!(b.title, "Early bird · cyex.tv", "the tooltip first");
        assert_eq!(b.image1x, "https://cdn.cyex.tv/cosmetics/badges/b1/1x.webp");
        assert_eq!(b.image2x, "https://cdn.cyex.tv/cosmetics/badges/b1/2x.webp");
        assert_eq!(b.image4x, "https://cdn.cyex.tv/cosmetics/badges/b1/4x.webp");
        assert_eq!(b.still.as_deref(), Some("https://cdn.cyex.tv/cosmetics/badges/b1/1x_static.webp"));
        let n = &c.badges[1];
        assert_eq!(n.title, "Tester · cyex.tv", "the name when there is no tooltip");
        assert_eq!(n.image2x, n.image1x, "a missing size borrows the smaller one");
        assert!(n.still.is_none());
    }

    #[test]
    fn users_are_listed_only_when_they_wear_something() {
        let got = parse_users(&fixture("cosmetics_users.json")).unwrap();
        assert_eq!(
            got,
            [
                Wear { user: "4001".into(), badge: Some("01BADGE".into()), paint: Some("01LINEAR".into()) },
                Wear { user: "4002".into(), badge: None, paint: Some("01IMAGE".into()) },
            ]
        );
        assert!(parse_users(&json!({})).unwrap().is_empty());
        assert!(parse_users(&json!([1])).is_err());
    }

    #[test]
    fn a_catalog_that_is_not_an_object_fails() {
        assert!(parse_catalog(&json!([1])).is_err());
        assert_eq!(parse_catalog(&json!({})).unwrap(), Catalog::default());
    }
}
