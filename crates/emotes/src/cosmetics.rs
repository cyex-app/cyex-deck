//! Who wears what (1.37-1.39): provider badges and 7TV paints by Twitch user id. BTTV
//! name effects aren't shown (D22). The UI looks people up when it draws their name, so cosmetics that
//! arrive after a message still show on it.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cyex_chat::model::Provider;
use serde::Serialize;
use ts_rs::TS;

use crate::badges::{BadgeSet, Loaded};
use crate::capped::Capped;
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

/// V107-M3 7TV users kept at most. Events are push-only (no per-user lookup to refetch from), so a
/// time sweep would lose looks for good; the least recently touched go past this instead.
const MAX_SEVENTV_USERS: usize = 20_000;
/// V107-R1 BTTV badge holders kept at most, trimmed the same way (Pro badges come over the socket,
/// one per person seen in any joined channel).
const MAX_BTTV_USERS: usize = 20_000;
/// V108-C10 Pictures asked about at most; past it the list starts over (the app's prints file
/// answers repeats without a fetch).
const MAX_ART_ASKED: usize = 20_000;

/// V108-C10 Pictures already handed to the app for a print, and the new ones waiting.
#[derive(Default)]
struct ArtAsks {
    asked: HashSet<String>,
    waiting: Vec<String>,
}

#[derive(Default)]
pub struct Cosmetics {
    paints: HashMap<String, PaintStyle>,
    badges: HashMap<BadgeRef, BadgeDef>,
    /// Last event or draw first to stay (S7).
    seventv: Capped<String, SevenUser, MAX_SEVENTV_USERS>,
    /// FFZ badge ids with their users' Twitch ids, sorted (FFZ's global list is long).
    ffz: Vec<(String, Vec<u64>)>,
    /// BTTV badge by user; last set or draw first to stay (S7).
    bttv: Capped<String, String, MAX_BTTV_USERS>,
    /// V108-N06 Extra badge sets: user id → badge ids, one map per loaded set.
    extra: HashMap<BadgeSet, HashMap<String, Vec<String>>>,
    /// V108-C10 Picture URL → the URL with the same art that counts for it (only where they differ).
    art: HashMap<String, String>,
    /// V108-C10 Filled while drawing (`look` takes `&self`).
    art_asks: Mutex<ArtAsks>,
}

/// V108-C10 The picture two badges are compared by.
fn picture_of(d: &BadgeDef) -> &str {
    if d.image2x.is_empty() { &d.image4x } else { &d.image2x }
}

