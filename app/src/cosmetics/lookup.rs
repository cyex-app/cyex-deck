//! V1051-04 Questions about an inventory, with no 7TV call and no paint CSS: what's owned,
//! what's worn, what a typed name means.

use cyex_emotes::seventv::account::model::Inventory;
use cyex_settings::schema::{Combo, ComboItem};

use super::Kind;

#[derive(Debug, PartialEq, Eq)]
pub enum Have {
    Usable,
    NeedsSub,
    NotOwned,
}

/// Id, name and whether it can be worn now, for every entry of one kind.
fn entries(inv: &Inventory, kind: Kind) -> Vec<(&str, &str, bool)> {
    match kind {
        Kind::Paint => inv
            .paints
            .iter()
            .map(|e| (e.paint.id.as_str(), e.paint.name.as_str(), e.usable))
            .collect(),
        Kind::Badge => inv
            .badges
            .iter()
            .map(|e| (e.id.as_str(), e.name.as_str(), e.usable))
            .collect(),
    }
}

#[allow(dead_code)] // V1051-04 group 4 (chat commands) uses this
fn fold(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

pub fn have(inv: &Inventory, kind: Kind, id: &str) -> Have {
    match entries(inv, kind).into_iter().find(|e| e.0 == id) {
        Some((_, _, true)) => Have::Usable,
        Some(_) => Have::NeedsSub,
        None => Have::NotOwned,
    }
}

/// `/paint` and `/badge`: an id (any case) or a name (case-insensitive, spaces collapsed).
/// Several with that name: a usable one first.
#[allow(dead_code)] // V1051-04 group 4 (chat commands) uses this
pub fn find(inv: &Inventory, kind: Kind, typed: &str) -> Option<ComboItem> {
    let all = entries(inv, kind);
    let typed = typed.trim();
    let hit = all.iter().find(|e| e.0.eq_ignore_ascii_case(typed)).or_else(|| {
        let want = fold(typed);
        let named = |usable: bool| all.iter().find(|e| e.2 == usable && fold(e.1) == want);
        named(true).or_else(|| named(false))
    })?;
    Some(ComboItem {
        id: hit.0.to_owned(),
        name: hit.1.to_owned(),
    })
}

fn item(inv: &Inventory, kind: Kind, id: &str) -> ComboItem {
    let name = entries(inv, kind)
        .into_iter()
        .find(|e| e.0 == id)
        .map_or(id, |e| e.1);
    ComboItem {
        id: id.to_owned(),
        name: name.to_owned(),
    }
}

/// What's worn, paint then badge. An id the inventory doesn't list keeps its id as its name.
pub fn worn(inv: &Inventory) -> (Option<ComboItem>, Option<ComboItem>) {
    (
        inv.paint.as_deref().map(|id| item(inv, Kind::Paint, id)),
        inv.badge.as_deref().map(|id| item(inv, Kind::Badge, id)),
    )
}

/// The first combo (list order) whose paint and badge ids equal what's worn.
pub fn current<'a>(combos: &'a [Combo], inv: &Inventory) -> Option<&'a Combo> {
    combos.iter().find(|c| {
        c.paint.as_ref().map(|p| p.id.as_str()) == inv.paint.as_deref()
            && c.badge.as_ref().map(|b| b.id.as_str()) == inv.badge.as_deref()
    })
}

/// Usable names, once each, A-Z: Tab's candidates.
#[allow(dead_code)] // V1051-04 group 4 (chat commands) uses this
pub fn names(inv: &Inventory, kind: Kind) -> Vec<String> {
    let mut out: Vec<String> = entries(inv, kind)
        .into_iter()
        .filter(|e| e.2)
        .map(|e| e.1.to_owned())
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    out.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
    out
}

#[cfg(test)]
pub(super) mod tests {
    use cyex_emotes::seventv::account::model::{BadgeEntry, Inventory, PaintEntry};

    use super::*;

    pub fn paint(id: &str, name: &str, usable: bool) -> PaintEntry {
        PaintEntry {
            paint: serde_json::from_value(serde_json::json!({"id": id, "name": name})).unwrap(),
            animated: false,
            usable,
        }
    }

    pub fn badge(id: &str, name: &str, usable: bool) -> BadgeEntry {
        BadgeEntry {
            id: id.into(),
            name: name.into(),
            images: Default::default(),
            stills: Default::default(),
            animated: false,
            usable,
        }
    }

    pub fn inv() -> Inventory {
        Inventory {
            seventv_id: "7tv".into(),
            paint: Some("P1".into()),
            badge: Some("B1".into()),
            subscribed: false,
            sub_end: None,
            paints: vec![
                paint("P1", "Gold", true),
                paint("P2", "Sub Glow", false),
                paint("P3", "Twin", false),
                paint("P4", "Twin", true),
            ],
            badges: vec![badge("B1", "Star", true), badge("B2", "Moon", true)],
            skipped: 0,
        }
    }

    fn combo(name: &str, p: Option<&str>, b: Option<&str>) -> Combo {
        let it = |id: &str| ComboItem {
            id: id.into(),
            name: id.into(),
        };
        Combo {
            id: name.into(),
            name: name.into(),
            paint: p.map(it),
            badge: b.map(it),
        }
    }

    #[test]
    fn have_answers_all_three() {
        let i = inv();
        assert_eq!(have(&i, Kind::Paint, "P1"), Have::Usable);
        assert_eq!(have(&i, Kind::Paint, "P2"), Have::NeedsSub);
        assert_eq!(have(&i, Kind::Badge, "P1"), Have::NotOwned);
    }

    #[test]
    fn find_by_id_name_and_duplicate() {
        let i = inv();
        assert_eq!(find(&i, Kind::Paint, "p1").unwrap().id, "P1");
        assert_eq!(find(&i, Kind::Paint, "  sub   GLOW ").unwrap().id, "P2");
        assert_eq!(find(&i, Kind::Paint, "twin").unwrap().id, "P4");
        assert_eq!(find(&i, Kind::Badge, "moon").unwrap().name, "Moon");
        assert!(find(&i, Kind::Badge, "nope").is_none());
    }

    #[test]
    fn worn_names_and_unknown_ids() {
        let mut i = inv();
        let (p, b) = worn(&i);
        assert_eq!((p.unwrap().name, b.unwrap().name), ("Gold".into(), "Star".into()));
        i.paint = Some("GONE".into());
        i.badge = None;
        let (p, b) = worn(&i);
        assert_eq!(p.unwrap().name, "GONE");
        assert!(b.is_none());
    }

    #[test]
    fn current_is_the_first_match() {
        let i = inv();
        let list = [
            combo("a", Some("P1"), None),
            combo("b", Some("P1"), Some("B1")),
            combo("c", Some("P1"), Some("B1")),
        ];
        assert_eq!(current(&list, &i).unwrap().name, "b");
        assert!(current(&list[..1], &i).is_none());
    }

    #[test]
    fn names_are_usable_once_and_sorted() {
        assert_eq!(names(&inv(), Kind::Paint), ["Gold", "Twin"]);
        assert_eq!(names(&inv(), Kind::Badge), ["Moon", "Star"]);
    }
}
