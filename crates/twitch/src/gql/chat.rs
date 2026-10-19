//! What chat needs from GQL, readable signed out (1.9, 1.40): badges, cheermotes, the
//! channel's pinned message and its channel points rewards.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::points::{Points, RawPointsSettings};
use super::{Auth, query};
use crate::{Http, Result};

pub(crate) const BADGE_FIELDS: &str = "setID version title description clickURL \
    image1x: imageURL(size: NORMAL) image2x: imageURL(size: DOUBLE) image4x: imageURL(size: QUADRUPLE)";

const GLOBAL: &str = "query { badges { BADGE } \
    cheerConfig { displayConfig { colors { bits color } } groups { templateURL nodes { prefix tiers { bits } } } } }";

const CHANNEL: &str = "query($login: String!) { \
    user(login: $login) { id broadcastBadges { BADGE } \
      cheer { cheerGroups { templateURL nodes { prefix tiers { bits } } } } } \
    channel(name: $login) { communityPointsSettings { POINTS } } }";

const PINNED: &str = "query($id: ID!) { channel(id: $id) { pinnedChatMessages(first: 1) { edges { node { \
    id type startsAt endsAt pinnedBy { login displayName } \
    pinnedMessage { id sender { id login displayName chatColor } \
      content { text fragments { text content { __typename ... on Emote { emoteID: id token } ... on User { login } } } } } \
    } } } } }";

/// A Twitch chat badge (`set/version`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BadgeInfo {
    pub set: String,
    pub version: String,
    pub title: String,
    pub description: Option<String>,
    pub click_url: Option<String>,
    pub image1x: String,
    pub image2x: String,
    pub image4x: String,
}

/// Bits emote. `template` has PREFIX, BACKGROUND, ANIMATION, TIER, SCALE and EXTENSION
/// placeholders (prefix lowercase).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Cheermote {
    pub prefix: String,
    pub template: String,
    pub tiers: Vec<CheerTier>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CheerTier {
    pub bits: u32,
    pub color: String,
}

pub struct GlobalChat {
    pub badges: Vec<BadgeInfo>,
    pub cheermotes: Vec<Cheermote>,
    /// Tier colors by minimum bits, ascending. Custom cheermotes use these too.
    pub cheer_colors: Vec<(u32, String)>,
}

pub struct ChannelChat {
    pub channel_id: String,
    pub badges: Vec<BadgeInfo>,
    /// Tier colors still to be filled from [`GlobalChat::cheer_colors`].
    pub cheermotes: Vec<Cheermote>,
    pub points: Option<Points>,
}

pub struct PinnedMessage {
    pub id: String,
    /// `MOD` or `PAID`.
    pub kind: String,
    /// V106-19 RFC 3339.
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub pinned_by: Option<String>,
    /// V107-P5 For the usercard.
    pub pinned_by_login: Option<String>,
    pub message_id: String,
    pub sender: Option<PinSender>,
    pub fragments: Vec<PinFragment>,
}

pub struct PinSender {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub color: Option<String>,
}

pub enum PinFragment {
    Text(String),
    Emote { id: String, name: String },
    Mention { text: String, login: String },
}

pub async fn global_chat(http: &Http) -> Result<GlobalChat> {
    let q = GLOBAL.replace("BADGE", BADGE_FIELDS);
    let data: RawGlobal = query(http, Auth::PUBLIC, &q, json!({})).await?;
    let mut colors: Vec<(u32, String)> = data
        .cheer_config
        .display_config
        .colors
        .into_iter()
        .map(|c| (c.bits, c.color))
        .collect();
    colors.sort_by_key(|c| c.0);
    let cheermotes = cheermotes(data.cheer_config.groups, &colors);
    Ok(GlobalChat {
        badges: data.badges.into_iter().flatten().map(BadgeInfo::from).collect(),
        cheermotes,
        cheer_colors: colors,
    })
}

/// `None` if the channel doesn't exist.
pub async fn channel_chat(http: &Http, login: &str, cheer_colors: &[(u32, String)]) -> Result<Option<ChannelChat>> {
    let q = CHANNEL
        .replace("BADGE", BADGE_FIELDS)
        .replace("POINTS", super::points::SETTINGS_FIELDS);
    let data: RawChannel = query(http, Auth::PUBLIC, &q, json!({ "login": login })).await?;
    let Some(user) = data.user else { return Ok(None) };
    let groups = user.cheer.map(|c| c.cheer_groups).unwrap_or_default();
    Ok(Some(ChannelChat {
        channel_id: user.id,
        badges: user.broadcast_badges.into_iter().flatten().map(BadgeInfo::from).collect(),
        cheermotes: cheermotes(groups, cheer_colors),
        points: data
            .channel
            .and_then(|c| c.community_points_settings)
            .map(Points::from),
    }))
}

