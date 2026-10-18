//! Twitch emotes (1.40, 1.41, E5): the ones an account can use, a channel's own, and one
//! emote's details for its card. Research: `research/twitch-api.md`, "Emotes, badges".

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

/// Pages of `availableEmoteSetsPaginated`. Accounts with hundreds of subs fit in a few.
const MAX_PAGES: usize = 30;

const AVAILABLE: &str = "query($channelID: ID!, $cursor: Cursor) { channel(id: $channelID) { self { \
    availableEmoteSetsPaginated(pageLimit: 350, after: $cursor) { \
      edges { cursor node { id emotes { id setID token type assetType modifiers { code } } \
        owner { id login displayName profileImageURL(width: 28) } } } \
      pageInfo { hasNextPage } } } } }";

const CHANNEL: &str = "query($id: ID!) { user(id: $id) { id login displayName profileImageURL(width: 28) \
    subscriptionProducts { tier emotes { EMOTE } } \
    cheer { badgeTierEmotes(filter: ALL) { EMOTE bitsBadgeTierSummary { threshold } } } \
    channel { localEmoteSets { id emotes { EMOTE } } } } }";

const CARD: &str = "query($id: ID!) { emote(id: $id) { EMOTE subscriptionTier \
    artist { login displayName } owner { id login displayName profileImageURL(width: 28) } \
    bitsBadgeTierSummary { threshold } } }";

const EMOTE_FIELDS: &str = "id token type setID assetType";

/// Where an emote comes from, as the emote menu groups them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EmoteKind {
    Global,
    Subscription,
    Follower,
    Bits,
    /// Unlocked with channel points.
    Points,
    HypeTrain,
    Prime,
    Turbo,
    LimitedTime,
    TwoFactor,
    Other,
}

