//! V108-N06 Chatsen: one JSON in the project's `resources` repo with the badge catalog (one
//! picture each) and the badges each Twitch id has. A person shows their most important one.

use std::collections::HashMap;

use serde::Deserialize;

use super::{BadgeSet, Loaded, def, picture, slug, title};

pub const URL: &str = "https://raw.githubusercontent.com/chatsen/resources/master/assets/data.json";

/// Most important first; anything else ranks after these.
const ORDER: [&str; 12] = [
    "developer",
    "early_supporter",
    "performance_artist",
    "early_bird",
    "relaxo",
    "patreon_tier4",
    "patreon_tier3s",
    "patreon_tier3",
    "patreon_tier2s",
    "patreon_tier2",
    "patreon_tier1s",
    "patreon_tier1",
];

#[derive(Deserialize)]
pub struct Raw {
    #[serde(default)]
    badges: Vec<Badge>,
    #[serde(default)]
    users: Vec<User>,
}

#[derive(Deserialize)]
struct Badge {
    #[serde(default)]
    name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    image: String,
}

#[derive(Deserialize)]
struct User {
    #[serde(default)]
    id: String,
    #[serde(default)]
    badges: Vec<Worn>,
}

#[derive(Deserialize)]
struct Worn {
    #[serde(default, rename = "badgeName")]
    name: String,
}

impl From<Raw> for Loaded {
    fn from(raw: Raw) -> Self {
        let mut out = Loaded::default();
        let mut known: HashMap<String, String> = HashMap::new();
        for b in raw.badges {
            let id = slug(&b.name);
            if id.is_empty() || !picture(&b.image) {
                continue;
            }
            let t = if b.title.trim().is_empty() { &b.name } else { &b.title };
            known.insert(b.name.clone(), format!("chatsen:{id}"));
            out.defs.push(def(BadgeSet::Chatsen, &id, &title(t), [&b.image, &b.image, &b.image]));
        }
        for u in raw.users {
            let best = u
                .badges
                .iter()
                .filter(|w| known.contains_key(&w.name))
                .min_by_key(|w| ORDER.iter().position(|o| *o == w.name).unwrap_or(ORDER.len()));
            if let Some(w) = best {
                out.give(&u.id, &known[&w.name]);
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
    fn the_most_important_badge_shows() {
        let l: Loaded = fixture::<Raw>("chatsen.json").into();
        assert_eq!(l.defs.len(), 3, "the one without a picture is skipped");
        assert_eq!(l.users["693065895"], ["chatsen:developer"]);
        assert_eq!(l.users["57459239"], ["chatsen:early-bird"]);
        assert_eq!(l.users["1"], ["chatsen:patreon-tier1"], "patreon_tier2 has no entry, tier1 does");
        assert!(!l.users.contains_key("2"), "only unknown badges");
        let dev = l.defs.iter().find(|d| d.id == "chatsen:developer").unwrap();
        assert_eq!(dev.title, "Chatsen Developer");
        assert_eq!(dev.image1x, dev.image4x);
        let tier = l.defs.iter().find(|d| d.id == "chatsen:patreon-tier1").unwrap();
        assert_eq!(tier.title, "patreon_tier1", "no title: the name");
    }

    #[test]
    fn empty_and_malformed() {
        assert_eq!(Loaded::from(serde_json::from_str::<Raw>("{}").unwrap()), Loaded::default());
        assert!(serde_json::from_str::<Raw>("3").is_err());
        assert!(serde_json::from_str::<Raw>("404: Not Found").is_err());
    }
}