/// The channel's pinned message, if any.
pub async fn pinned(http: &Http, channel_id: &str) -> Result<Option<PinnedMessage>> {
    let data: RawPinData = query(http, Auth::PUBLIC, PINNED, json!({ "id": channel_id })).await?;
    let node = data
        .channel
        .and_then(|c| c.pinned_chat_messages)
        .and_then(|p| p.edges.into_iter().next())
        .map(|e| e.node);
    Ok(node.map(|n| {
        let msg = n.pinned_message;
        PinnedMessage {
            id: n.id,
            kind: n.kind,
            starts_at: n.starts_at,
            ends_at: n.ends_at,
            pinned_by_login: n.pinned_by.as_ref().and_then(|p| p.login.clone()),
            pinned_by: n.pinned_by.map(|p| p.display_name),
            message_id: msg.as_ref().map(|m| m.id.clone()).unwrap_or_default(),
            sender: msg.as_ref().and_then(|m| m.sender.as_ref()).map(|s| PinSender {
                id: s.id.clone(),
                login: s.login.clone(),
                display_name: s.display_name.clone(),
                color: s.chat_color.clone(),
            }),
            fragments: msg
                .map(|m| m.content.fragments.into_iter().map(PinFragment::from).collect())
                .unwrap_or_default(),
        }
    }))
}

fn cheermotes(groups: Vec<RawCheerGroup>, colors: &[(u32, String)]) -> Vec<Cheermote> {
    let color_for = |bits: u32| {
        colors
            .iter()
            .rev()
            .find(|(min, _)| *min <= bits)
            .map_or_else(|| "#979797".to_owned(), |c| c.1.clone())
    };
    groups
        .into_iter()
        .flat_map(|g| {
            let template = g.template_url;
            g.nodes.into_iter().map(move |n| (template.clone(), n))
        })
        .map(|(template, n)| Cheermote {
            prefix: n.prefix,
            template,
            tiers: n
                .tiers
                .into_iter()
                .map(|t| CheerTier {
                    bits: t.bits,
                    color: color_for(t.bits),
                })
                .collect(),
        })
        .collect()
}

// Raw shapes.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGlobal {
    #[serde(default)]
    badges: Vec<Option<RawBadge>>,
    cheer_config: RawCheerConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheerConfig {
    display_config: RawDisplayConfig,
    #[serde(default)]
    groups: Vec<RawCheerGroup>,
}

#[derive(Deserialize)]
struct RawDisplayConfig {
    #[serde(default)]
    colors: Vec<RawColor>,
}

