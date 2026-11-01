//! V108-E1 What an account wears now (query `WEARING`), for its own lines. 7TV sends a
//! chatter's paint and badge only after a presence, and only to sockets already listening
//! there, so the app reads its own straight from the profile.

use std::collections::HashSet;

use serde::Deserialize;
use serde_json::Value;

use super::Error;
use super::error::classify;
use super::model::{RawBadge, V4Paint, badge_entry};
use super::view::badge_item;
use crate::cosmetics::BadgeDef;
use crate::seventv::paint::{self, PaintStyle};

/// The paint and badge others see on the account's lines. `None` is none worn, or one it
/// may not wear any more (7TV's presence leaves those out too), or one that draws nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Wearing {
    pub seventv_id: String,
    pub paint: Option<PaintStyle>,
    pub badge: Option<BadgeDef>,
}

#[derive(Deserialize)]
struct RawUser {
    id: String,
    style: Option<RawStyle>,
    inventory: Option<RawInventory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStyle {
    active_paint: Option<V4Paint>,
    active_badge: Option<RawBadge>,
}

#[derive(Deserialize)]
struct RawInventory {
    #[serde(default)]
    paints: Vec<Edge<PaintTo>>,
    #[serde(default)]
    badges: Vec<Edge<BadgeTo>>,
}

#[derive(Deserialize)]
struct Edge<T> {
    to: Option<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaintTo {
    paint_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BadgeTo {
    badge_id: Option<String>,
}

/// Reads a `users.userByConnection` value. `null` is no 7TV user.
pub fn parse(user: Value) -> Result<Option<Wearing>, Error> {
    let raw: Option<RawUser> = serde_json::from_value(user).map_err(|_| Error::Bad)?;
    Ok(raw.map(wearing))
}

/// The whole answer: status, rate-limit hint and body.
pub fn read(status: u16, reset: Option<u64>, body: &[u8]) -> Result<Option<Wearing>, Error> {
    let data = classify(status, reset, body)?;
    parse(data.pointer("/users/userByConnection").cloned().unwrap_or(Value::Null))
}

fn wearing(raw: RawUser) -> Wearing {
    let inv = raw.inventory.unwrap_or(RawInventory {
        paints: Vec::new(),
        badges: Vec::new(),
    });
    let paints: HashSet<String> = inv
        .paints
        .into_iter()
        .filter_map(|e| e.to?.paint_id)
        .collect();
    let badges: HashSet<String> = inv
        .badges
        .into_iter()
        .filter_map(|e| e.to?.badge_id)
        .collect();
    let (paint, badge) = raw
        .style
        .map_or((None, None), |s| (s.active_paint, s.active_badge));
    let paint = paint
        .filter(|p| paints.contains(&p.id))
        .map(|p| paint::from_v4(&p).style())
        .filter(|s| !s.css.is_empty());
    let badge = badge
        .map(|b| badge_entry(b, true))
        .filter(|e| badges.contains(&e.id) && e.images.iter().any(Option::is_some))
        .map(|e| badge_item(&e).def);
    Wearing {
        seventv_id: raw.id,
        paint,
        badge,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::seventv::account::model::tests::fixture;

    const PAINT: &str = "01KC2M1QNZ1Q1E47KPM14VSERH";
    const BADGE: &str = "01KB4X7XGZWPSJBP7FTFSRGZ90";

    #[test]
    fn caressz_wears_a_paint_and_a_badge() {
        let w = parse(fixture("v4_wearing_caressz.json")).unwrap().unwrap();
        assert_eq!(w.seventv_id, "01KB7N57EYTF67B9V4JY3SB6RD");
        let p = w.paint.unwrap();
        assert_eq!((p.id.as_str(), p.name.as_str()), (PAINT, "Evelone"));
        assert!(p.css.contains("cdn.7tv.app/paint/"), "{}", p.css);
        let b = w.badge.unwrap();
        assert_eq!((b.id.as_str(), b.title.as_str()), (BADGE, "NNYS 2025"));
        assert!(b.image1x.ends_with("1x.webp"), "{}", b.image1x);
        assert!(!b.image4x.is_empty());
    }

    #[test]
    fn no_user_is_none() {
        assert!(parse(fixture("v4_inventory_no_user.json")).unwrap().is_none());
        assert!(parse(Value::Null).unwrap().is_none());
    }

    #[test]
    fn worn_but_not_owned_now_is_left_out() {
        let mut v = fixture("v4_wearing_caressz.json");
        v["inventory"] = json!({ "paints": [], "badges": [] });
        let w = parse(v).unwrap().unwrap();
        assert_eq!((w.paint, w.badge), (None, None));
    }

    #[test]
    fn nothing_worn() {
        let mut v = fixture("v4_wearing_caressz.json");
        v["style"] = json!({ "activePaint": null, "activeBadge": null });
        let w = parse(v).unwrap().unwrap();
        assert_eq!((w.paint, w.badge), (None, None));
    }

    #[test]
    fn whole_answers() {
        let body = |user: Value| serde_json::to_vec(&json!({ "data": { "users": { "userByConnection": user } } })).unwrap();
        let ok = read(200, None, &body(fixture("v4_wearing_caressz.json"))).unwrap().unwrap();
        assert!(ok.paint.is_some() && ok.badge.is_some());
        assert_eq!(read(200, None, &body(Value::Null)), Ok(None));
        assert_eq!(read(500, None, b"{}"), Err(Error::Down));
        assert_eq!(read(429, Some(5), b"{}"), Err(Error::RateLimited { seconds: 5 }));
    }
}
