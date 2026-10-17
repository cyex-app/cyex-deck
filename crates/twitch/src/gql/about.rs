//! A channel's about page (1.21): bio, followers, last stream, social links and panels.
//! Public, checked 2026/10/09. Extension panels are iframes and can't be drawn here, so
//! they are left out (V106-23).

use std::collections::HashSet;

use cyex_core::markdown::{self, Block};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, nullable, query_partial};
use crate::{Http, Result};

const ABOUT: &str =
    "query($login: String!) { user(login: $login) { id login displayName description createdAt \
    bannerImageURL primaryColorHex profileImageURL(width: 150) followers { totalCount } \
    roles { isPartner isAffiliate } lastBroadcast { startedAt title game { displayName } } \
    channel { socialMedias { name title url } } \
    panels { id type ... on DefaultPanel { title description imageURL linkURL } } } }";

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct About {
    pub id: String,
    pub login: String,
    pub name: String,
    pub bio: String,
    /// RFC 3339.
    pub created_at: String,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    /// `#rrggbb`.
    pub color: Option<String>,
    #[ts(type = "number")]
    pub followers: u64,
    pub partner: bool,
    pub affiliate: bool,
    pub last_stream: Option<LastStream>,
    pub socials: Vec<Social>,
    pub panels: Vec<Panel>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LastStream {
    /// RFC 3339.
    pub started_at: String,
    pub title: String,
    pub game: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Social {
    /// Lowercase site, e.g. `youtube`.
    pub site: String,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Panel {
    pub title: Option<String>,
    pub image: Option<String>,
    /// Where the image leads.
    pub link: Option<String>,
    pub body: Vec<Block>,
}

impl Panel {
    fn shows(&self) -> bool {
        self.title.is_some() || self.image.is_some() || !self.body.is_empty()
    }
}

impl About {
    /// V106-24 Drops images that are fully see-through; a panel left with nothing goes.
    pub fn without_images(&mut self, blank: &HashSet<String>) {
        for p in &mut self.panels {
            if p.image.as_ref().is_some_and(|u| blank.contains(u)) {
                p.image = None;
            }
        }
        self.panels.retain(Panel::shows);
    }

    /// V106-24 Panel images worth checking (each once, in order).
    pub fn panel_images(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        self.panels
            .iter()
            .filter_map(|p| p.image.clone())
            .filter(|u| seen.insert(u.clone()))
            .collect()
    }
}

/// `None` if the channel doesn't exist.
pub async fn about(http: &Http, login: &str) -> Result<Option<About>> {
    // V106-21 A failing side field must not blank the page.
    let data: Data = query_partial(http, Auth::PUBLIC, ABOUT, json!({ "login": login }), has_user).await?;
    Ok(data.user.map(About::from))
}

#[derive(Deserialize)]
struct Data {
    user: Option<RawUser>,
}

/// V106-21 Partial data is kept only when the user itself came back.
fn has_user(d: &Data) -> bool {
    d.user.is_some()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    login: String,
    display_name: Option<String>,
    description: Option<String>,
    created_at: Option<String>,
    #[serde(rename = "bannerImageURL")]
    banner_image_url: Option<String>,
    primary_color_hex: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    followers: Option<RawCount>,
    roles: Option<RawRoles>,
    last_broadcast: Option<RawBroadcast>,
    channel: Option<RawChannel>,
    #[serde(default, deserialize_with = "nullable")]
    panels: Vec<Option<RawPanel>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCount {
    total_count: Option<u64>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRoles {
    is_partner: Option<bool>,
    is_affiliate: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBroadcast {
    started_at: Option<String>,
    title: Option<String>,
    game: Option<RawGame>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGame {
    display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    #[serde(default)]
    social_medias: Option<Vec<RawSocial>>,
}

#[derive(Deserialize)]
struct RawSocial {
    name: Option<String>,
    title: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPanel {
    #[serde(rename = "type")]
    kind: Option<String>,
    title: Option<String>,
    description: Option<String>,
    #[serde(rename = "imageURL")]
    image_url: Option<String>,
    #[serde(rename = "linkURL")]
    link_url: Option<String>,
}

/// Trimmed, `None` if empty.
fn some(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn web(s: Option<String>) -> Option<String> {
    some(s).filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

/// V106-21 The CSP blocks plain-http images; Twitch's CDNs answer both.
fn web_image(s: Option<String>) -> Option<String> {
    web(s).map(|u| match u.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => u,
    })
}

impl From<RawUser> for About {
    fn from(u: RawUser) -> Self {
        let roles = u.roles.unwrap_or_default();
        let socials = u.channel.and_then(|c| c.social_medias).unwrap_or_default();
        let panels = u
            .panels
            .into_iter()
            .flatten()
            // V106-23 Extension panels are iframes Twitch runs; nothing can draw them here.
            .filter(|p| p.kind.as_deref() != Some("EXTENSION"))
            .map(|p| Panel {
                title: some(p.title),
                image: web_image(p.image_url),
                link: web(p.link_url),
                body: some(p.description)
                    .map(|d| markdown::parse(&d))
                    .unwrap_or_default(),
            })
            // Some panels come back with nothing in them.
            .filter(Panel::shows)
            .collect();
        let mut socials: Vec<Social> = socials
            .into_iter()
            .filter_map(|s| {
                Some(Social {
                    url: web(s.url)?,
                    title: some(s.title).or_else(|| s.name.clone())?,
                    site: s.name.unwrap_or_default().to_ascii_lowercase(),
                })
            })
            .collect();
        // V106-21 Twitch can list the same link twice (@faith); the first one stays.
        let mut seen = HashSet::new();
        socials.retain(|s| seen.insert(s.url.clone()));
        About {
            id: u.id,
            name: some(u.display_name).unwrap_or_else(|| u.login.clone()),
            login: u.login,
            bio: u.description.unwrap_or_default().trim().to_owned(),
            created_at: u.created_at.unwrap_or_default(),
            avatar: u.profile_image_url,
            banner: web_image(u.banner_image_url),
            color: some(u.primary_color_hex).map(|c| format!("#{}", c.trim_start_matches('#'))),
            followers: u.followers.and_then(|f| f.total_count).unwrap_or(0),
            partner: roles.is_partner.unwrap_or(false),
            affiliate: roles.is_affiliate.unwrap_or(false),
            last_stream: u.last_broadcast.and_then(|b| {
                Some(LastStream {
                    started_at: b.started_at?,
                    title: b.title.unwrap_or_default(),
                    game: b.game.and_then(|g| some(g.display_name)),
                })
            }),
            socials,
            panels,
        }
    }
}

#[cfg(test)]
mod tests {
    use cyex_core::markdown::Span;
    use serde_json::Value;

    use super::super::{partial, Response};
    use super::*;

    fn raw(name: &str) -> &'static str {
        match name {
            "faith" => include_str!("../../tests/fixtures/about/faith.json"),
            "marc" => include_str!("../../tests/fixtures/about/marc.json"),
            "xqc" => include_str!("../../tests/fixtures/about/xqc.json"),
            "esl_csgo" => include_str!("../../tests/fixtures/about/esl_csgo.json"),
            "pokimane" => include_str!("../../tests/fixtures/about/pokimane.json"),
            "gamesdonequick" => include_str!("../../tests/fixtures/about/gamesdonequick.json"),
            "sodapoppin" => include_str!("../../tests/fixtures/about/sodapoppin.json"),
            _ => unreachable!(),
        }
    }

    fn parse(raw: &str) -> About {
        let res: Response<Data> = serde_json::from_str(raw).unwrap();
        About::from(partial(res, has_user).unwrap().user.unwrap())
    }

    fn fixture(name: &str) -> About {
        parse(raw(name))
    }

    /// faith's answer as JSON, to edit for the odd cases.
    fn faith_json() -> Value {
        serde_json::from_str(raw("faith")).unwrap()
    }

    /// faith with only the given panels.
    fn with_panels(panels: Value) -> About {
        let mut v = faith_json();
        v["data"]["user"]["panels"] = panels;
        parse(&v.to_string())
    }

    #[test]
    fn faith_socials_are_unique() {
        let a = fixture("faith");
        assert_eq!(a.socials.len(), 1, "the repeated link is dropped");
        assert_eq!(a.panels.len(), 7, "both extension panels are gone");
    }

    #[test]
    fn extension_panels_are_left_out() {
        let d: Data = serde_json::from_str(
            r#"{"user":{"id":"1","login":"xqc","displayName":"xQc","description":" hi ","createdAt":"2014-01-01T00:00:00.123456Z",
              "bannerImageURL":null,"primaryColorHex":"FF0000","profileImageURL":"https://p/a.png",
              "followers":{"totalCount":12000000},"roles":{"isPartner":true,"isAffiliate":false},
              "lastBroadcast":{"startedAt":"2026-10-08T20:00:00Z","title":"stream","game":null},
              "channel":{"socialMedias":[{"name":"youtube","title":"YouTube","url":"https://youtube.com/x "},
                                          {"name":"bad","title":"Bad","url":"javascript:x"}]},
              "panels":[{"id":"a","type":"DEFAULT","title":"Specs","description":"**CPU:** fast","imageURL":null,"linkURL":"https://shop.example "},
                        {"id":"b","type":"DEFAULT","title":null,"description":null,"imageURL":null,"linkURL":null},
                        {"id":"c","type":"EXTENSION"},
                        {"id":"d","type":"EXTENSION"}]}}"#,
        )
        .unwrap();
        let a = About::from(d.user.unwrap());
        assert_eq!((a.bio.as_str(), a.color.as_deref()), ("hi", Some("#FF0000")));
        assert_eq!(a.followers, 12_000_000);
        assert!(a.partner && !a.affiliate);
        assert_eq!(a.socials.len(), 1);
        assert_eq!(a.socials[0].url, "https://youtube.com/x");
        assert_eq!(a.panels.len(), 1, "the empty panel and both extensions are gone");
        assert_eq!(a.panels[0].link.as_deref(), Some("https://shop.example"));
        assert_eq!(a.panels[0].body.len(), 1);
        assert_eq!(a.last_stream.unwrap().title, "stream");
    }

    #[test]
    fn xqc_has_no_extension_panels() {
        let v: Value = serde_json::from_str(raw("xqc")).unwrap();
        let panels = v["data"]["user"]["panels"].as_array().unwrap();
        let ext = panels.iter().filter(|p| p["type"] == "EXTENSION").count();
        assert_eq!(ext, 3);
        assert!(fixture("xqc").panels.len() <= panels.len() - ext);
    }

    #[test]
    fn no_panels() {
        let a = fixture("esl_csgo");
        assert!(a.panels.is_empty());
        assert_eq!(a.banner, None);
    }

    #[test]
    fn many_panels() {
        assert_eq!(fixture("marc").panels.len(), 62);
    }

    #[test]
    fn image_only_panels_stay() {
        let v: Value = serde_json::from_str(raw("pokimane")).unwrap();
        let image_only = v["data"]["user"]["panels"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["imageURL"].is_string() && p["title"].is_null() && p["description"].is_null())
            .count();
        assert!(image_only > 0, "the fixture has some");
        let a = fixture("pokimane");
        let kept = a
            .panels
            .iter()
            .filter(|p| p.image.is_some() && p.title.is_none() && p.body.is_empty())
            .count();
        assert_eq!(kept, image_only);
    }

    #[test]
    fn long_markdown() {
        let v: Value = serde_json::from_str(raw("gamesdonequick")).unwrap();
        let panels = v["data"]["user"]["panels"].as_array().unwrap();
        let long = panels.iter().position(|p| {
            p["description"]
                .as_str()
                .is_some_and(|d| d.chars().count() >= 816)
        });
        let long = &panels[long.expect("the 816-char panel")];
        let a = fixture("gamesdonequick");
        let p = a.panels.iter().find(|p| {
            p.image == long["imageURL"].as_str().map(str::to_owned)
                && p.title == long["title"].as_str().map(str::to_owned)
        });
        assert!(!p.expect("kept").body.is_empty());
    }

    #[test]
    fn odd_characters() {
        let a = with_panels(
            json!([{"id":"1","type":"DEFAULT","title":"  ✨ Ｍｅｍｂｅｒｓ 💜\u{200b} ",
            "description":"<script>x</script> **b** ^^^ $3+ #h","imageURL":null,"linkURL":null}]),
        );
        let p = &a.panels[0];
        assert_eq!(
            p.title.as_deref(),
            Some("✨ Ｍｅｍｂｅｒｓ 💜\u{200b}"),
            "zero-width kept as Twitch sends it"
        );
        let Block::Paragraph { spans } = &p.body[0] else {
            panic!("{:?}", p.body)
        };
        let text: String = spans
            .iter()
            .filter_map(|s| match s {
                Span::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("<script>x</script>"),
            "HTML stays plain text: {text}"
        );
    }

    #[test]
    fn http_links_and_images() {
        let a = fixture("sodapoppin");
        assert!(a
            .panels
            .iter()
            .all(|p| p.image.as_deref().is_none_or(|u| u.starts_with("https://"))));
        let a = with_panels(json!([{"id":"1","type":"DEFAULT","title":"x","description":null,
            "imageURL":"http://panels.twitch.tv/x.png","linkURL":"javascript:alert(1)"}]));
        assert_eq!(
            a.panels[0].image.as_deref(),
            Some("https://panels.twitch.tv/x.png")
        );
        assert_eq!(a.panels[0].link, None);
        let mut v = faith_json();
        v["data"]["user"]["bannerImageURL"] = json!("http://static-cdn.jtvnw.net/b.png");
        assert_eq!(
            parse(&v.to_string()).banner.as_deref(),
            Some("https://static-cdn.jtvnw.net/b.png")
        );
    }

    #[test]
    fn very_large_image() {
        let url = format!("https://panels.twitch.tv/{}.png", "a".repeat(4000));
        let a = with_panels(
            json!([{"id":"1","type":"DEFAULT","title":null,"description":null,"imageURL":url,"linkURL":null}]),
        );
        assert_eq!(a.panels[0].image.as_deref(), Some(url.as_str()));
    }

    #[test]
    fn null_fields() {
        let mut v = faith_json();
        let u = &mut v["data"]["user"];
        u["followers"]["totalCount"] = Value::Null;
        u["lastBroadcast"]["game"] = json!({ "displayName": null });
        u["panels"] = Value::Null;
        u["displayName"] = Value::Null;
        let a = parse(&v.to_string());
        assert_eq!(a.followers, 0);
        assert_eq!(a.last_stream.map(|s| s.game), Some(None));
        assert!(a.panels.is_empty());
        assert_eq!(a.name, a.login);
    }

    #[test]
    fn partial_with_errors() {
        let mut v = faith_json();
        v["errors"] = json!([{ "message": "service error" }]);
        let a = parse(&v.to_string());
        assert_eq!(a.panels.len(), 7);
        // Without the user, the error wins.
        let res: Response<Data> =
            serde_json::from_str(r#"{"data":{"user":null},"errors":[{"message":"service error"}]}"#).unwrap();
        assert_eq!(
            partial(res, has_user)
                .err()
                .and_then(|e| e.api_message().map(str::to_owned))
                .as_deref(),
            Some("service error")
        );
    }

    #[test]
    fn failing_image_url() {
        let a = with_panels(json!([{"id":"1","type":"DEFAULT","title":null,"description":null,
            "imageURL":"https://panels.twitch.tv/missing.png","linkURL":null}]));
        assert_eq!(
            a.panels[0].image.as_deref(),
            Some("https://panels.twitch.tv/missing.png")
        );
    }

    #[test]
    fn marc_blank_images_go() {
        let analysis: Vec<Value> =
            serde_json::from_str(include_str!("../../tests/fixtures/about/marc-blank.json")).unwrap();
        let tail = |i: usize| {
            analysis.iter().find(|e| e["i"] == i).unwrap()["url"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let mut a = fixture("marc");
        let before = a.panels.clone();
        let images = a.panel_images();
        assert_eq!(
            images.len(),
            images.iter().collect::<HashSet<_>>().len(),
            "each once"
        );
        let blank: HashSet<String> = images
            .into_iter()
            .filter(|u| {
                analysis
                    .iter()
                    .any(|e| e["transparent"] == 1 && u.ends_with(e["url"].as_str().unwrap()))
            })
            .collect();
        a.without_images(&blank);
        assert_eq!(a.panels.len(), before.len() - 14);
        let with_text = before
            .iter()
            .filter(|p| p.image.as_ref().is_some_and(|u| blank.contains(u)) && !p.body.is_empty())
            .count();
        assert_eq!(with_text, 6);
        assert!(a
            .panels
            .iter()
            .all(|p| p.image.as_ref().is_none_or(|u| !blank.contains(u))));
        let visible = before
            .iter()
            .filter(|p| p.image.as_ref().is_some_and(|u| !blank.contains(u)))
            .count();
        assert_eq!(
            a.panels.iter().filter(|p| p.image.is_some()).count(),
            visible,
            "real images stay"
        );
        let thin = tail(13);
        assert!(
            a.panels
                .iter()
                .any(|p| p.image.as_ref().is_some_and(|u| u.ends_with(&thin))),
            "the 99 % graphic stays"
        );
    }
}