#[derive(Deserialize)]
struct RawColor {
    bits: u32,
    color: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheerGroup {
    #[serde(rename = "templateURL")]
    template_url: String,
    #[serde(default)]
    nodes: Vec<RawCheerNode>,
}

#[derive(Deserialize)]
struct RawCheerNode {
    prefix: String,
    #[serde(default)]
    tiers: Vec<RawTier>,
}

#[derive(Deserialize)]
struct RawTier {
    bits: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawBadge {
    #[serde(rename = "setID")]
    set_id: String,
    version: String,
    title: String,
    description: Option<String>,
    #[serde(rename = "clickURL")]
    click_url: Option<String>,
    image1x: String,
    image2x: String,
    image4x: String,
}

impl From<RawBadge> for BadgeInfo {
    fn from(b: RawBadge) -> Self {
        Self {
            set: b.set_id,
            version: b.version,
            title: b.title,
            description: b.description.filter(|d| !d.is_empty()),
            click_url: b.click_url.filter(|u| !u.is_empty()),
            image1x: b.image1x,
            image2x: b.image2x,
            image4x: b.image4x,
        }
    }
}

#[derive(Deserialize)]
struct RawChannel {
    user: Option<RawChannelUser>,
    channel: Option<RawChannelPoints>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannelUser {
    id: String,
    #[serde(default)]
    broadcast_badges: Vec<Option<RawBadge>>,
    cheer: Option<RawCheer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheer {
    #[serde(default)]
    cheer_groups: Vec<RawCheerGroup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannelPoints {
    community_points_settings: Option<RawPointsSettings>,
}

#[derive(Deserialize)]
struct RawPinData {
    channel: Option<RawPinChannel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPinChannel {
    pinned_chat_messages: Option<RawEdges>,
}

#[derive(Deserialize)]
struct RawEdges {
    #[serde(default)]
    edges: Vec<RawEdge>,
}

#[derive(Deserialize)]
struct RawEdge {
    node: RawPin,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPin {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    starts_at: Option<String>,
    ends_at: Option<String>,
    pinned_by: Option<RawPinnedBy>,
    pinned_message: Option<RawPinMessage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPinnedBy {
    #[serde(default)]
    login: Option<String>,
    display_name: String,
}

#[derive(Deserialize)]
struct RawPinMessage {
    id: String,
    sender: Option<RawPinSender>,
    content: RawContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPinSender {
    id: String,
    login: String,
    display_name: String,
    chat_color: Option<String>,
}

#[derive(Deserialize)]
struct RawContent {
    #[serde(default)]
    fragments: Vec<RawFragment>,
}

#[derive(Deserialize)]
struct RawFragment {
    text: String,
    content: Option<RawFragmentContent>,
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum RawFragmentContent {
    Emote {
        #[serde(rename = "emoteID")]
        emote_id: String,
        token: String,
    },
    User {
        login: String,
    },
    #[serde(other)]
    Other,
}

impl From<RawFragment> for PinFragment {
    fn from(f: RawFragment) -> Self {
        match f.content {
            Some(RawFragmentContent::Emote { emote_id, token }) => PinFragment::Emote { id: emote_id, name: token },
            Some(RawFragmentContent::User { login }) => PinFragment::Mention { text: f.text, login },
            _ => PinFragment::Text(f.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_parses_and_colors_tiers() {
        let raw: RawGlobal = serde_json::from_str(
            r##"{"badges":[{"setID":"vip","version":"1","title":"VIP","description":"","clickURL":null,
                "image1x":"a/1","image2x":"a/2","image4x":"a/3"}],
              "cheerConfig":{"displayConfig":{"colors":[{"bits":100,"color":"#9c3ee8"},{"bits":1,"color":"#979797"}]},
                "groups":[{"templateURL":"https://x/actions/PREFIX/BACKGROUND/ANIMATION/TIER/SCALE.EXTENSION",
                  "nodes":[{"prefix":"Cheer","tiers":[{"bits":1},{"bits":100},{"bits":500}]}]}]}}"##,
        )
        .unwrap();
        let mut colors: Vec<_> = raw.cheer_config.display_config.colors.iter().map(|c| (c.bits, c.color.clone())).collect();
        colors.sort_by_key(|c| c.0);
        let c = cheermotes(raw.cheer_config.groups, &colors);
        assert_eq!(c[0].tiers.iter().map(|t| t.color.as_str()).collect::<Vec<_>>(), ["#979797", "#9c3ee8", "#9c3ee8"]);
        let b: Vec<BadgeInfo> = raw.badges.into_iter().flatten().map(BadgeInfo::from).collect();
        assert_eq!(b[0].description, None);
    }

    #[test]
    fn pinned_parses() {
        let raw: RawPinData = serde_json::from_str(
            r##"{"channel":{"pinnedChatMessages":{"edges":[{"node":{"id":"p1","type":"MOD","startsAt":"2026-10-11T12:00:00Z","endsAt":null,
              "pinnedBy":{"login":"mod","displayName":"Mod"},"pinnedMessage":{"id":"m1","sender":{"id":"7","login":"foo","displayName":"Foo","chatColor":"#FF0000"},
              "content":{"text":"hi Kappa @bar","fragments":[{"text":"hi ","content":null},{"text":"Kappa","content":{"__typename":"Emote","emoteID":"25","token":"Kappa"}},
              {"text":" ","content":null},{"text":"@bar","content":{"__typename":"User","login":"bar"}}]}}}}]}}}"##,
        )
        .unwrap();
        let node = raw.channel.unwrap().pinned_chat_messages.unwrap().edges.into_iter().next().unwrap().node;
        assert_eq!(node.starts_at.as_deref(), Some("2026-10-11T12:00:00Z"));
        assert_eq!(node.pinned_by.as_ref().and_then(|p| p.login.as_deref()), Some("mod"), "V107-P5");
        let frags: Vec<PinFragment> = node.pinned_message.unwrap().content.fragments.into_iter().map(PinFragment::from).collect();
        assert!(matches!(&frags[1], PinFragment::Emote { id, .. } if id == "25"));
        assert!(matches!(&frags[3], PinFragment::Mention { login, .. } if login == "bar"));
    }
}