impl EmoteKind {
    /// From GQL's `Emote.type` or Helix's `emote_type`, with the set ids twitch.tv
    /// treats specially (FFZ's list).
    pub fn classify(set_id: &str, kind: &str) -> Self {
        match set_id {
            "0" | "33" | "42" => return EmoteKind::Global,
            "457" | "793" => return EmoteKind::Turbo,
            "19151" | "19194" => return EmoteKind::Prime,
            "300238151" => return EmoteKind::Points,
            "300374282" => return EmoteKind::TwoFactor,
            _ => {}
        }
        let k: String = kind
            .chars()
            .filter(|c| *c != '_')
            .collect::<String>()
            .to_ascii_lowercase();
        match k.as_str() {
            "globals" | "smilies" => EmoteKind::Global,
            "subscriptions" => EmoteKind::Subscription,
            "follower" => EmoteKind::Follower,
            "bitsbadgetiers" | "bitstier" => EmoteKind::Bits,
            "channelpoints" => EmoteKind::Points,
            "hypetrain" => EmoteKind::HypeTrain,
            "prime" => EmoteKind::Prime,
            "turbo" => EmoteKind::Turbo,
            "limitedtime" | "megacommerce" => EmoteKind::LimitedTime,
            "twofactor" => EmoteKind::TwoFactor,
            _ => EmoteKind::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwitchEmote {
    pub id: String,
    pub name: String,
    pub set_id: String,
    pub kind: EmoteKind,
    pub animated: bool,
    /// Modified versions unlocked with points (`BW`), used as `<id>_<code>`.
    pub modifiers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmoteOwner {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
}

/// One emote set the account can use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmoteSet {
    pub id: String,
    /// The channel, or `None` for Twitch's own sets.
    pub owner: Option<EmoteOwner>,
    pub emotes: Vec<TwitchEmote>,
}

/// A channel's own emotes, whether the viewer has them or not (E5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelEmotes {
    pub owner: EmoteOwner,
    /// Tier 1 to 3, in order.
    pub tiers: Vec<SubTier>,
    pub follower: Vec<TwitchEmote>,
    pub bits: Vec<BitsEmote>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SubTier {
    /// 1, 2 or 3.
    pub tier: u8,
    pub emotes: Vec<TwitchEmote>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BitsEmote {
    pub emote: TwitchEmote,
    /// Bits cheered in the channel to unlock it.
    pub bits: u32,
}

/// What an emote card shows for a Twitch emote (1.41).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwitchEmoteInfo {
    pub emote: TwitchEmote,
    /// The channel it belongs to; `None` for Twitch's own.
    pub owner: Option<EmoteOwner>,
    pub tier: Option<u8>,
    pub bits: Option<u32>,
    /// Who drew it, when the channel says.
    pub artist: Option<String>,
    /// 1.41 The artist's login, for the in-app channel link.
    #[serde(skip)]
    pub artist_login: Option<String>,
    /// GQL's own `type` (`TWO_FACTOR`, `SMILIES`, `ARCHIVE`, `MEGA_COMMERCE`, ...) for card labels.
    #[serde(skip)]
    pub raw_type: Option<String>,
}

/// Every emote set the account can use in a channel, the channel's follower emotes
/// included. Needs the account's GQL token.
pub async fn available_emotes(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<EmoteSet>> {
    let mut sets = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let data: RawAvailable = query(
            http,
            auth,
            AVAILABLE,
            json!({ "channelID": channel_id, "cursor": cursor }),
        )
        .await?;
        let Some(page) = data
            .channel
            .and_then(|c| c.self_)
            .and_then(|s| s.available_emote_sets_paginated)
        else {
            break;
        };
        cursor = page.edges.last().and_then(|e| e.cursor.clone());
        sets.extend(page.edges.into_iter().filter_map(|e| e.node).map(EmoteSet::from));
        if !page.page_info.has_next_page || cursor.is_none() {
            break;
        }
    }
    Ok(sets)
}

/// A channel's sub, follower and bits emotes, readable signed out. `None` if the
/// channel doesn't exist.
pub async fn channel_emotes(http: &Http, channel_id: &str) -> Result<Option<ChannelEmotes>> {
    let q = CHANNEL.replace("EMOTE", EMOTE_FIELDS);
    let data: RawChannelData = query(http, Auth::PUBLIC, &q, json!({ "id": channel_id })).await?;
    Ok(data.user.map(ChannelEmotes::from))
}

/// One emote's details, readable signed out. `None` if Twitch doesn't know it.
pub async fn emote_info(http: &Http, id: &str) -> Result<Option<TwitchEmoteInfo>> {
    let q = CARD.replace("EMOTE", EMOTE_FIELDS);
    let data: RawCardData = query(http, Auth::PUBLIC, &q, json!({ "id": id })).await?;
    Ok(data.emote.map(TwitchEmoteInfo::from))
}

// Raw shapes.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawEmote {
    id: String,
    token: String,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(rename = "setID", default)]
    set_id: Option<String>,
    #[serde(default)]
    asset_type: Option<String>,
    #[serde(default)]
    modifiers: Option<Vec<RawModifier>>,
}

#[derive(Deserialize)]
struct RawModifier {
    code: String,
}

impl From<RawEmote> for TwitchEmote {
    fn from(e: RawEmote) -> Self {
        let set_id = e.set_id.unwrap_or_default();
        Self {
            kind: EmoteKind::classify(&set_id, e.kind.as_deref().unwrap_or("")),
            id: e.id,
            name: e.token,
            set_id,
            animated: e.asset_type.as_deref() == Some("ANIMATED"),
            modifiers: e
                .modifiers
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.code)
                .collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOwner {
    id: String,
    login: String,
    display_name: String,
    #[serde(rename = "profileImageURL", default)]
    profile_image_url: Option<String>,
}

impl From<RawOwner> for EmoteOwner {
    fn from(o: RawOwner) -> Self {
        Self {
            id: o.id,
            login: o.login,
            name: o.display_name,
            avatar: o.profile_image_url,
        }
    }
}

/// Twitch's test account owns some of its own sets (2FA, promotions).
fn real_owner(o: Option<RawOwner>) -> Option<EmoteOwner> {
    o.filter(|o| o.id != "139075904").map(EmoteOwner::from)
}

#[derive(Deserialize)]
struct RawAvailable {
    channel: Option<RawChannelSelf>,
}

#[derive(Deserialize)]
struct RawChannelSelf {
    #[serde(rename = "self")]
    self_: Option<RawSelf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSelf {
    available_emote_sets_paginated: Option<RawSetPage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSetPage {
    #[serde(default)]
    edges: Vec<RawSetEdge>,
    page_info: RawPageInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPageInfo {
    #[serde(default)]
    has_next_page: bool,
}

#[derive(Deserialize)]
struct RawSetEdge {
    cursor: Option<String>,
    node: Option<RawSet>,
}

#[derive(Deserialize)]
struct RawSet {
    id: String,
    #[serde(default)]
    emotes: Vec<Option<RawEmote>>,
    owner: Option<RawOwner>,
}

impl From<RawSet> for EmoteSet {
    fn from(s: RawSet) -> Self {
        Self {
            emotes: s
                .emotes
                .into_iter()
                .flatten()
                .map(|e| {
                    let mut e = TwitchEmote::from(e);
                    if e.set_id.is_empty() {
                        e.set_id.clone_from(&s.id);
                        e.kind = EmoteKind::classify(&s.id, "");
                    }
                    e
                })
                .collect(),
            owner: real_owner(s.owner),
            id: s.id,
        }
    }
}

#[derive(Deserialize)]
struct RawChannelData {
    user: Option<RawChannelUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannelUser {
    #[serde(flatten)]
    owner: RawOwner,
    #[serde(default)]
    subscription_products: Option<Vec<Option<RawProduct>>>,
    cheer: Option<RawCheer>,
    channel: Option<RawLocal>,
}

#[derive(Deserialize)]
struct RawProduct {
    tier: String,
    #[serde(default)]
    emotes: Vec<Option<RawEmote>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheer {
    #[serde(default)]
    badge_tier_emotes: Vec<Option<RawBitsEmote>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBitsEmote {
    #[serde(flatten)]
    emote: RawEmote,
    bits_badge_tier_summary: Option<RawThreshold>,
}

#[derive(Deserialize)]
struct RawThreshold {
    threshold: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLocal {
    #[serde(default)]
    local_emote_sets: Option<Vec<Option<RawSet>>>,
}

impl From<RawChannelUser> for ChannelEmotes {
    fn from(u: RawChannelUser) -> Self {
        let mut tiers: Vec<SubTier> = Vec::new();
        for p in u.subscription_products.unwrap_or_default().into_iter().flatten() {
            let tier = match p.tier.as_str() {
                "1000" => 1,
                "2000" => 2,
                "3000" => 3,
                _ => continue,
            };
            let emotes: Vec<TwitchEmote> = p.emotes.into_iter().flatten().map(TwitchEmote::from).collect();
            match tiers.iter_mut().find(|t| t.tier == tier) {
                Some(t) => t.emotes.extend(emotes),
                None => tiers.push(SubTier { tier, emotes }),
            }
        }
        tiers.sort_by_key(|t| t.tier);
        let follower = u
            .channel
            .and_then(|c| c.local_emote_sets)
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .flat_map(|s| EmoteSet::from(s).emotes)
            .map(|mut e| {
                e.kind = EmoteKind::Follower;
                e
            })
            .collect();
        let mut bits: Vec<BitsEmote> = u
            .cheer
            .map(|c| c.badge_tier_emotes)
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|b| BitsEmote {
                bits: b.bits_badge_tier_summary.map_or(0, |t| t.threshold),
                emote: TwitchEmote {
                    kind: EmoteKind::Bits,
                    ..TwitchEmote::from(b.emote)
                },
            })
            .collect();
        bits.sort_by_key(|b| b.bits);
        Self {
            owner: u.owner.into(),
            tiers,
            follower,
            bits,
        }
    }
}

#[derive(Deserialize)]
struct RawCardData {
    emote: Option<RawCard>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCard {
    #[serde(flatten)]
    emote: RawEmote,
    subscription_tier: Option<String>,
    artist: Option<RawArtist>,
    owner: Option<RawOwner>,
    bits_badge_tier_summary: Option<RawThreshold>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawArtist {
    #[serde(default)]
    login: Option<String>,
    display_name: String,
}

impl From<RawCard> for TwitchEmoteInfo {
    fn from(c: RawCard) -> Self {
        Self {
            raw_type: c.emote.kind.clone(),
            emote: c.emote.into(),
            owner: real_owner(c.owner),
            tier: c
                .subscription_tier
                .as_deref()
                .and_then(|t| t.strip_prefix("TIER_")?.parse().ok()),
            bits: c.bits_badge_tier_summary.map(|t| t.threshold),
            artist_login: c.artist.as_ref().and_then(|a| a.login.clone()),
            artist: c.artist.map(|a| a.display_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_set_ids_first() {
        assert_eq!(EmoteKind::classify("42", "SUBSCRIPTIONS"), EmoteKind::Global);
        assert_eq!(
            EmoteKind::classify("19503", "SUBSCRIPTIONS"),
            EmoteKind::Subscription
        );
        assert_eq!(EmoteKind::classify("1", "BITS_BADGE_TIERS"), EmoteKind::Bits);
        assert_eq!(EmoteKind::classify("1", "bitstier"), EmoteKind::Bits);
        assert_eq!(EmoteKind::classify("1", "hypetrain"), EmoteKind::HypeTrain);
        assert_eq!(EmoteKind::classify("1", "NEW_THING"), EmoteKind::Other);
    }

    #[test]
    fn channel_emotes_parse() {
        let data: RawChannelData =
            serde_json::from_str(include_str!("../../tests/fixtures/channel_emotes.json"))
                .map(|d: serde_json::Value| serde_json::from_value(d["data"].clone()).unwrap())
                .unwrap();
        let c = ChannelEmotes::from(data.user.unwrap());
        assert_eq!(c.owner.login, "xqc");
        assert_eq!(c.tiers.iter().map(|t| t.tier).collect::<Vec<_>>(), [1, 2, 3]);
        let t1 = &c.tiers[0].emotes;
        assert!(
            t1.iter()
                .any(|e| e.name == "xqcSlam" && e.animated && e.set_id == "359369083")
        );
        assert!(c.follower.iter().all(|e| e.kind == EmoteKind::Follower));
        assert!(!c.follower.is_empty());
    }

    #[test]
    fn card_parses() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/emote_card.json")).unwrap();
        let data: RawCardData = serde_json::from_value(v["data"].clone()).unwrap();
        let info = TwitchEmoteInfo::from(data.emote.unwrap());
        assert_eq!(info.emote.name, "xqcSlam");
        assert_eq!(info.tier, Some(1));
        assert_eq!(info.raw_type.as_deref(), Some("SUBSCRIPTIONS"));
        assert_eq!(info.owner.unwrap().login, "xqc");
        assert!(info.emote.animated);
    }

    #[test]
    fn card_artist_has_login() {
        let raw = r#"{"id":"1","token":"x","type":"SUBSCRIPTIONS","setID":"9","subscriptionTier":"TIER_3",
            "artist":{"login":"drawer","displayName":"Drawer"},"owner":null,"bitsBadgeTierSummary":null}"#;
        let info = TwitchEmoteInfo::from(serde_json::from_str::<RawCard>(raw).unwrap());
        assert_eq!(info.tier, Some(3));
        assert_eq!(info.artist.as_deref(), Some("Drawer"));
        assert_eq!(info.artist_login.as_deref(), Some("drawer"));
    }

    #[test]
    fn available_sets_parse() {
        let raw = r#"{"channel":{"self":{"availableEmoteSetsPaginated":{"edges":[
            {"cursor":"c1","node":{"id":"0","owner":null,"emotes":[{"id":"25","setID":"0","token":"Kappa","type":"GLOBALS","assetType":"STATIC","modifiers":null}]}},
            {"cursor":"c2","node":{"id":"19503","owner":{"id":"71092938","login":"xqc","displayName":"xQc","profileImageURL":"a"},
              "emotes":[{"id":"305535174","setID":"19503","token":"xqcOmega","type":"SUBSCRIPTIONS","assetType":"STATIC","modifiers":[{"code":"BW"}]}]}},
            {"cursor":"c3","node":{"id":"300374282","owner":{"id":"139075904","login":"qa_tw_partner","displayName":"q","profileImageURL":null},"emotes":[]}}
          ],"pageInfo":{"hasNextPage":false}}}}}"#;
        let data: RawAvailable = serde_json::from_str(raw).unwrap();
        let page = data
            .channel
            .unwrap()
            .self_
            .unwrap()
            .available_emote_sets_paginated
            .unwrap();
        let sets: Vec<EmoteSet> = page
            .edges
            .into_iter()
            .filter_map(|e| e.node)
            .map(EmoteSet::from)
            .collect();
        assert_eq!(sets[0].emotes[0].kind, EmoteKind::Global);
        assert_eq!(sets[1].owner.as_ref().unwrap().name, "xQc");
        assert_eq!(sets[1].emotes[0].modifiers, ["BW"]);
        assert!(sets[2].owner.is_none());
    }
}
