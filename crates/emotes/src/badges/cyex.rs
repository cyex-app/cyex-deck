//! V109-B02 cyex badges (open read API, `docs/badges/spec.md`): founder (numbered), supporter
//! and the Supporter+ owner's own picture. One snapshot of everyone (`/badges/v1/users`), asked
//! at start and every 30 minutes with `If-None-Match`; nothing is asked per message.
//!
//! At most two show per person: one one-time badge, then the Supporter+ badge ([`shown`]).
//! V109-B10 One-time badges are any store item: kinds, titles and pictures come from the
//! catalog (`/badges/v1/badges`); a numbered one shows "#n" whatever its kind.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use cyex_chat::model::Provider;
use cyex_twitch::{Call, Error, Http, Reply, Result, Verb};

use super::{BadgeSet, Loaded, picture, user_id};
use crate::cosmetics::BadgeDef;

pub const BASE: &str = "https://api.cyex.app";
pub const USERS: &str = "/badges/v1/users";
pub const KINDS: &str = "/badges/v1/badges";
/// The spec asks for every 30 minutes (never under 5).
pub const REFRESH: Duration = Duration::from_secs(30 * 60);
/// The snapshot of everyone; far more than 100.000 people fit.
const MAX_BODY: usize = 16 << 20;

pub const FOUNDER: &str = "founder";
pub const SUPPORTER: &str = "supporter";
pub const CUSTOM: &str = "custom";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Images {
    #[serde(rename = "1x", default)]
    pub x1: String,
    #[serde(rename = "2x", default)]
    pub x2: String,
    #[serde(rename = "4x", default)]
    pub x4: String,
}

/// One badge a person holds, as the snapshot has it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub id: String,
    /// Numbered badges (founder, and any limited store item).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    /// Supporter+ only: the owner's own picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Images>,
}

/// `GET /badges/v1/users`. Unknown fields are ignored (spec: new ones may appear).
#[derive(Debug, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub users: HashMap<String, Vec<Held>>,
}

/// `GET /badges/v1/badges`: every one-time badge in store order (inactive ones too), then
/// `custom`.
#[derive(Debug, Default, Deserialize)]
pub struct Kinds {
    #[serde(default)]
    badges: Vec<Kind>,
}

#[derive(Debug, Deserialize)]
struct Kind {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    images: Option<Images>,
}

/// V109-B10 One one-time badge of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OneTime {
    pub id: String,
    pub title: String,
    pub images: Images,
}

impl Kinds {
    pub fn knows(&self, id: &str) -> bool {
        self.badges.iter().any(|k| k.id == id)
    }

    /// The one-time badges in store order, each with its pictures (or the documented path).
    pub fn one_time(&self, base: &str) -> Vec<OneTime> {
        self.badges
            .iter()
            .filter(|k| k.id != CUSTOM && kind_ok(&k.id))
            .map(|k| {
                let (images, title) = built_in(Some(self), base, &k.id);
                OneTime {
                    id: k.id.clone(),
                    title: title.filter(|t| !t.trim().is_empty()).map_or_else(|| fallback_title(&k.id), |t| super::title(&t)),
                    images,
                }
            })
            .collect()
    }
}

