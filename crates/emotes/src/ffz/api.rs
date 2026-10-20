//! FFZ over HTTP (1.39): global and room emotes with modifiers (C7), room mod and VIP
//! badges, and FFZ's own badges. FFZ has no live updates in use.

use std::collections::HashMap;

use cyex_chat::model::Provider;
use cyex_twitch::pace::{Lane, Opts, Service};
use cyex_twitch::{Http, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::cosmetics::BadgeDef;
use crate::model::{Emote, EmoteSet, Scope};

const API: &str = "https://api.frankerfacez.com/v1";

pub struct Global {
    /// For everyone.
    pub sets: Vec<EmoteSet>,
    /// Sets only some users may use (C9), with their Twitch ids.
    pub personal: Vec<(EmoteSet, Vec<String>)>,
}

pub async fn global(http: &Http) -> Result<Global> {
    let raw: RawGlobal = http
        .get_json_paced(Service::Ffz, Lane::Front, &format!("{API}/set/global/ids"))
        .await?;
    Ok(raw.into())
}

/// A room's pictures for its mods and VIPs, replacing Twitch's.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomBadges {
    pub moderator: Option<BadgeDef>,
    pub vip: Option<BadgeDef>,
}

pub struct Room {
    pub set: Option<EmoteSet>,
    pub badges: RoomBadges,
}

