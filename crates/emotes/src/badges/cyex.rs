//! V109-B02 cyex badges (open read API, `docs/badges/spec.md`): founder (numbered), supporter
//! and the Supporter+ owner's own picture. One snapshot of everyone (`/badges/v1/users`), asked
//! at start and every 30 minutes with `If-None-Match`; nothing is asked per message.
//!
//! At most two show per person: founder or supporter, then the Supporter+ badge ([`shown`]).

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
    /// Founders only.
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

/// `GET /badges/v1/badges`: titles and the built-in pictures.
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

/// The equip rule: founder or supporter (founder first when both come), then Supporter+.
/// Anything else is skipped.
pub fn shown(list: &[Held]) -> Vec<&Held> {
    let main = list
        .iter()
        .find(|h| h.id == FOUNDER)
        .or_else(|| list.iter().find(|h| h.id == SUPPORTER));
    let custom = list.iter().find(|h| h.id == CUSTOM && h.images.is_some());
    main.into_iter().chain(custom).collect()
}

/// The tooltip: `cyex founder #37`, `cyex supporter`, `cyex supporter+`.
pub fn title(kind: &str, number: Option<u32>, named: Option<&str>) -> String {
    let base = named.filter(|t| !t.trim().is_empty()).map_or_else(
        || match kind {
            FOUNDER => "cyex founder".to_owned(),
            SUPPORTER => "cyex supporter".to_owned(),
            _ => "cyex supporter+".to_owned(),
        },
        super::title,
    );
    match (kind, number) {
        (FOUNDER, Some(n)) => format!("{base} #{n}"),
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

/// The pictures of `founder` or `supporter`.
pub fn pictures(kinds: Option<&Kinds>, base: &str, kind: &str) -> Images {
    built_in(kinds, base, kind).0
}

/// The built-in badges' pictures: from `/badges`, else the documented paths.
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

/// The badge id a held badge draws as (`cyex:founder-37`, `cyex:supporter`, `cyex:u123`).
pub fn badge_id(user: &str, h: &Held) -> String {
    let p = BadgeSet::Cyex.prefix();
    match (h.id.as_str(), h.number) {
        (FOUNDER, Some(n)) => format!("{p}:founder-{n}"),
        (CUSTOM, _) => format!("{p}:u{user}"),
        (kind, _) => format!("{p}:{kind}"),
    }
}

/// Everyone's badges as a set for [`crate::Cosmetics`], with the equip rule applied.
pub fn build<'a>(kinds: Option<&Kinds>, base: &str, users: impl IntoIterator<Item = (&'a String, &'a Vec<Held>)>) -> Loaded {
    let mut out = Loaded::default();
    let founder = built_in(kinds, base, FOUNDER);
    let supporter = built_in(kinds, base, SUPPORTER);
    let custom_title = kinds.and_then(|k| k.badges.iter().find(|b| b.id == CUSTOM)).map(|k| k.title.clone());
    let mut made = std::collections::HashSet::new();
    for (user, list) in users {
        if !user_id(user) {
            continue;
        }
        for h in shown(list) {
            let (images, named) = match h.id.as_str() {
                FOUNDER => (&founder.0, founder.1.as_deref()),
                SUPPORTER => (&supporter.0, supporter.1.as_deref()),
                _ => match &h.images {
                    Some(i) => (i, custom_title.as_deref()),
                    None => continue,
                },
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
        assert_eq!(ids("5"), ["supporter", "custom"], "unknown kinds are skipped");
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
    }

    #[test]
    fn etags_are_kept_and_sent_back() {
        let mut t = Tagged::default();
        assert_eq!(t.header(), None);
        let first = t.read(reply(200, Some("\"abc\""), SNAP)).unwrap().unwrap();
        assert_eq!(first.users.len(), 5);
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
