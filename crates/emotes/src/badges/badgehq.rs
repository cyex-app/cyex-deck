//! V108-N06 BadgeHQ: one list at `www.badgehq.xyz/api/v1/badges` with the badges (1x to 3x
//! pictures, a priority, a still frame for moving ones) and the badge names each Twitch id wears.

use std::collections::HashMap;

use serde::Deserialize;

use super::{BadgeSet, Loaded, def, picture, title};

pub const URL: &str = "https://www.badgehq.xyz/api/v1/badges";

#[derive(Deserialize)]
pub struct Raw {
    #[serde(default)]
    badges: Vec<Badge>,
    #[serde(default)]
    users: HashMap<String, User>,
}

#[derive(Deserialize)]
struct Badge {
    #[serde(default)]
    id: String,
    #[serde(default)]
    tooltip: String,
    /// Lower comes first.
    #[serde(default)]
    priority: i64,
    #[serde(default)]
    animated: bool,
    #[serde(default)]
    images: Pictures,
    #[serde(default, rename = "staticImages")]
    still: Pictures,
}

#[derive(Default, Deserialize)]
struct Pictures {
    #[serde(default, rename = "1x")]
    x1: String,
    #[serde(default, rename = "2x")]
    x2: String,
    #[serde(default, rename = "3x")]
    x3: String,
}

#[derive(Deserialize)]
struct User {
    #[serde(default)]
    badges: Vec<String>,
}

impl From<Raw> for Loaded {
    fn from(raw: Raw) -> Self {
        let mut out = Loaded::default();
        let mut priority: HashMap<String, i64> = HashMap::new();
        for b in raw.badges {
            let name = super::slug(&b.id);
            let t = title(&b.tooltip);
            let i = &b.images;
            if name.is_empty() || b.id.len() > 64 || t.is_empty() || !picture(&i.x1) {
                continue;
            }
            // 1x is 18 px, 2x 36, 3x 72; a missing size falls back to the next smaller.
            let x2 = if picture(&i.x2) { &i.x2 } else { &i.x1 };
            let x3 = if picture(&i.x3) { &i.x3 } else { x2 };
            let mut badge = def(BadgeSet::BadgeHq, &name, &t, [&i.x1, x2, x3]);
            if b.animated {
                badge.still = [&b.still.x2, &b.still.x1].into_iter().find(|u| picture(u)).cloned();
            }
            priority.insert(b.id, b.priority);
            out.defs.push(badge);
        }
        for (user, worn) in raw.users {
            let mut ids: Vec<(i64, String)> = worn
                .badges
                .iter()
                .filter_map(|b| Some((*priority.get(b)?, format!("badgehq:{}", super::slug(b)))))
                .collect();
            ids.sort();
            for (_, id) in ids {
                out.give(&user, &id);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture;
    use super::*;

    #[test]
    fn badges_are_worn_by_priority() {
        let l: Loaded = fixture::<Raw>("badgehq.json").into();
        assert_eq!(l.defs.len(), 3, "the one with a bad picture is skipped");
        assert_eq!(l.users["38835453"], ["badgehq:founder"]);
        assert_eq!(l.users["5"], ["badgehq:developer", "badgehq:supporter"]);
        assert!(!l.users.contains_key("not-a-number"));
        let founder = l.defs.iter().find(|d| d.id == "badgehq:founder").unwrap();
        assert_eq!(founder.title, "BadgeHQ Founder");
        assert_eq!(founder.image4x, "https://badgehq.example/badges/v1/founder-3x.png?v=3", "4x is empty");
        assert_eq!(founder.still.as_deref(), Some("https://badgehq.example/badges/v1/founder-static-2x.png"));
        let dev = l.defs.iter().find(|d| d.id == "badgehq:developer").unwrap();
        assert_eq!(dev.still, None);
    }

    #[test]
    fn empty_and_malformed() {
        assert_eq!(Loaded::from(serde_json::from_str::<Raw>("{}").unwrap()), Loaded::default());
        assert_eq!(
            Loaded::from(serde_json::from_str::<Raw>(r#"{"badges":[],"users":{"1":{"badges":["x"]}}}"#).unwrap()),
            Loaded::default(),
            "unknown badge names wear nothing"
        );
        assert!(serde_json::from_str::<Raw>("3").is_err());
        assert!(serde_json::from_str::<Raw>("").is_err());
    }
}