/// `None` when the channel has no FFZ room (404 or 500, B7: not retried). Paced
/// (V107-P2); S2 in the background it fails fast, the emote retry queue asks again.
pub async fn room(http: &Http, twitch_id: &str, lane: Lane) -> Result<Option<Room>> {
    let opts = Opts {
        lane,
        quick: true,
        answers: &[500],
    };
    match http
        .get_json_paced_with::<RawRoomData>(Service::Ffz, opts, &format!("{API}/room/id/{twitch_id}"))
        .await
    {
        Ok(r) => Ok(Some(r.into())),
        Err(cyex_twitch::Error::Api {
            status: 404 | 500, ..
        }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// FFZ's badges and who has them, as sorted Twitch ids (40.000+ people, kept small).
pub async fn badges(http: &Http) -> Result<(Vec<BadgeDef>, Vec<(String, Vec<u64>)>)> {
    let raw: RawBadges = http
        .get_json_paced(Service::Ffz, Lane::Front, &format!("{API}/badges/ids"))
        .await?;
    Ok(raw.into())
}

#[derive(Deserialize)]
struct RawGlobal {
    default_sets: Vec<u64>,
    sets: HashMap<String, RawSet>,
    #[serde(default)]
    user_ids: HashMap<String, Vec<u64>>,
}

#[derive(Deserialize)]
struct RawSet {
    id: u64,
    #[serde(default)]
    emoticons: Vec<RawEmote>,
}

#[derive(Deserialize)]
struct RawEmote {
    id: u64,
    name: String,
    width: Option<u16>,
    height: Option<u16>,
    #[serde(default)]
    modifier: bool,
    #[serde(default)]
    modifier_flags: u32,
    owner: Option<RawOwner>,
    animated: Option<serde_json::Value>,
    /// 1.41 Upload date (ISO), shown as the card's Added date.
    #[serde(default)]
    created_at: Option<String>,
}

#[derive(Deserialize)]
struct RawOwner {
    /// Login, in the owner's page link.
    #[serde(default)]
    name: Option<String>,
    display_name: String,
}

impl RawEmote {
    fn emote(self) -> Emote {
        let mut e = Emote::new(Provider::Ffz, self.id.to_string(), self.name);
        e.modifier = self.modifier.then_some(self.modifier_flags);
        e.animated = self.animated.is_some_and(|a| a.is_object());
        e.size = self.width.zip(self.height);
        e.owner_key = self.owner.as_ref().and_then(|o| o.name.clone());
        e.owner = self.owner.map(|o| o.display_name);
        e.added_at = self.created_at;
        e
    }
}

impl RawSet {
    fn into_set(self, scope: Scope) -> EmoteSet {
        EmoteSet::new(
            format!("ffz-{}", self.id),
            Provider::Ffz,
            scope,
            self.emoticons.into_iter().map(RawEmote::emote).collect(),
        )
    }
}

impl From<RawGlobal> for Global {
    fn from(mut g: RawGlobal) -> Self {
        let mut sets = Vec::new();
        for id in &g.default_sets {
            if let Some(s) = g.sets.remove(&id.to_string()) {
                sets.push(s.into_set(Scope::Global));
            }
        }
        let personal = g
            .user_ids
            .into_iter()
            .filter_map(|(set, users)| {
                let s = g.sets.remove(&set)?;
                Some((
                    s.into_set(Scope::Personal),
                    users.into_iter().map(|u| u.to_string()).collect(),
                ))
            })
            .collect();
        Global { sets, personal }
    }
}

#[derive(Deserialize)]
struct RawRoomData {
    room: RawRoom,
    #[serde(default)]
    sets: HashMap<String, RawSet>,
}

#[derive(Deserialize)]
struct RawRoom {
    set: Option<u64>,
    mod_urls: Option<HashMap<String, String>>,
    vip_badge: Option<HashMap<String, String>>,
}

/// A badge from FFZ's `{1, 2, 4}` URLs.
fn room_badge(
    kind: &str,
    title: &str,
    urls: &HashMap<String, String>,
    color: Option<&str>,
) -> Option<BadgeDef> {
    let one = urls.get("1")?.clone();
    let two = urls.get("2").cloned().unwrap_or_else(|| one.clone());
    let four = urls.get("4").cloned().unwrap_or_else(|| two.clone());
    Some(BadgeDef {
        provider: Provider::Ffz,
        id: kind.to_owned(),
        title: title.to_owned(),
        image1x: one,
        image2x: two,
        image4x: four,
        still: None,
        color: color.map(str::to_owned),
        replaces: Some(kind.to_owned()),
    })
}

impl From<RawRoomData> for Room {
    fn from(mut r: RawRoomData) -> Self {
        let set = r
            .room
            .set
            .and_then(|id| r.sets.remove(&id.to_string()))
            .map(|s| s.into_set(Scope::Channel));
        Room {
            set,
            badges: RoomBadges {
                // FFZ draws the room's mod badge on Twitch's mod green.
                moderator: r
                    .room
                    .mod_urls
                    .as_ref()
                    .and_then(|u| room_badge("moderator", "Moderator", u, Some("#34ae0a"))),
                vip: r
                    .room
                    .vip_badge
                    .as_ref()
                    .and_then(|u| room_badge("vip", "VIP", u, None)),
            },
        }
    }
}

#[derive(Deserialize)]
struct RawBadges {
    badges: Vec<RawBadge>,
    #[serde(default)]
    users: HashMap<String, Vec<u64>>,
}

#[derive(Deserialize)]
struct RawBadge {
    id: u64,
    title: String,
    color: Option<String>,
    replaces: Option<String>,
    urls: HashMap<String, String>,
}

impl From<RawBadges> for (Vec<BadgeDef>, Vec<(String, Vec<u64>)>) {
    fn from(b: RawBadges) -> Self {
        let defs = b
            .badges
            .into_iter()
            .filter_map(|r| {
                let mut def = room_badge(&r.id.to_string(), &r.title, &r.urls, r.color.as_deref())?;
                def.replaces = r.replaces;
                Some(def)
            })
            .collect();
        let users = b
            .users
            .into_iter()
            .map(|(id, mut ids)| {
                ids.sort_unstable();
                ids.dedup();
                (id, ids)
            })
            .collect();
        (defs, users)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
        let path = format!("{}/tests/fixtures/ffz/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn global_sets_and_subwoofer_users() {
        let g: Global = fixture::<RawGlobal>("set_global_ids.json").into();
        assert!(!g.sets.is_empty());
        let (set, users) = &g.personal[0];
        assert!(!users.is_empty());
        let hyper = set.get("ffzHyper").unwrap();
        assert_eq!(hyper.modifier, Some(12289));
    }

    #[test]
    fn room_has_emotes_and_badges() {
        let r: Room = fixture::<RawRoomData>("room.json").into();
        let set = r.set.unwrap();
        let laugh = set.get("forsenLaughingAtYou").unwrap();
        assert!(laugh.animated);
        assert_eq!(laugh.size, Some((28, 28)));
        assert_eq!(laugh.owner_key.as_deref(), Some("sadgw"));
        assert!(laugh.added_at.as_deref().is_some_and(|d| d.starts_with("20")));
        assert_eq!(r.badges.moderator.unwrap().color.as_deref(), Some("#34ae0a"));
        assert!(r.badges.vip.unwrap().image4x.ends_with("/4"));
    }

    #[test]
    fn badges_keep_users_sorted() {
        let (defs, users): (Vec<BadgeDef>, Vec<(String, Vec<u64>)>) =
            fixture::<RawBadges>("badges_ids.json").into();
        let bot = defs.iter().find(|d| d.id == "2").unwrap();
        assert_eq!(bot.replaces.as_deref(), Some("moderator"));
        assert!(users.iter().all(|(_, ids)| ids.windows(2).all(|w| w[0] < w[1])));
    }
}