/// 7TV paint or badge, given or taken.
pub enum SevenChange<'a> {
    Paint(Option<&'a str>),
    Badge(Option<&'a str>),
}

impl Cosmetics {
    /// V108-BD True when the paint is new or changed.
    pub fn add_paint(&mut self, p: PaintStyle) -> bool {
        if self.paints.get(&p.id) == Some(&p) {
            return false;
        }
        self.paints.insert(p.id.clone(), p);
        true
    }

    /// V108-BD True when the badge is new or changed.
    pub fn add_badge(&mut self, b: BadgeDef) -> bool {
        let key = b.key();
        if self.badges.get(&key) == Some(&b) {
            return false;
        }
        self.badges.insert(key, b);
        true
    }

    /// V108-BD 7TV users who wear this paint or badge. A definition that arrives after its
    /// entitlement (or changes) sends their looks again; `look` skipped it until then.
    pub fn seventv_wearers(&self, change: SevenChange<'_>) -> Vec<String> {
        let wears = |u: &SevenUser| match change {
            SevenChange::Paint(id) => id.is_some() && u.paint.as_deref() == id,
            SevenChange::Badge(id) => id.is_some() && u.badge.as_deref() == id,
        };
        self.seventv.iter().filter(|(_, u)| wears(u)).map(|(k, _)| k.clone()).collect()
    }

    /// Returns true if the user's look changed.
    pub fn set_seventv(&mut self, user: &str, change: SevenChange<'_>) -> bool {
        let u = self.seventv.get_or_default(user.to_owned());
        let (slot, value) = match change {
            SevenChange::Paint(v) => (&mut u.paint, v),
            SevenChange::Badge(v) => (&mut u.badge, v),
        };
        let changed = slot.as_deref() != value;
        if changed {
            *slot = value.map(str::to_owned);
        }
        if u.paint.is_none() && u.badge.is_none() {
            self.seventv.remove(user);
        } else {
            self.seventv.trim();
        }
        changed
    }

    pub fn seventv_users(&self) -> usize {
        self.seventv.len()
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
        if slot.is_none() || slot.as_deref() != id {
            return false;
        }
        *slot = None;
        if u.paint.is_none() && u.badge.is_none() {
            self.seventv.remove(user);
        }
        true
    }

    pub fn set_ffz_users(&mut self, users: Vec<(String, Vec<u64>)>) {
        self.ffz = users;
    }

    pub fn set_bttv_badge(&mut self, user: &str, badge: Option<&str>) -> bool {
        let Some(badge) = badge else {
            return self.bttv.remove(user).is_some();
        };
        if let Some(u) = self.bttv.get_mut(user) {
            if u == badge {
                return false;
            }
            badge.clone_into(u);
            return true;
        }
        self.bttv.insert(user.to_owned(), badge.to_owned());
        self.bttv.trim();
        true
    }

    pub fn bttv_users(&self) -> usize {
        self.bttv.len()
    }

    /// V108-N06 Puts a set in, in place of its older list.
    pub fn set_extra(&mut self, set: BadgeSet, loaded: Loaded) {
        self.clear_extra(set);
        for d in loaded.defs {
            self.add_badge(d);
        }
        self.extra.insert(set, loaded.users);
    }

    /// V108-N06 Takes a set out (its switch went off) and frees its list.
    pub fn clear_extra(&mut self, set: BadgeSet) {
        self.extra.remove(&set);
        // V108-C10 Its pictures leave the same-art map too (the app keeps their prints on disk).
        let mut gone = HashSet::new();
        self.badges.retain(|k, d| {
            let keep = BadgeSet::of_id(&k.id) != Some(set);
            if !keep {
                gone.insert(picture_of(d).to_owned());
            }
            keep
        });
        if !gone.is_empty() {
            self.art.retain(|url, canon| !gone.contains(url) && !gone.contains(canon));
            let mut asks = self.art_asks.lock().unwrap_or_else(|p| p.into_inner());
            asks.asked.retain(|u| !gone.contains(u));
            asks.waiting.retain(|u| !gone.contains(u));
        }
    }

    /// V108-C10 The same-art map from the app's prints. True when it changed.
    pub fn set_art(&mut self, map: HashMap<String, String>) -> bool {
        let changed = self.art != map;
        self.art = map;
        changed
    }

    /// V108-C10 Pictures the app already has a print for: never asked again.
    pub fn art_known<'a>(&self, urls: impl IntoIterator<Item = &'a str>) {
        let mut asks = self.art_asks.lock().unwrap_or_else(|p| p.into_inner());
        asks.asked.extend(urls.into_iter().map(str::to_owned));
    }

    /// V108-C10 Pictures whose fetch failed: asked again the next time they meet on a line.
    pub fn art_forget<'a>(&self, urls: impl IntoIterator<Item = &'a str>) {
        let mut asks = self.art_asks.lock().unwrap_or_else(|p| p.into_inner());
        for u in urls {
            asks.asked.remove(u);
        }
    }

    /// V108-C10 Some picture waits for a print.
    pub fn art_waiting(&self) -> bool {
        !self.art_asks.lock().unwrap_or_else(|p| p.into_inner()).waiting.is_empty()
    }

    /// V108-C10 Pictures that met another on a line and have no print yet.
    pub fn take_art_asks(&self) -> Vec<String> {
        std::mem::take(&mut self.art_asks.lock().unwrap_or_else(|p| p.into_inner()).waiting)
    }

    /// V108-C10 Hands this look's pictures to the app for prints, once each. Only a look with two
    /// or more pictures can show the same art twice.
    fn ask_art(&self, pictures: &[&str]) {
        if pictures.len() < 2 {
            return;
        }
        let mut asks = self.art_asks.lock().unwrap_or_else(|p| p.into_inner());
        if asks.asked.len() >= MAX_ART_ASKED {
            asks.asked.clear();
        }
        for p in pictures {
            if !asks.asked.contains(*p) {
                asks.asked.insert((*p).to_owned());
                asks.waiting.push((*p).to_owned());
            }
        }
    }

    pub fn extra_loaded(&self, set: BadgeSet) -> bool {
        self.extra.contains_key(&set)
    }

    pub fn look(&self, user: &str) -> Option<Look> {
        let mut look = Look::default();
        // Drawn: counts as recent, so it outlives the trim.
        if let Some(u) = self.seventv.get(user) {
            look.paint = u.paint.clone().filter(|p| self.paints.contains_key(p));
            if let Some(b) = &u.badge {
                look.badges.push(BadgeRef {
                    provider: Provider::Seventv,
                    id: b.clone(),
                });
            }
        }
        if let Some(badge) = self.bttv.get(user) {
            look.badges.push(BadgeRef {
                provider: Provider::Bttv,
                id: badge.clone(),
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
        // V108-N06 Their badges carry no emote provider: Twitch stands for "none".
        for set in BadgeSet::ALL {
            for id in self.extra.get(&set).and_then(|u| u.get(user)).into_iter().flatten() {
                look.badges.push(BadgeRef {
                    provider: Provider::Twitch,
                    id: id.clone(),
                });
            }
        }
        look.badges.retain(|b| self.badges.contains_key(b));
        // V108-C10 One picture once: lists repeat a badge under two ids (Homies' developer
        // badge sits in both team lists), and the same art sits at other URLs (`art`: one map
        // lookup per badge). The first in provider order stays.
        let pictures: Vec<&str> = look
            .badges
            .iter()
            .map(|b| picture_of(&self.badges[b]))
            .filter(|p| !p.is_empty())
            .collect();
        self.ask_art(&pictures);
        let mut seen = HashSet::new();
        look.badges.retain(|b| {
            let pic = picture_of(&self.badges[b]);
            pic.is_empty() || seen.insert(self.art.get(pic).map_or(pic, String::as_str))
        });
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

    // V108-C10
    #[test]
    fn look_drops_a_badge_whose_picture_is_already_there() {
        let mut c = Cosmetics::default();
        let mut a = badge(Provider::Seventv, "a");
        a.image2x = "https://x/heart.png".into();
        let mut b = badge(Provider::Bttv, "b");
        b.image2x = "https://x/heart.png".into();
        let mut d = badge(Provider::Ffz, "2");
        d.image2x = "https://x/cat.png".into();
        for x in [a, b, d] {
            c.add_badge(x);
        }
        c.set_seventv("1", SevenChange::Badge(Some("a")));
        c.set_bttv_badge("1", Some("b"));
        c.set_ffz_users(vec![("2".into(), vec![1])]);
        let ids: Vec<_> = c.look("1").unwrap().badges.into_iter().map(|b| b.id).collect();
        assert_eq!(ids, ["a", "2"]);
    }

    // V108-C10 Same art at another URL: asked once, dropped once the map says so.
    #[test]
    fn look_asks_for_prints_and_drops_the_same_art() {
        let mut c = Cosmetics::default();
        let mut a = badge(Provider::Seventv, "a");
        a.image2x = "https://x/heart.png".into();
        let mut b = badge(Provider::Bttv, "b");
        b.image2x = "https://y/heart.webp".into();
        let mut d = badge(Provider::Ffz, "2");
        d.image2x = "https://x/cat.png".into();
        for x in [a, b, d] {
            c.add_badge(x);
        }
        c.set_bttv_badge("9", Some("b"));
        c.look("9");
        assert!(c.take_art_asks().is_empty(), "one badge can't repeat");
        c.set_bttv_badge("1", Some("b"));
        c.set_seventv("1", SevenChange::Badge(Some("a")));
        c.set_ffz_users(vec![("2".into(), vec![1])]);
        assert_eq!(c.look("1").unwrap().badges.len(), 3, "no prints yet: all show");
        assert_eq!(c.take_art_asks(), ["https://x/heart.png", "https://y/heart.webp", "https://x/cat.png"]);
        c.look("1");
        assert!(c.take_art_asks().is_empty(), "asked once");
        assert!(c.set_art(HashMap::from([("https://y/heart.webp".into(), "https://x/heart.png".into())])));
        let ids: Vec<_> = c.look("1").unwrap().badges.into_iter().map(|b| b.id).collect();
        assert_eq!(ids, ["a", "2"]);
        // A failed fetch is asked again.
        c.art_forget(["https://x/cat.png"]);
        c.look("1");
        assert_eq!(c.take_art_asks(), ["https://x/cat.png"]);
        // Known from the file: never asked.
        let mut e = badge(Provider::Ffz, "3");
        e.image2x = "https://z/known.png".into();
        c.add_badge(e);
        c.art_known(["https://z/known.png"]);
        c.set_ffz_users(vec![("2".into(), vec![1]), ("3".into(), vec![1])]);
        c.look("1");
        assert!(c.take_art_asks().is_empty());
    }

    // V108-C10 A set switched off takes its pictures out of the same-art map.
    #[test]
    fn clearing_a_set_drops_its_art() {
        use crate::badges::{BadgeSet, Loaded};
        let mut c = Cosmetics::default();
        let mut loaded = Loaded::default();
        let mut def = badge(Provider::Twitch, "homies:u1");
        def.image2x = "https://h/made.webp".into();
        loaded.defs.push(def);
        loaded.users.insert("1".into(), vec!["homies:u1".into()]);
        c.set_extra(BadgeSet::Homies, loaded);
        c.set_art(HashMap::from([
            ("https://h/made.webp".into(), "https://t/team.png".into()),
            ("https://o/other.png".into(), "https://t/team.png".into()),
        ]));
        c.clear_extra(BadgeSet::Homies);
        assert_eq!(c.art.len(), 1);
        assert!(c.art.contains_key("https://o/other.png"));
    }

    // V108-C10 Badges without a picture url aren't "the same picture".
    #[test]
    fn look_keeps_every_badge_with_an_empty_image() {
        let mut c = Cosmetics::default();
        c.add_badge(badge(Provider::Seventv, "a"));
        c.add_badge(badge(Provider::Bttv, "b"));
        c.add_badge(badge(Provider::Ffz, "2"));
        c.set_seventv("1", SevenChange::Badge(Some("a")));
        c.set_bttv_badge("1", Some("b"));
        c.set_ffz_users(vec![("2".into(), vec![1])]);
        let ids: Vec<_> = c.look("1").unwrap().badges.into_iter().map(|b| b.id).collect();
        assert_eq!(ids, ["a", "b", "2"]);
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
    fn extra_sets_come_and_go_whole() {
        use crate::badges::{BadgeSet, Loaded};
        let mut c = Cosmetics::default();
        let def = |id: &str| BadgeDef { id: id.into(), ..badge(Provider::Twitch, id) };
        let mut loaded = Loaded::default();
        loaded.defs.push(def("ffzap:1"));
        loaded.users.insert("1".into(), vec!["ffzap:1".into(), "ffzap:gone".into()]);
        c.set_extra(BadgeSet::FfzAp, loaded);
        assert!(c.extra_loaded(BadgeSet::FfzAp));
        let look = c.look("1").unwrap();
        assert_eq!(look.badges.len(), 1, "a badge without a picture is not worn");
        assert_eq!(look.badges[0].id, "ffzap:1");
        // A new list replaces the old one.
        let mut next = Loaded::default();
        next.defs.push(def("ffzap:2"));
        next.users.insert("2".into(), vec!["ffzap:2".into()]);
        c.set_extra(BadgeSet::FfzAp, next);
        assert!(c.look("1").is_none() && c.look("2").is_some());
        // Other sets and providers stay.
        c.add_badge(badge(Provider::Bttv, "pro"));
        c.clear_extra(BadgeSet::FfzAp);
        assert!(!c.extra_loaded(BadgeSet::FfzAp) && c.look("2").is_none());
        assert!(c.badges.contains_key(&BadgeRef { provider: Provider::Bttv, id: "pro".into() }));
        assert!(c.badges.len() == 1);
    }

    #[test]
    fn bttv_badges_change_and_go() {
        let mut c = Cosmetics::default();
        c.add_badge(badge(Provider::Bttv, "staff"));
        assert!(c.set_bttv_badge("1", Some("pro")));
        assert!(!c.set_bttv_badge("1", Some("pro")));
        assert!(c.set_bttv_badge("1", Some("staff")));
        assert_eq!(c.look("1").unwrap().badges[0].id, "staff");
        assert!(c.set_bttv_badge("1", None));
        assert!(!c.set_bttv_badge("1", None));
        assert_eq!(c.bttv_users(), 0);
    }

    #[test]
    fn bttv_trim_keeps_the_recent() {
        let mut c = Cosmetics::default();
        c.add_badge(badge(Provider::Bttv, "pro"));
        for u in 0..10 {
            c.set_bttv_badge(&u.to_string(), Some("pro"));
        }
        c.look("0");
        c.bttv.trim_to(3);
        assert_eq!(c.bttv_users(), 3);
        assert!(c.look("0").is_some(), "drawn last, so kept");
        assert!(c.look("9").is_some() && c.look("8").is_some());
        assert!(c.look("1").is_none());
    }

    #[test]
    fn bttv_holders_are_capped() {
        let mut c = Cosmetics::default();
        for u in 0..MAX_BTTV_USERS + MAX_BTTV_USERS / 10 + 1 {
            c.set_bttv_badge(&u.to_string(), Some("pro"));
        }
        assert_eq!(c.bttv_users(), MAX_BTTV_USERS);
    }

    #[test]
    fn taking_only_removes_the_same_one() {
        let mut c = Cosmetics::default();
        c.set_seventv("1", SevenChange::Paint(Some("a")));
        assert!(!c.take_seventv("1", SevenChange::Paint(Some("b"))));
        assert!(c.take_seventv("1", SevenChange::Paint(Some("a"))));
    }

    #[test]
    fn emptied_users_leave_the_map() {
        let mut c = Cosmetics::default();
        c.set_seventv("1", SevenChange::Paint(Some("a")));
        c.set_seventv("1", SevenChange::Badge(Some("b")));
        assert!(c.take_seventv("1", SevenChange::Paint(Some("a"))));
        assert_eq!(c.seventv_users(), 1);
        assert!(c.take_seventv("1", SevenChange::Badge(Some("b"))));
        assert_eq!(c.seventv_users(), 0);
        c.set_seventv("2", SevenChange::Paint(None));
        assert_eq!(c.seventv_users(), 0);
    }

    #[test]
    fn least_recent_users_go_past_the_cap() {
        let mut c = Cosmetics::default();
        for i in 0..30_000 {
            c.set_seventv(&i.to_string(), SevenChange::Paint(Some("a")));
        }
        let n = c.seventv_users();
        assert!((MAX_SEVENTV_USERS..=MAX_SEVENTV_USERS * 11 / 10).contains(&n));
        assert!(c.seventv.contains_key("29999"));
        assert!(!c.seventv.contains_key("0"));
    }

    // V108-BD A badge defined after it was given: its wearers are found, and their look has it.
    #[test]
    fn a_late_definition_finds_its_wearers() {
        let mut c = Cosmetics::default();
        c.set_seventv("1", SevenChange::Badge(Some("z")));
        c.set_seventv("2", SevenChange::Badge(Some("y")));
        c.set_seventv("3", SevenChange::Paint(Some("z")));
        assert!(c.look("1").unwrap_or_default().badges.is_empty());
        assert!(c.add_badge(badge(Provider::Seventv, "z")));
        assert!(!c.add_badge(badge(Provider::Seventv, "z")));
        assert_eq!(c.seventv_wearers(SevenChange::Badge(Some("z"))), ["1"]);
        assert!(c.seventv_wearers(SevenChange::Badge(None)).is_empty());
        assert_eq!(c.look("1").unwrap().badges.len(), 1);
        let mut moved = badge(Provider::Seventv, "z");
        moved.image2x = "https://x/new.png".into();
        assert!(c.add_badge(moved));
    }

    #[test]
    fn a_user_who_is_read_survives_the_trim() {
        let mut c = Cosmetics::default();
        c.set_seventv("0", SevenChange::Paint(Some("a")));
        for i in 1..30_000 {
            c.set_seventv(&i.to_string(), SevenChange::Paint(Some("a")));
            if i % 1000 == 0 {
                c.look("0");
            }
        }
        assert!(c.seventv.contains_key("0"));
        assert!(!c.seventv.contains_key("1"));
    }
}
