//! V1051-03 What the cosmetics menu gets from Rust.

use cyex_chat::model::Provider;
use serde::Serialize;
use ts_rs::TS;

use super::model::{BadgeEntry, Inventory, PaintEntry};
use crate::cosmetics::BadgeDef;
use crate::seventv::paint::PaintStyle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Status {
    NoConsent,
    NoAccount,
    SignedOut,
    Expired,
    Ready,
    Down,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CosmeticsView {
    pub status: Status,
    pub subscribed: bool,
    /// `yyyy/mm/dd`.
    pub sub_end: Option<String>,
    /// `yyyy/mm/dd`.
    pub signed_in_until: Option<String>,
    pub paint: Option<String>,
    pub badge: Option<String>,
    pub paints: Vec<PaintItem>,
    pub badges: Vec<BadgeItem>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaintItem {
    pub style: PaintStyle,
    pub animated: bool,
    pub usable: bool,
    /// The id's time in ms (0 if it isn't a ULID).
    pub added: f64,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeItem {
    pub def: BadgeDef,
    /// Always 4 (1x to 4x), empty where 7TV has none.
    pub images: Vec<String>,
    /// 4, for moving badges only.
    pub stills: Option<Vec<String>>,
    pub animated: bool,
    pub usable: bool,
    pub added: f64,
}

impl CosmeticsView {
    pub fn empty(status: Status) -> Self {
        Self {
            status,
            subscribed: false,
            sub_end: None,
            signed_in_until: None,
            paint: None,
            badge: None,
            paints: Vec::new(),
            badges: Vec::new(),
        }
    }

    pub fn from_inventory(inv: &Inventory, status: Status, signed_in_until: Option<String>) -> Self {
        Self {
            status,
            subscribed: inv.subscribed,
            sub_end: inv.sub_end.as_deref().and_then(date),
            signed_in_until,
            paint: inv.paint.clone(),
            badge: inv.badge.clone(),
            paints: inv.paints.iter().map(paint_item).collect(),
            badges: inv.badges.iter().map(badge_item).collect(),
        }
    }
}

pub fn paint_item(e: &PaintEntry) -> PaintItem {
    PaintItem {
        style: e.style.clone(),
        animated: e.animated,
        usable: e.usable,
        added: added(&e.style.id),
    }
}

pub fn badge_item(e: &BadgeEntry) -> BadgeItem {
    let all = |a: &[Option<String>; 4]| -> Vec<String> {
        a.iter().map(|u| u.clone().unwrap_or_default()).collect()
    };
    let one = |i: usize| e.images[i].clone().unwrap_or_default();
    BadgeItem {
        def: BadgeDef {
            provider: Provider::Seventv,
            id: e.id.clone(),
            title: e.name.clone(),
            image1x: one(0),
            image2x: one(1),
            image4x: one(3),
            still: e.stills[0].clone(),
            color: None,
            replaces: None,
        },
        images: all(&e.images),
        stills: e.animated.then(|| all(&e.stills)),
        animated: e.animated,
        usable: e.usable,
        added: added(&e.id),
    }
}

fn added(id: &str) -> f64 {
    ulid_ms(id).unwrap_or(0) as f64
}

/// The time in a ULID: the first 10 Crockford base32 characters, in ms.
pub fn ulid_ms(id: &str) -> Option<u64> {
    const ALPHA: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let head = id.get(..10)?;
    let mut ms = 0u64;
    for c in head.bytes() {
        let v = ALPHA.iter().position(|&a| a == c.to_ascii_uppercase())?;
        ms = ms.checked_mul(32)?.checked_add(v as u64)?;
    }
    Some(ms)
}

/// `2026-11-15T20:21:04+00:00` to `2026/11/15`.
fn date(iso: &str) -> Option<String> {
    let d = iso.get(..10)?;
    let ok = d.bytes().enumerate().all(|(i, b)| {
        if i == 4 || i == 7 {
            b == b'-'
        } else {
            b.is_ascii_digit()
        }
    });
    ok.then(|| d.replace('-', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seventv::account::model::parse_user;

    #[test]
    fn ulid_time() {
        let ms = ulid_ms("01JQM5X9N65Z6M8Q3Z6RV8BDT4").unwrap();
        // 2025/03/30 to 2025/04/01 in ms.
        assert!((1_743_300_000_000..1_743_600_000_000).contains(&ms), "{ms}");
        assert_eq!(ulid_ms("not-a-ulid"), None);
        assert_eq!(added("x"), 0.0);
    }

    #[test]
    fn dates_are_yyyy_mm_dd() {
        assert_eq!(date("2026-11-15T20:21:04+00:00").as_deref(), Some("2026/11/15"));
        assert_eq!(date("soon"), None);
    }

    #[test]
    fn view_from_fixture() {
        let user = super::super::model::tests::fixture("v4_inventory_cyexbot.json");
        let inv = parse_user(user).unwrap().unwrap();
        let v = CosmeticsView::from_inventory(&inv, Status::Ready, None);
        assert_eq!(v.sub_end.as_deref(), Some("2026/11/15"));
        let b = v.badges.iter().find(|b| b.animated).unwrap();
        assert_eq!(b.images.len(), 4);
        assert_eq!(b.stills.as_ref().unwrap().len(), 4);
        assert!(v.paints[0].added > 0.0);
    }

    #[test]
    fn big_inventory_maps_fast() {
        let paints: Vec<_> = (0..2000)
            .map(|i| {
                serde_json::json!({"accessible": true, "to": {"paint": {
                    "id": format!("01JQM5X9N65Z6M8Q3Z6R{i:06}"), "name": format!("p{i}"),
                    "data": {"layers": [{"ty": {"__typename": "PaintLayerTypeLinearGradient",
                        "angle": 90, "repeating": false,
                        "stops": [{"at": 0, "color": {"hex": "#FF0000FF"}}, {"at": 1, "color": {"hex": "#0000FFFF"}}]}}],
                    "shadows": [{"color": {"hex": "#000000FF"}, "offsetX": 0, "offsetY": 0, "blur": 1}]}}}})
            })
            .collect();
        let badges: Vec<_> = (0..300)
            .map(|i| {
                serde_json::json!({"accessible": true, "to": {"badge": {
                    "id": format!("b{i}"), "name": "b",
                    "images": [{"url": format!("https://cdn.7tv.app/badge/b{i}/1x.webp"), "mime": "image/webp", "scale": 1, "frameCount": 1}]}}})
            })
            .collect();
        let user = serde_json::json!({"id": "u", "inventory": {"paints": paints, "badges": badges}});
        let t = std::time::Instant::now();
        let inv = parse_user(user).unwrap().unwrap();
        let v = CosmeticsView::from_inventory(&inv, Status::Ready, None);
        assert_eq!(v.paints.len(), 2000);
        assert_eq!(v.badges.len(), 300);
        assert!(t.elapsed().as_millis() < 100, "{:?}", t.elapsed());
    }
}
