//! A channel's about page (1.21): bio, followers, last stream, social links and panels.
//! Public, checked 2026/10/09. Extension panels are iframes and can't be drawn here, so
//! they only carry the extension's name.

use cyex_core::markdown::{self, Block};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

const ABOUT: &str = "query($login: String!) { user(login: $login) { id login displayName description createdAt \
    bannerImageURL primaryColorHex profileImageURL(width: 150) followers { totalCount } \
    roles { isPartner isAffiliate } lastBroadcast { startedAt title game { displayName } } \
    channel { socialMedias { name title url } selfInstalledExtensions { installation { \
    extension { name } activationConfig { slot } } } } \
    panels { id type ... on DefaultPanel { title description imageURL linkURL } ... on ExtensionPanel { slotID } } } }";

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
    /// An extension panel: the extension's name, if Twitch says.
    pub extension: Option<String>,
}

/// `None` if the channel doesn't exist.
pub async fn about(http: &Http, login: &str) -> Result<Option<About>> {
    let data: Data = query(http, Auth::PUBLIC, ABOUT, json!({ "login": login })).await?;
    Ok(data.user.map(About::from))
}

#[derive(Deserialize)]
struct Data {
    user: Option<RawUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    login: String,
    display_name: String,
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
    #[serde(default)]
    panels: Vec<Option<RawPanel>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCount {
    total_count: u64,
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
    display_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    #[serde(default)]
    social_medias: Option<Vec<RawSocial>>,
    #[serde(default)]
    self_installed_extensions: Option<Vec<RawInstalled>>,
}

#[derive(Deserialize)]
struct RawSocial {
    name: Option<String>,
    title: Option<String>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct RawInstalled {
    installation: Option<RawInstallation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInstallation {
    extension: Option<RawExtension>,
    activation_config: Option<RawActivation>,
}

#[derive(Deserialize)]
struct RawExtension {
    name: Option<String>,
}

#[derive(Deserialize)]
struct RawActivation {
    slot: Option<String>,
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
    #[serde(rename = "slotID")]
    slot_id: Option<String>,
}

/// Trimmed, `None` if empty.
fn some(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn web(s: Option<String>) -> Option<String> {
    some(s).filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

impl From<RawUser> for About {
    fn from(u: RawUser) -> Self {
        let roles = u.roles.unwrap_or_default();
        let channel = u.channel;
        let (socials, installed) = match channel {
            Some(c) => (c.social_medias.unwrap_or_default(), c.self_installed_extensions.unwrap_or_default()),
            None => (Vec::new(), Vec::new()),
        };
        let extension_name = |slot: &str| {
            installed
                .iter()
                .filter_map(|i| i.installation.as_ref())
                .find(|i| i.activation_config.as_ref().and_then(|a| a.slot.as_deref()) == Some(slot))
                .and_then(|i| i.extension.as_ref()?.name.clone())
        };
        let panels = u
            .panels
            .into_iter()
            .flatten()
            .filter_map(|p| {
                if p.kind.as_deref() == Some("EXTENSION") {
                    let name = p.slot_id.as_deref().and_then(extension_name);
                    return Some(Panel {
                        title: None,
                        image: None,
                        link: None,
                        body: Vec::new(),
                        extension: Some(name.unwrap_or_default()),
                    });
                }
                let panel = Panel {
                    title: some(p.title),
                    image: web(p.image_url),
                    link: web(p.link_url),
                    body: some(p.description).map(|d| markdown::parse(&d)).unwrap_or_default(),
                    extension: None,
                };
                // Some panels come back with nothing in them.
                (panel.title.is_some() || panel.image.is_some() || !panel.body.is_empty()).then_some(panel)
            })
            .collect();
        About {
            id: u.id,
            login: u.login,
            name: u.display_name,
            bio: u.description.unwrap_or_default().trim().to_owned(),
            created_at: u.created_at.unwrap_or_default(),
            avatar: u.profile_image_url,
            banner: web(u.banner_image_url),
            color: some(u.primary_color_hex).map(|c| format!("#{}", c.trim_start_matches('#'))),
            followers: u.followers.map_or(0, |f| f.total_count),
            partner: roles.is_partner.unwrap_or(false),
            affiliate: roles.is_affiliate.unwrap_or(false),
            last_stream: u.last_broadcast.and_then(|b| {
                Some(LastStream {
                    started_at: b.started_at?,
                    title: b.title.unwrap_or_default(),
                    game: b.game.map(|g| g.display_name),
                })
            }),
            socials: socials
                .into_iter()
                .filter_map(|s| {
                    Some(Social {
                        url: web(s.url)?,
                        title: some(s.title).or_else(|| s.name.clone())?,
                        site: s.name.unwrap_or_default().to_ascii_lowercase(),
                    })
                })
                .collect(),
            panels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_parses_panels_and_extensions() {
        let d: Data = serde_json::from_str(
            r#"{"user":{"id":"1","login":"xqc","displayName":"xQc","description":" hi ","createdAt":"2014-01-01T00:00:00.123456Z",
              "bannerImageURL":null,"primaryColorHex":"FF0000","profileImageURL":"https://p/a.png",
              "followers":{"totalCount":12000000},"roles":{"isPartner":true,"isAffiliate":false},
              "lastBroadcast":{"startedAt":"2026-10-08T20:00:00Z","title":"stream","game":null},
              "channel":{"socialMedias":[{"name":"youtube","title":"YouTube","url":"https://youtube.com/x "},
                                          {"name":"bad","title":"Bad","url":"javascript:x"}],
                         "selfInstalledExtensions":[{"installation":{"extension":{"name":"PC Build"},
                           "activationConfig":{"slot":"extension-panel-1"}}}]},
              "panels":[{"id":"a","type":"DEFAULT","title":"Specs","description":"**CPU:** fast","imageURL":null,"linkURL":"https://shop.example "},
                        {"id":"b","type":"DEFAULT","title":null,"description":null,"imageURL":null,"linkURL":null},
                        {"id":"c","type":"EXTENSION","slotID":"extension-panel-1"},
                        {"id":"d","type":"EXTENSION","slotID":"extension-panel-2"}]}}"#,
        )
        .unwrap();
        let a = About::from(d.user.unwrap());
        assert_eq!((a.bio.as_str(), a.color.as_deref()), ("hi", Some("#FF0000")));
        assert_eq!(a.followers, 12_000_000);
        assert!(a.partner && !a.affiliate);
        assert_eq!(a.socials.len(), 1);
        assert_eq!(a.socials[0].url, "https://youtube.com/x");
        assert_eq!(a.panels.len(), 3, "the empty panel is dropped");
        assert_eq!(a.panels[0].link.as_deref(), Some("https://shop.example"));
        assert_eq!(a.panels[0].body.len(), 1);
        assert_eq!(a.panels[1].extension.as_deref(), Some("PC Build"));
        assert_eq!(a.panels[2].extension.as_deref(), Some(""));
        assert_eq!(a.last_stream.unwrap().title, "stream");
    }
}
