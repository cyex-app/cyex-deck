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
    social_medias: Option<Vec<Option<RawSocial>>>,
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
            .flatten()
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
            // R23 Same https-only rule as the banner and panel images.
            avatar: web_image(u.profile_image_url),
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
mod tests;
