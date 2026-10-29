//! V108-N06 DankChat: `GET flxrs.com/api/badges` is a list of badge types, each with one
//! picture and the Twitch ids that have it.

use std::collections::HashSet;

use serde::Deserialize;

use super::{BadgeSet, Loaded, def, picture, slug, title};

pub const URL: &str = "https://flxrs.com/api/badges";
/// The official app hides this one too.
const HIDDEN: &str = "DankChat Top Supporter";

#[derive(Deserialize)]
#[serde(transparent)]
pub struct Raw(Vec<Kind>);

#[derive(Deserialize)]
struct Kind {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    users: Vec<String>,
}

impl From<Raw> for Loaded {
    fn from(raw: Raw) -> Self {
        let mut out = Loaded::default();
        let mut taken = HashSet::new();
        for k in raw.0 {
            let t = title(&k.kind);
            if t.is_empty() || k.kind == HIDDEN || !picture(&k.url) {
                continue;
            }
            let mut name = slug(&k.kind);
            if name.is_empty() || !taken.insert(name.clone()) {
                // Two types that read alike: told apart by their position.
                name = format!("{name}-{}", taken.len());
                taken.insert(name.clone());
            }
            let badge = def(BadgeSet::DankChat, &name, &t, [&k.url, &k.url, &k.url]);
            for u in &k.users {
                out.give(u, &badge.id);
            }
            out.defs.push(badge);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture;
    use super::*;

    #[test]
    fn types_become_badges() {
        let l: Loaded = fixture::<Raw>("dankchat.json").into();
        assert_eq!(l.defs.len(), 3, "the hidden type and the one without a picture are skipped");
        let sup = l.defs.iter().find(|d| d.id == "dankchat:dankchat-supporter").unwrap();
        assert_eq!(sup.title, "DankChat Supporter");
        assert_eq!(sup.image2x, "https://api.dank.chat/badges/supporter.png");
        assert_eq!(l.users["97120011"], ["dankchat:borgir-connoisseur"]);
        assert_eq!(l.users["62882992"], ["dankchat:duckerz", "dankchat:dankchat-supporter"]);
        assert!(!l.users.contains_key("7"), "only the hidden type");
        assert!(!l.users.contains_key(""));
    }

    #[test]
    fn types_that_read_alike_stay_apart() {
        let raw: Raw = serde_json::from_str(
            r#"[{"type":"A b","url":"https://x/1.png","users":["1"]},{"type":"a-B","url":"https://x/2.png","users":["2"]}]"#,
        )
        .unwrap();
        let l: Loaded = raw.into();
        assert_eq!(l.defs.len(), 2);
        assert_ne!(l.defs[0].id, l.defs[1].id);
    }

    #[test]
    fn empty_and_malformed() {
        assert_eq!(Loaded::from(serde_json::from_str::<Raw>("[]").unwrap()), Loaded::default());
        assert!(serde_json::from_str::<Raw>("{}").is_err());
        assert!(serde_json::from_str::<Raw>("[{\"type\":1}]").is_err());
    }
}
