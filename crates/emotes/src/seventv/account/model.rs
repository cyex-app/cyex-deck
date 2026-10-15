//! V1051-03 7TV's v4 inventory answer, typed (research §2, §4, §6).

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::Error;
use crate::seventv::paint::{self, Paint};

/// A public account's cosmetics.
#[derive(Debug, Clone)]
pub struct Inventory {
    pub seventv_id: String,
    pub paint: Option<String>,
    pub badge: Option<String>,
    pub subscribed: bool,
    /// `activePeriod.end`, as 7TV gave it.
    pub sub_end: Option<String>,
    pub paints: Vec<PaintEntry>,
    pub badges: Vec<BadgeEntry>,
    /// Paints that draw nothing (a picture layer with no pictures), for the log.
    pub skipped: usize,
}

#[derive(Debug, Clone)]
pub struct PaintEntry {
    pub paint: Paint,
    pub animated: bool,
    pub usable: bool,
}

#[derive(Debug, Clone)]
pub struct BadgeEntry {
    pub id: String,
    pub name: String,
    /// 1x to 4x, WebP only.
    pub images: [Option<String>; 4],
    /// Still frames, only when the badge moves.
    pub stills: [Option<String>; 4],
    pub animated: bool,
    pub usable: bool,
}

#[derive(Deserialize)]
struct RawUser {
    id: String,
    style: Option<RawStyle>,
    billing: Option<RawBilling>,
    inventory: Option<RawInventory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStyle {
    active_paint_id: Option<String>,
    active_badge_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBilling {
    subscription_info: Option<RawSubInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSubInfo {
    active_period: Option<RawPeriod>,
}

#[derive(Deserialize)]
struct RawPeriod {
    end: Option<String>,
}

#[derive(Deserialize)]
struct RawInventory {
    #[serde(default)]
    paints: Vec<RawPaintEdge>,
    #[serde(default)]
    badges: Vec<RawBadgeEdge>,
}

#[derive(Deserialize)]
struct RawPaintEdge {
    #[serde(default)]
    accessible: bool,
    to: Option<RawPaintTo>,
}

#[derive(Deserialize)]
struct RawPaintTo {
    paint: Option<V4Paint>,
}

#[derive(Deserialize)]
struct RawBadgeEdge {
    #[serde(default)]
    accessible: bool,
    to: Option<RawBadgeTo>,
}

#[derive(Deserialize)]
struct RawBadgeTo {
    badge: Option<RawBadge>,
}

#[derive(Deserialize)]
struct RawBadge {
    id: String,
    name: String,
    #[serde(default)]
    images: Vec<V4Image>,
}

/// A v4 paint definition. `opacity` is always 1 today, so it isn't read.
#[derive(Debug, Deserialize)]
pub(crate) struct V4Paint {
    pub id: String,
    pub name: String,
    pub data: V4Data,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct V4Data {
    #[serde(default)]
    pub layers: Vec<V4Layer>,
    #[serde(default)]
    pub shadows: Vec<V4Shadow>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V4Layer {
    pub ty: V4Ty,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
pub(crate) enum V4Ty {
    #[serde(rename = "PaintLayerTypeSingleColor")]
    SingleColor { color: V4Color },
    #[serde(rename = "PaintLayerTypeLinearGradient")]
    Linear {
        #[serde(default)]
        angle: f64,
        #[serde(default)]
        repeating: bool,
        #[serde(default)]
        stops: Vec<V4Stop>,
    },
    #[serde(rename = "PaintLayerTypeRadialGradient")]
    Radial {
        #[serde(default)]
        shape: String,
        #[serde(default)]
        repeating: bool,
        #[serde(default)]
        stops: Vec<V4Stop>,
    },
    #[serde(rename = "PaintLayerTypeImage")]
    Image {
        #[serde(default)]
        images: Vec<V4Image>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V4Color {
    pub hex: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct V4Stop {
    pub at: f64,
    pub color: V4Color,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct V4Shadow {
    pub color: V4Color,
    #[serde(default)]
    pub offset_x: f64,
    #[serde(default)]
    pub offset_y: f64,
    #[serde(default)]
    pub blur: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct V4Image {
    pub url: Option<String>,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub scale: u32,
    #[serde(default)]
    pub frame_count: u32,
}

impl V4Image {
    fn webp_at(&self, scale: u32) -> bool {
        self.mime == "image/webp" && self.scale == scale && self.url.is_some()
    }

    /// The URL, if it is on 7TV's CDN and safe for CSS.
    pub(crate) fn safe(&self) -> Option<String> {
        self.url.clone().filter(|u| paint::safe_url(u))
    }
}

/// The WebP of this scale with the most frames (the moving one, if there is one).
pub(crate) fn best_webp(images: &[V4Image], scale: u32) -> Option<&V4Image> {
    images
        .iter()
        .filter(|i| i.webp_at(scale))
        .max_by_key(|i| i.frame_count)
}

/// The still WebP of this scale.
pub(crate) fn still_webp(images: &[V4Image], scale: u32) -> Option<&V4Image> {
    images.iter().find(|i| i.webp_at(scale) && i.frame_count <= 1)
}

/// Reads `users.userByConnection`. `null` is no 7TV user.
pub fn parse_user(user: &Value) -> Result<Option<Inventory>, Error> {
    if user.is_null() {
        return Ok(None);
    }
    let raw: RawUser = serde_json::from_value(user.clone()).map_err(|_| Error::Bad)?;
    let period = raw
        .billing
        .and_then(|b| b.subscription_info)
        .and_then(|s| s.active_period);
    let (paint, badge) = raw
        .style
        .map_or((None, None), |s| (s.active_paint_id, s.active_badge_id));
    let inv = raw.inventory.unwrap_or(RawInventory {
        paints: Vec::new(),
        badges: Vec::new(),
    });

    let mut skipped = 0;
    let mut paints: Vec<PaintEntry> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for edge in inv.paints {
        let Some(p) = edge.to.and_then(|t| t.paint) else {
            continue;
        };
        if let Some(&i) = seen.get(&p.id) {
            paints[i].usable |= edge.accessible;
            continue;
        }
        let mapped = paint::from_v4(&p);
        if mapped.style().css.is_empty() {
            skipped += 1;
            continue;
        }
        seen.insert(p.id.clone(), paints.len());
        let animated = p.data.layers.iter().any(|l| match &l.ty {
            V4Ty::Image { images } => images.iter().any(|i| i.frame_count > 1),
            _ => false,
        });
        paints.push(PaintEntry {
            paint: mapped,
            animated,
            usable: edge.accessible,
        });
    }

    let mut badges: Vec<BadgeEntry> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for edge in inv.badges {
        let Some(b) = edge.to.and_then(|t| t.badge) else {
            continue;
        };
        if let Some(&i) = seen.get(&b.id) {
            badges[i].usable |= edge.accessible;
            continue;
        }
        seen.insert(b.id.clone(), badges.len());
        badges.push(badge_entry(b, edge.accessible));
    }

    Ok(Some(Inventory {
        seventv_id: raw.id,
        paint,
        badge,
        subscribed: period.is_some(),
        sub_end: period.and_then(|p| p.end),
        paints,
        badges,
        skipped,
    }))
}

fn badge_entry(b: RawBadge, usable: bool) -> BadgeEntry {
    let animated = b.images.iter().any(|i| i.frame_count > 1);
    let mut images: [Option<String>; 4] = Default::default();
    let mut stills: [Option<String>; 4] = Default::default();
    for n in 1..=4u32 {
        let k = n as usize - 1;
        images[k] = best_webp(&b.images, n).and_then(V4Image::safe);
        if animated {
            stills[k] = still_webp(&b.images, n).and_then(V4Image::safe);
        }
    }
    BadgeEntry {
        id: b.id,
        name: b.name,
        images,
        stills,
        animated,
        usable,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/7tv/{name}", env!("CARGO_MANIFEST_DIR"));
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        v.pointer("/data/users/userByConnection").unwrap().clone()
    }

    #[test]
    fn cyexbot_inventory() {
        let inv = parse_user(&fixture("v4_inventory_cyexbot.json"))
            .unwrap()
            .unwrap();
        assert!(inv.subscribed);
        assert_eq!(inv.paints.len(), 3);
        assert_eq!(inv.badges.len(), 3);
        assert!(inv.paints.iter().all(|p| p.usable));
        assert_eq!(inv.paints.iter().filter(|p| p.animated).count(), 1);
        assert_eq!(inv.paint.as_deref(), Some("01KC2M1QNZ1Q1E47KPM14VSERH"));
    }

    #[test]
    fn dedupes_and_usable_from_any_edge() {
        let inv = parse_user(&fixture("v4_inventory_no_sub.json")).unwrap().unwrap();
        assert!(!inv.subscribed);
        assert_eq!(inv.paints.len(), 3);
        let usable: Vec<_> = inv.paints.iter().map(|p| p.usable).collect();
        assert_eq!(usable, [false, false, true]);
    }

    #[test]
    fn no_user_is_none() {
        let v = fixture("v4_inventory_no_user.json");
        assert!(parse_user(&v).unwrap().is_none());
    }

    #[test]
    fn badge_pictures_are_webp_only() {
        let inv = parse_user(&fixture("v4_inventory_cyexbot.json"))
            .unwrap()
            .unwrap();
        let moving = inv.badges.iter().find(|b| b.animated).unwrap();
        for u in moving.images.iter().chain(&moving.stills).flatten() {
            assert!(u.ends_with(".webp"), "{u}");
        }
        assert!(moving.images.iter().all(Option::is_some));
        assert!(moving.stills[0].as_deref().unwrap().contains("1x_static"));
        let still = inv.badges.iter().find(|b| !b.animated).unwrap();
        assert!(still.stills.iter().all(Option::is_none));
    }

    #[test]
    fn foreign_badge_urls_are_dropped() {
        let img = |url: &str, scale| V4Image {
            url: Some(url.into()),
            mime: "image/webp".into(),
            scale,
            frame_count: 1,
        };
        let b = RawBadge {
            id: "b".into(),
            name: "b".into(),
            images: vec![
                img("https://evil.example/1x.webp", 1),
                img("https://cdn.7tv.app/badge/b/2x.webp", 2),
            ],
        };
        let e = badge_entry(b, true);
        assert!(e.images[0].is_none());
        assert!(e.images[1].is_some());
    }
}
