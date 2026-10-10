//! Who wears what (1.37-1.39): provider badges and 7TV paints by Twitch user id. BTTV
//! name effects aren't shown (D22). The UI looks people up when it draws their name, so cosmetics that
//! arrive after a message still show on it.

use std::collections::HashMap;

use cyex_chat::model::Provider;
use serde::Serialize;
use ts_rs::TS;

use crate::seventv::paint::PaintStyle;

/// A provider badge's pictures.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeDef {
    pub provider: Provider,
    pub id: String,
    pub title: String,
    pub image1x: String,
    pub image2x: String,
    pub image4x: String,
    /// E2 Still frame of a moving badge.
    pub still: Option<String>,
    /// FFZ: color behind the picture.
    pub color: Option<String>,
    /// FFZ: the Twitch badge set it stands in for, e.g. `moderator`.
    pub replaces: Option<String>,
}

impl BadgeDef {
    pub fn key(&self) -> BadgeRef {
        BadgeRef {
            provider: self.provider,
            id: self.id.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeRef {
    pub provider: Provider,
    pub id: String,
}

/// One user's cosmetics, as the UI draws them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Look {
    /// 7TV paint id.
    pub paint: Option<String>,
    pub badges: Vec<BadgeRef>,
}

impl Look {
    pub fn is_empty(&self) -> bool {
        self.paint.is_none() && self.badges.is_empty()
    }
}

/// Looks for some users plus the paints and badges they mention.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Looks {
    /// User id → look; `null` for users with nothing.
    pub users: HashMap<String, Option<Look>>,
    pub paints: Vec<PaintStyle>,
    pub badges: Vec<BadgeDef>,
}

#[derive(Default)]
struct SevenUser {
    paint: Option<String>,
    badge: Option<String>,
}

#[derive(Default)]
pub struct Cosmetics {
    paints: HashMap<String, PaintStyle>,
    badges: HashMap<BadgeRef, BadgeDef>,
    seventv: HashMap<String, SevenUser>,
    /// FFZ badge ids with their users' Twitch ids, sorted (FFZ's global list is long).
    ffz: Vec<(String, Vec<u64>)>,
    /// BTTV badge by user.
    bttv: HashMap<String, String>,
}

/// 7TV paint or badge, given or taken.
pub enum SevenChange<'a> {
    Paint(Option<&'a str>),
    Badge(Option<&'a str>),
}

impl Cosmetics {
    pub fn add_paint(&mut self, p: PaintStyle) {
        self.paints.insert(p.id.clone(), p);
    }

    pub fn add_badge(&mut self, b: BadgeDef) {
        self.badges.insert(b.key(), b);
    }

    /// Returns true if the user's look changed.
    pub fn set_seventv(&mut self, user: &str, change: SevenChange<'_>) -> bool {
        let u = self.seventv.entry(user.to_owned()).or_default();
        let (slot, value) = match change {
            SevenChange::Paint(v) => (&mut u.paint, v),
            SevenChange::Badge(v) => (&mut u.badge, v),
        };
        if slot.as_deref() == value {
            return false;
        }
        *slot = value.map(str::to_owned);
        true
    }

    /// Takes a 7TV paint or badge off a user, if it's that one.
    pub fn take_seventv(&mut self, user: &str, change: SevenChange<'_>) -> bool {
        let Some(u) = self.seventv.get_mut(user) else {
            return false;
        };
        let (slot, id) = match change {
            SevenChange::Paint(v) => (&mut u.paint, v),
            SevenChange::Badge(v) => (&mut u.badge, v),
        };
        if slot.is_some() && slot.as_deref() == id {
            *slot = None;
            return true;
        }
        false
    }

    pub fn set_ffz_users(&mut self, users: Vec<(String, Vec<u64>)>) {
        self.ffz = users;
    }

    pub fn set_bttv_badge(&mut self, user: &str, badge: Option<&str>) -> bool {
        set(&mut self.bttv, user, badge)
    }

    pub fn look(&self, user: &str) -> Option<Look> {
        let mut look = Look::default();
        if let Some(u) = self.seventv.get(user) {
            look.paint = u.paint.clone().filter(|p| self.paints.contains_key(p));
            if let Some(b) = &u.badge {
                look.badges.push(BadgeRef {
                    provider: Provider::Seventv,
                    id: b.clone(),
                });
            }
        }
        if let Some(b) = self.bttv.get(user) {
            look.badges.push(BadgeRef {
                provider: Provider::Bttv,
                id: b.clone(),
            });
        }
        if let Ok(id) = user.parse::<u64>() {
            for (b, ids) in &self.ffz {
                if ids.binary_search(&id).is_ok() {
                    look.badges.push(BadgeRef {
                        provider: Provider::Ffz,
                        id: b.clone(),
                    });
                }
            }
        }
        look.badges.retain(|b| self.badges.contains_key(b));
        (!look.is_empty()).then_some(look)
    }

    /// Looks for these users, with the paints and badges they use.
    pub fn looks<'a>(&self, users: impl IntoIterator<Item = &'a str>) -> Looks {
        let mut out = Looks::default();
        let mut paints = Vec::new();
        let mut badges = Vec::new();
        for id in users {
            let look = self.look(id);
            if let Some(l) = &look {
                paints.extend(l.paint.clone());
                badges.extend(l.badges.iter().cloned());
            }
            out.users.insert(id.to_owned(), look);
        }
        paints.sort();
        paints.dedup();
        badges.sort_by(|a, b| (a.provider as u8, &a.id).cmp(&(b.provider as u8, &b.id)));
        badges.dedup();
        out.paints = paints
            .iter()
            .filter_map(|p| self.paints.get(p).cloned())
            .collect();
        out.badges = badges
            .iter()
            .filter_map(|b| self.badges.get(b).cloned())
            .collect();
        out
    }
}

fn set(map: &mut HashMap<String, String>, user: &str, value: Option<&str>) -> bool {
    match value {
        Some(v) if map.get(user).map(String::as_str) != Some(v) => {
            map.insert(user.to_owned(), v.to_owned());
            true
        }
        None => map.remove(user).is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn badge(provider: Provider, id: &str) -> BadgeDef {
        BadgeDef {
            provider,
            id: id.into(),
            title: id.into(),
            image1x: String::new(),
            image2x: String::new(),
            image4x: String::new(),
            still: None,
            color: None,
            replaces: None,
        }
    }

    #[test]
    fn looks_combine_providers_and_skip_unknown_badges() {
        let mut c = Cosmetics::default();
        c.add_badge(badge(Provider::Seventv, "s"));
        c.add_badge(badge(Provider::Ffz, "2"));
        assert!(c.set_seventv("1", SevenChange::Badge(Some("s"))));
        assert!(!c.set_seventv("1", SevenChange::Badge(Some("s"))));
        c.set_seventv("1", SevenChange::Paint(Some("missing")));
        c.set_ffz_users(vec![("2".into(), vec![1, 5]), ("3".into(), vec![1])]);
        let look = c.look("1").unwrap();
        assert_eq!(look.badges.len(), 2);
        assert!(look.paint.is_none(), "paint not known yet");
        assert!(c.look("9").is_none());
        let looks = c.looks(["1", "9"]);
        assert_eq!(looks.badges.len(), 2);
        assert_eq!(looks.users["9"], None);
    }

    #[test]
    fn taking_only_removes_the_same_one() {
        let mut c = Cosmetics::default();
        c.set_seventv("1", SevenChange::Paint(Some("a")));
        assert!(!c.take_seventv("1", SevenChange::Paint(Some("b"))));
        assert!(c.take_seventv("1", SevenChange::Paint(Some("a"))));
    }
}
