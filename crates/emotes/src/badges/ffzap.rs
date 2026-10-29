//! V108-N06 FFZ:AP supporters: `GET api.ffzap.com/v1/supporters` lists Twitch ids; the badge
//! picture of each is `…/v1/user/badge/<id>/{1,2,3}` (a picture per person, some tinted).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use super::{BadgeSet, Loaded, def, picture, user_id};

pub const URL: &str = "https://api.ffzap.com/v1/supporters";
const PICTURES: &str = "https://api.ffzap.com/v1/user/badge";

/// The people behind FFZ:AP, as the project names them.
static STAFF: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    HashMap::from([
        ("26964566", "FFZ:AP Developer"),
        ("11819690", "FFZ:AP Helper"),
        ("36442149", "FFZ:AP Helper"),
        ("29519423", "FFZ:AP Helper"),
        ("22025290", "FFZ:AP Helper"),
        ("4867723", "FFZ:AP Helper"),
    ])
});

#[derive(Deserialize)]
#[serde(transparent)]
pub struct Raw(Vec<Supporter>);

#[derive(Deserialize)]
struct Supporter {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    badge_color: Option<String>,
    /// `0`/`1` today; a real boolean would do too.
    #[serde(default)]
    badge_is_colored: Option<serde_json::Value>,
}

impl Supporter {
    fn colored(&self) -> bool {
        match &self.badge_is_colored {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
            _ => false,
        }
    }
}

fn hex_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

impl From<Raw> for Loaded {
    fn from(raw: Raw) -> Self {
        let mut out = Loaded::default();
        for s in raw.0 {
            let Some(id) = s.id.as_deref().filter(|i| user_id(i)) else {
                continue;
            };
            let title = STAFF.get(id).copied().unwrap_or("FFZ:AP Supporter");
            let base = format!("{PICTURES}/{id}");
            let sizes = [format!("{base}/1"), format!("{base}/2"), format!("{base}/3")];
            if !sizes.iter().all(|u| picture(u)) {
                continue;
            }
            let mut badge = def(BadgeSet::FfzAp, id, title, [&sizes[0], &sizes[1], &sizes[2]]);
            if s.colored() {
                badge.color = s.badge_color.filter(|c| hex_color(c));
            }
            out.give(id, &badge.id);
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
    fn supporters_get_their_own_picture() {
        let l: Loaded = fixture::<Raw>("ffzap.json").into();
        assert_eq!(l.defs.len(), 4, "the entry without an id is skipped");
        let helper = l.defs.iter().find(|d| d.id == "ffzap:4867723").unwrap();
        assert_eq!(helper.title, "FFZ:AP Helper");
        assert_eq!(helper.image2x, "https://api.ffzap.com/v1/user/badge/4867723/2");
        assert_eq!(helper.color, None, "not colored: the tint is ignored");
        let tinted = l.defs.iter().find(|d| d.id == "ffzap:47965880").unwrap();
        assert_eq!(tinted.color.as_deref(), Some("#D319C3"));
        assert_eq!(tinted.title, "FFZ:AP Supporter");
        assert_eq!(l.users["7864791"], ["ffzap:7864791"]);
    }

    #[test]
    fn bad_colors_and_ids_are_dropped() {
        let raw: Raw = serde_json::from_str(
            r#"[{"id":"1","badge_is_colored":true,"badge_color":"red;x"},{"id":"../x"},{"id":"undefined"},{}]"#,
        )
        .unwrap();
        let l: Loaded = raw.into();
        assert_eq!(l.defs.len(), 1);
        assert_eq!(l.defs[0].color, None);
    }

    #[test]
    fn empty_and_malformed() {
        assert_eq!(Loaded::from(serde_json::from_str::<Raw>("[]").unwrap()), Loaded::default());
        assert!(serde_json::from_str::<Raw>("{}").is_err());
        assert!(serde_json::from_str::<Raw>("<html>").is_err());
    }
}