/// V109-B10 A kind id the app takes: lowercase letters, digits and dashes, like the store's.
pub fn kind_ok(id: &str) -> bool {
    let b = id.as_bytes();
    (1..=40).contains(&b.len()) && b[0].is_ascii_lowercase() && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// V109-B10 Held ids the catalog doesn't name yet (`custom` and junk left out).
pub fn unknown<'a>(kinds: Option<&Kinds>, held: impl IntoIterator<Item = &'a Held>) -> Vec<String> {
    let mut out: Vec<String> = held
        .into_iter()
        .filter(|h| h.id != CUSTOM && kind_ok(&h.id) && !kinds.is_some_and(|k| k.knows(&h.id)))
        .map(|h| h.id.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The equip rule: one one-time badge (founder first, then supporter, then the first other
/// one), then Supporter+. Junk ids are skipped.
pub fn shown(list: &[Held]) -> Vec<&Held> {
    let main = list
        .iter()
        .find(|h| h.id == FOUNDER)
        .or_else(|| list.iter().find(|h| h.id == SUPPORTER))
        .or_else(|| list.iter().find(|h| h.id != CUSTOM && kind_ok(&h.id)));
    let custom = list.iter().find(|h| h.id == CUSTOM && h.images.is_some());
    main.into_iter().chain(custom).collect()
}

/// A kind the catalog hasn't named: `streak-1y` → `cyex streak 1y`.
fn fallback_title(kind: &str) -> String {
    match kind {
        FOUNDER => "cyex founder".to_owned(),
        SUPPORTER => "cyex supporter".to_owned(),
        CUSTOM => "cyex supporter+".to_owned(),
        k => format!("cyex {}", k.replace('-', " ")),
    }
}

/// The tooltip: `cyex founder #37`, `cyex supporter`, `cyex supporter+`; any numbered
/// one-time badge gets its number.
pub fn title(kind: &str, number: Option<u32>, named: Option<&str>) -> String {
    let base = named
        .filter(|t| !t.trim().is_empty())
        .map_or_else(|| fallback_title(kind), super::title);
    match number {
        Some(n) if kind != CUSTOM => format!("{base} #{n}"),
        _ => base,
    }
}

/// Keeps the ETag between asks: unchanged (`304`) answers keep what's loaded.
#[derive(Debug, Default)]
pub struct Tagged {
    etag: Option<String>,
}

impl Tagged {
    pub fn header(&self) -> Option<&str> {
        self.etag.as_deref()
    }

    /// Forgets the tag, so the next ask loads the whole list (the switch went off and on).
    pub fn reset(&mut self) {
        self.etag = None;
    }

    /// `Ok(None)`: unchanged. The tag is only kept from an answer that parsed.
    pub fn read(&mut self, reply: Reply) -> Result<Option<Snapshot>> {
        match reply.status {
            304 if self.etag.is_some() => Ok(None),
            200..=299 => {
                let snap: Snapshot = serde_json::from_slice(&reply.body).map_err(Error::Decode)?;
                self.etag = reply.etag.filter(|t| t.len() <= 200 && t.is_ascii());
                Ok(Some(snap))
            }
            status => Err(Error::Api {
                status,
                message: "cyex badges".into(),
            }),
        }
    }
}

/// Asks for the snapshot; `Ok(None)` when it hasn't changed since the last one.
pub async fn fetch_users(http: &Http, base: &str, tag: &mut Tagged) -> Result<Option<Snapshot>> {
    let call = Call {
        if_none_match: tag.header(),
        max: MAX_BODY,
        ..Call::default()
    };
    let reply = http.call(Verb::Get, &format!("{base}{USERS}"), call).await?;
    tag.read(reply)
}

pub async fn fetch_kinds(http: &Http, base: &str) -> Result<Kinds> {
    let call = Call {
        max: 1 << 20,
        ..Call::default()
    };
    let reply = http.call(Verb::Get, &format!("{base}{KINDS}"), call).await?;
    if !(200..300).contains(&reply.status) {
        return Err(Error::Api {
            status: reply.status,
            message: "cyex badge kinds".into(),
        });
    }
    serde_json::from_slice(&reply.body).map_err(Error::Decode)
}

/// Pictures over https; test builds also take small `data:image/` pictures from the mock.
pub fn picture_ok(url: &str) -> bool {
    if picture(url) {
        return true;
    }
    cfg!(feature = "mocks")
        && url.starts_with("data:image/")
        && url.len() <= 8192
        && !url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\"'<>\\`".contains(c))
}

fn sizes(i: &Images) -> Option<[&str; 3]> {
    if !picture_ok(&i.x1) {
        return None;
    }
    let x2 = if picture_ok(&i.x2) { &i.x2 } else { &i.x1 };
    let x4 = if picture_ok(&i.x4) { &i.x4 } else { x2 };
    Some([&i.x1, x2, x4])
}

/// The pictures of a one-time badge.
pub fn pictures(kinds: Option<&Kinds>, base: &str, kind: &str) -> Images {
    built_in(kinds, base, kind).0
}

/// A one-time badge's pictures: from `/badges`, else the documented paths.
fn built_in(kinds: Option<&Kinds>, base: &str, kind: &str) -> (Images, Option<String>) {
    let k = kinds.and_then(|k| k.badges.iter().find(|b| b.id == kind));
    let images = k.and_then(|k| k.images.clone()).unwrap_or_else(|| {
        let at = |s: &str| format!("{base}/badges/v1/img/{kind}/{s}.png");
        Images {
            x1: at("1x"),
            x2: at("2x"),
            x4: at("4x"),
        }
    });
    (images, k.map(|k| k.title.clone()))
}

fn def(id: String, title: String, sizes: [&str; 3]) -> BadgeDef {
    BadgeDef {
        provider: Provider::Twitch,
        id,
        title,
        image1x: sizes[0].to_owned(),
        image2x: sizes[1].to_owned(),
        image4x: sizes[2].to_owned(),
        still: None,
        color: None,
        replaces: None,
    }
}

/// The badge id a held badge draws as (`cyex:founder-37`, `cyex:streak-1y-4`, `cyex:supporter`,
/// `cyex:u123`).
pub fn badge_id(user: &str, h: &Held) -> String {
    let p = BadgeSet::Cyex.prefix();
    match (h.id.as_str(), h.number) {
        (CUSTOM, _) => format!("{p}:u{user}"),
        (kind, Some(n)) => format!("{p}:{kind}-{n}"),
        (kind, None) => format!("{p}:{kind}"),
    }
}

/// Everyone's badges as a set for [`crate::Cosmetics`], with the equip rule applied.
pub fn build<'a>(kinds: Option<&Kinds>, base: &str, users: impl IntoIterator<Item = (&'a String, &'a Vec<Held>)>) -> Loaded {
    let mut out = Loaded::default();
    // One look-up per kind, however many people hold it.
    let mut one_time: HashMap<String, (Images, Option<String>)> = HashMap::new();
    let custom_title = kinds.and_then(|k| k.badges.iter().find(|b| b.id == CUSTOM)).map(|k| k.title.clone());
    let mut made = std::collections::HashSet::new();
    for (user, list) in users {
        if !user_id(user) {
            continue;
        }
        for h in shown(list) {
            let (images, named) = if h.id == CUSTOM {
                match &h.images {
                    Some(i) => (i, custom_title.as_deref()),
                    None => continue,
                }
            } else {
                let k = one_time.entry(h.id.clone()).or_insert_with(|| built_in(kinds, base, &h.id));
                (&k.0, k.1.as_deref())
            };
            let Some(sizes) = sizes(images) else { continue };
            let id = badge_id(user, h);
            if made.insert(id.clone()) {
                out.defs.push(def(id.clone(), title(&h.id, h.number, named), sizes));
            }
            out.give(user, &id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNAP: &str = r#"{
      "updated": "2026-11-05T12:00:00Z", "extra": 1,
      "users": {
        "71092938": [ { "id": "founder", "number": 37 }, { "id": "supporter" } ],
        "1294600497": [ { "id": "custom", "images": { "1x": "https://api.cyex.app/u/1.png", "2x": "https://api.cyex.app/u/2.png", "4x": "https://api.cyex.app/u/4.png" } } ],
        "5": [ { "id": "supporter" }, { "id": "custom", "images": { "1x": "https://a.b/1.png" } }, { "id": "future" } ],
        "7": [ { "id": "streak-1y", "number": 4 } ],
        "8": [ { "id": "Bad Id" } ],
        "not-a-number": [ { "id": "supporter" } ],
        "6": [ { "id": "custom" } ]
      }
    }"#;

    fn reply(status: u16, etag: Option<&str>, body: &str) -> Reply {
        Reply {
            status,
            etag: etag.map(str::to_owned),
            body: body.as_bytes().to_vec(),
        }
    }

    fn snap() -> Snapshot {
        serde_json::from_str(SNAP).unwrap()
    }

    #[test]
    fn the_snapshot_parses_and_the_rule_keeps_two() {
        let s = snap();
        let ids = |u: &str| shown(&s.users[u]).iter().map(|h| h.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("71092938"), ["founder"], "founder wins over supporter");
        assert_eq!(ids("1294600497"), ["custom"]);
        assert_eq!(ids("5"), ["supporter", "custom"], "one one-time badge: supporter before others");
        assert_eq!(ids("7"), ["streak-1y"], "V109-B10 any store kind shows");
        assert!(ids("8").is_empty(), "junk ids are skipped");
        assert!(ids("6").is_empty(), "a custom badge without pictures is skipped");
    }

    #[test]
    fn the_rule_takes_one_main_and_supporter_plus_in_order() {
        let h = |id: &str| Held {
            id: id.into(),
            number: None,
            images: (id == CUSTOM).then(Images::default),
        };
        let list = [h(CUSTOM), h(SUPPORTER), h(FOUNDER), h(SUPPORTER)];
        let got: Vec<&str> = shown(&list).iter().map(|x| x.id.as_str()).collect();
        assert_eq!(got, [FOUNDER, CUSTOM]);
        assert!(shown(&[]).is_empty());
    }

    #[test]
    fn badges_carry_titles_and_pictures() {
        let s = snap();
        let l = build(None, BASE, &s.users);
        assert_eq!(l.users["71092938"], ["cyex:founder-37"]);
        assert_eq!(l.users["5"], ["cyex:supporter", "cyex:u5"]);
        assert!(!l.users.contains_key("not-a-number") && !l.users.contains_key("6"));
        let f = l.defs.iter().find(|d| d.id == "cyex:founder-37").unwrap();
        assert_eq!(f.title, "cyex founder #37");
        assert_eq!(f.image2x, "https://api.cyex.app/badges/v1/img/founder/2x.png");
        let c = l.defs.iter().find(|d| d.id == "cyex:u5").unwrap();
        assert_eq!(c.title, "cyex supporter+");
        assert_eq!((c.image2x.as_str(), c.image4x.as_str()), ("https://a.b/1.png", "https://a.b/1.png"));
        assert_eq!(l.defs.iter().filter(|d| d.id == "cyex:supporter").count(), 1, "one def per kind");
        let st = l.defs.iter().find(|d| d.id == "cyex:streak-1y-4").unwrap();
        assert_eq!(st.title, "cyex streak 1y #4", "not in the catalog yet: a readable name");
        assert_eq!(st.image1x, "https://api.cyex.app/badges/v1/img/streak-1y/1x.png");
    }

    #[test]
    fn the_kinds_list_names_and_pictures_built_in_badges() {
        let kinds: Kinds = serde_json::from_str(
            r#"{"badges":[{"id":"founder","title":"cyex founder","images":{"1x":"https://x/f1.png?v=1","2x":"https://x/f2.png?v=1","4x":"https://x/f4.png?v=1"}},
                           {"id":"custom","title":"cyex supporter+","images":null}]}"#,
        )
        .unwrap();
        let s = snap();
        let l = build(Some(&kinds), BASE, &s.users);
        let f = l.defs.iter().find(|d| d.id == "cyex:founder-37").unwrap();
        assert_eq!(f.image1x, "https://x/f1.png?v=1");
        assert_eq!(title(FOUNDER, Some(1), None), "cyex founder #1");
        assert_eq!(title(SUPPORTER, None, Some("  ")), "cyex supporter");
        assert_eq!(title(CUSTOM, Some(3), None), "cyex supporter+");
        assert_eq!(title("gold", Some(2), Some("cyex gold")), "cyex gold #2");
    }

    #[test]
    fn the_catalog_lists_one_time_badges_in_store_order() {
        let kinds: Kinds = serde_json::from_str(
            r#"{"badges":[{"id":"founder","title":"cyex founder","images":null},
                           {"id":"supporter","title":"cyex supporter","images":null},
                           {"id":"streak-1y","title":"cyex 1 year","images":{"1x":"https://x/s1.png","2x":"https://x/s2.png","4x":"https://x/s4.png"}},
                           {"id":"Bad","title":"x","images":null},
                           {"id":"custom","title":"cyex supporter+","images":null}]}"#,
        )
        .unwrap();
        let list = kinds.one_time(BASE);
        assert_eq!(list.iter().map(|k| k.id.as_str()).collect::<Vec<_>>(), ["founder", "supporter", "streak-1y"]);
        assert_eq!(list[2].title, "cyex 1 year");
        assert_eq!(list[2].images.x2, "https://x/s2.png");
        assert_eq!(list[0].images.x1, "https://api.cyex.app/badges/v1/img/founder/1x.png");
        let s = snap();
        let held = s.users.values().flatten();
        assert_eq!(unknown(Some(&kinds), held), ["future"], "the catalog doesn't name it yet");
        assert_eq!(unknown(None, s.users["71092938"].iter()), ["founder", "supporter"]);
        assert!(kind_ok("streak-1y") && !kind_ok("Bad") && !kind_ok("") && !kind_ok("1x") && !kind_ok("a/b"));
    }

    #[test]
    fn etags_are_kept_and_sent_back() {
        let mut t = Tagged::default();
        assert_eq!(t.header(), None);
        let first = t.read(reply(200, Some("\"abc\""), SNAP)).unwrap().unwrap();
        assert_eq!(first.users.len(), 7);
        assert_eq!(t.header(), Some("\"abc\""));
        assert!(t.read(reply(304, None, "")).unwrap().is_none(), "unchanged");
        assert_eq!(t.header(), Some("\"abc\""), "a 304 keeps the tag");
        t.reset();
        assert!(t.read(reply(304, None, "")).is_err(), "a 304 with nothing loaded is an error");
    }

    #[test]
    fn bad_answers_keep_the_old_tag() {
        let mut t = Tagged::default();
        t.read(reply(200, Some("\"1\""), r#"{"users":{}}"#)).unwrap();
        assert!(t.read(reply(200, Some("\"2\""), "not json")).is_err());
        assert_eq!(t.header(), Some("\"1\""));
        let e = t.read(reply(429, None, "")).unwrap_err();
        assert!(e.is_rate_limited());
        assert!(t.read(reply(503, None, "")).unwrap_err().is_server_error());
        assert_eq!(t.header(), Some("\"1\""));
        let empty = t.read(reply(200, None, "{}")).unwrap().unwrap();
        assert!(empty.users.is_empty());
        assert_eq!(t.header(), None, "an answer without a tag asks in full next time");
    }

    #[test]
    fn junk_pictures_are_refused() {
        assert!(picture_ok("https://api.cyex.app/x.png"));
        assert!(!picture_ok("http://api.cyex.app/x.png"));
        assert!(!picture_ok("https://a/\"onerror=1"));
        assert_eq!(picture_ok("data:image/png;base64,AAAA"), cfg!(feature = "mocks"));
    }
}
