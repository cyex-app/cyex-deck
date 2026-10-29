//! V108-N06 Chatterino Homies, three lists: the project's own badges (two lists on
//! `itzalex.github.io`, a badge with its Twitch ids) and badges people made for themselves
//! (`chatterinohomies.com/api/badges/list`, about 4 MB, one badge per Twitch id). A list that
//! fails is skipped; the set fails only when all three do.

use futures_util::future::join3;
use serde::Deserialize;

use cyex_twitch::{Http, Result};

use super::{BadgeSet, Loaded, def, picture, title, user_id};

pub const MADE: &str = "https://chatterinohomies.com/api/badges/list";
pub const TEAM: &str = "https://itzalex.github.io/badges";
pub const TEAM_MORE: &str = "https://itzalex.github.io/badges2";

#[derive(Deserialize)]
pub struct Made {
    #[serde(default)]
    badges: Vec<Own>,
}

#[derive(Deserialize)]
struct Own {
    #[serde(default, rename = "userId")]
    user: String,
    #[serde(default)]
    tooltip: String,
    #[serde(default)]
    image1: String,
    #[serde(default)]
    image2: String,
    #[serde(default)]
    image3: String,
}

#[derive(Deserialize)]
pub struct Team {
    #[serde(default)]
    badges: Vec<Shared>,
}

#[derive(Deserialize)]
struct Shared {
    #[serde(default)]
    tooltip: String,
    #[serde(default)]
    image1: String,
    #[serde(default)]
    image2: String,
    #[serde(default)]
    image3: String,
    #[serde(default)]
    users: Vec<String>,
}

pub async fn fetch(http: &Http) -> Result<Loaded> {
    let (made, team, more) = join3(
        http.get_json::<Made>(MADE),
        http.get_json::<Team>(TEAM),
        http.get_json::<Team>(TEAM_MORE),
    )
    .await;
    match (made, team, more) {
        (Err(e), Err(_), Err(_)) => Err(e),
        (made, team, more) => Ok(combine(made.ok(), team.ok(), more.ok())),
    }
}

/// The team's badges first, then the ones people made.
pub fn combine(made: Option<Made>, team: Option<Team>, more: Option<Team>) -> Loaded {
    let mut out = Loaded::default();
    for (tag, list) in [("a", team), ("b", more)] {
        for (i, b) in list.into_iter().flat_map(|t| t.badges).enumerate() {
            let t = title(&b.tooltip);
            if t.is_empty() || !picture(&b.image1) {
                continue;
            }
            let x2 = if picture(&b.image2) { &b.image2 } else { &b.image1 };
            let x3 = if picture(&b.image3) { &b.image3 } else { x2 };
            let badge = def(BadgeSet::Homies, &format!("{tag}{i}"), &t, [&b.image1, x2, x3]);
            // The second list ships with an empty id as a placeholder.
            for u in b.users.iter().filter(|u| user_id(u)) {
                out.give(u, &badge.id);
            }
            out.defs.push(badge);
        }
    }
    for b in made.into_iter().flat_map(|m| m.badges) {
        let t = title(&b.tooltip);
        if !user_id(&b.user) || t.is_empty() || !picture(&b.image1) {
            continue;
        }
        let x2 = if picture(&b.image2) { &b.image2 } else { &b.image1 };
        let x3 = if picture(&b.image3) { &b.image3 } else { x2 };
        let badge = def(BadgeSet::Homies, &format!("u{}", b.user), &t, [&b.image1, x2, x3]);
        out.give(&b.user, &badge.id);
        out.defs.push(badge);
    }
    // Only pictures somebody wears are kept.
    let worn: std::collections::HashSet<&String> = out.users.values().flatten().collect();
    let keep: std::collections::HashSet<String> = out.defs.iter().filter(|d| worn.contains(&d.id)).map(|d| d.id.clone()).collect();
    out.defs.retain(|d| keep.contains(&d.id));
    out
}

#[cfg(test)]
mod tests {
    use super::super::fixture;
    use super::*;

    fn load() -> Loaded {
        combine(
            Some(fixture::<Made>("homies_made.json")),
            Some(fixture::<Team>("homies_team.json")),
            Some(fixture::<Team>("homies_team_more.json")),
        )
    }

    #[test]
    fn team_badges_come_before_made_ones() {
        let l = load();
        assert_eq!(l.users["59842770"], ["homies:a0", "homies:u59842770"]);
        assert_eq!(l.users["92888412"], ["homies:a1"]);
        assert_eq!(l.users["95700563"], ["homies:u95700563"]);
        let dev = l.defs.iter().find(|d| d.id == "homies:a0").unwrap();
        assert_eq!(dev.title, "Homies Developer");
        assert_eq!(dev.image2x, "https://itzalex.github.io/badgesusers/dev/badge2x.png");
        let own = l.defs.iter().find(|d| d.id == "homies:u95700563").unwrap();
        assert!(own.image4x.ends_with("/72.webp"));
        assert_eq!(l.users["7"], ["homies:b0"], "the second list");
    }

    #[test]
    fn nobody_and_junk_are_dropped() {
        let l = load();
        assert!(!l.users.contains_key(""));
        assert!(!l.users.contains_key("not-a-number"));
        assert!(!l.defs.iter().any(|d| d.title == "Placeholder"), "badges nobody wears are not kept");
        assert!(l.defs.iter().all(|d| d.id != "homies:u0"));
    }

    #[test]
    fn lists_may_be_missing_or_empty() {
        assert_eq!(combine(None, None, None), Loaded::default());
        let empty: Team = serde_json::from_str("{}").unwrap();
        assert_eq!(combine(None, Some(empty), None), Loaded::default());
        assert!(serde_json::from_str::<Made>("not json").is_err());
        assert!(serde_json::from_str::<Team>("3").is_err());
    }
}
