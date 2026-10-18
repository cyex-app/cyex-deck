//! 1.43 Sending chat GIFs (Tier 2 and 3 subscribers where the channel allows them).
//! Twitch hands out its GIPHY key with the picker settings; it stays in Rust.

use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{Auth, persisted_or_raw, query};
use crate::{Http, Result};

const ACCESS: &str = "query($id: ID!) { gifPickerConfig(channelID: $id) { isEnabled isAllowlisted apiKey contentRating } \
    user(id: $id) { subscriptionProducts { tier hasGifs } self { subscriptionBenefit { tier } } } }";

const SEND: (&str, &str, &str) = (
    "sendGifMessage",
    "1b7a15ea1afa85371aa360c47b4c26d49c995ef6fb02902f6fb9f1ed8a5f9e45",
    "mutation sendGifMessage($input: SendGifMessageInput!) { sendGifMessage(input: $input) { error secondsUntilCanSend message { id } } }",
);

/// Whether the account can send GIFs in a channel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GifAccess {
    pub can_send: bool,
    /// Why not, ready to show.
    pub reason: Option<&'static str>,
    pub api_key: Option<String>,
    /// GIPHY rating: `g`, `pg` or `pg-13`.
    pub rating: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GifSent {
    /// Twitch's code when it didn't go through.
    pub error: Option<String>,
    /// Wait this long before the next one.
    pub wait_secs: Option<u32>,
}

/// `account` is the signed-in user's id: the broadcaster can post in their own chat.
pub async fn gif_access(http: &Http, auth: Auth<'_>, channel_id: &str, account: &str) -> Result<GifAccess> {
    let d: RawData = query(http, auth, ACCESS, json!({ "id": channel_id })).await?;
    Ok(decide(d, account == channel_id))
}

/// What the room says right now; not cached with the access.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoomGate {
    pub emote_only: bool,
    /// Shield Mode turns the picker off (`isGifPickerDisabled`).
    pub shield: bool,
}

impl GifAccess {
    /// 1.43 Twitch's picker is off in emote-only mode and in Shield Mode.
    pub fn in_room(mut self, room: RoomGate) -> Self {
        let reason = if room.emote_only {
            Some("No GIFs in emote-only mode.")
        } else if room.shield {
            Some("No GIFs while Shield Mode is on.")
        } else {
            None
        };
        if reason.is_some() {
            self.can_send = false;
            self.reason = reason;
        }
        self
    }
}

fn decide(d: RawData, broadcaster: bool) -> GifAccess {
    let channel_has = d
        .user
        .as_ref()
        .is_some_and(|u| u.subscription_products.iter().any(|p| p.has_gifs));
    let tier = d
        .user
        .as_ref()
        .and_then(|u| u.me.as_ref())
        .and_then(|s| s.subscription_benefit.as_ref())
        .and_then(|b| b.tier.parse::<u32>().ok())
        .unwrap_or(0);
    let cfg = d.gif_picker_config.unwrap_or_default();
    let reason = if !channel_has {
        Some("This channel doesn’t allow GIFs.")
    } else if !(cfg.is_enabled && cfg.is_allowlisted) {
        Some("GIFs are unavailable right now.")
    } else if tier < 2000 && !broadcaster {
        Some("Subscribe at Tier 2 or 3 to send GIFs here.")
    } else {
        None
    };
    GifAccess {
        can_send: reason.is_none() && cfg.api_key.is_some(),
        reason,
        api_key: cfg.api_key,
        rating: match cfg.content_rating.as_deref() {
            Some("PG_13") => "pg-13",
            Some("G_PG") => "pg",
            _ => "g",
        },
    }
}

pub async fn send_gif(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    gif_id: &str,
    gif_url: &str,
    search: &str,
) -> Result<GifSent> {
    let mut input = json!({ "channelID": channel_id, "gifID": gif_id, "gifURL": gif_url });
    if !search.is_empty() {
        input["searchTerm"] = json!(search);
    }
    let v = persisted_or_raw(http, auth, SEND, json!({ "input": input })).await?;
    let r = &v["sendGifMessage"];
    Ok(GifSent {
        error: r["error"].as_str().map(str::to_owned),
        wait_secs: r["secondsUntilCanSend"]
            .as_u64()
            .map(|s| s as u32)
            .filter(|s| *s > 0),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawData {
    gif_picker_config: Option<RawConfig>,
    user: Option<RawUser>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawConfig {
    is_enabled: bool,
    is_allowlisted: bool,
    api_key: Option<String>,
    content_rating: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    #[serde(default)]
    subscription_products: Vec<RawProduct>,
    #[serde(rename = "self")]
    me: Option<RawSelf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProduct {
    has_gifs: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSelf {
    subscription_benefit: Option<RawBenefit>,
}

#[derive(Deserialize)]
struct RawBenefit {
    tier: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(name: &str) -> RawData {
        let raw = match name {
            "t2" => include_str!("../../tests/fixtures/gifs/access_t2.json"),
            "t1" => include_str!("../../tests/fixtures/gifs/access_t1.json"),
            "owner" => include_str!("../../tests/fixtures/gifs/access_owner.json"),
            "not_allowlisted" => include_str!("../../tests/fixtures/gifs/access_not_allowlisted.json"),
            _ => include_str!("../../tests/fixtures/gifs/access_off.json"),
        };
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn who_can_send() {
        let ok = decide(data("t2"), false);
        assert!(ok.can_send);
        assert_eq!(ok.rating, "pg");
        let tier1 = decide(data("t1"), false);
        assert!(!tier1.can_send);
        assert_eq!(tier1.reason, Some("Subscribe at Tier 2 or 3 to send GIFs here."));
        let off = decide(data("off"), false);
        assert_eq!(off.reason, Some("This channel doesn’t allow GIFs."));
        let listed = decide(data("not_allowlisted"), false);
        assert!(!listed.can_send);
        assert_eq!(listed.reason, Some("GIFs are unavailable right now."));
    }

    #[test]
    fn the_broadcaster_needs_no_sub() {
        assert!(!decide(data("owner"), false).can_send);
        let own = decide(data("owner"), true);
        assert!(own.can_send);
        assert_eq!(own.rating, "pg-13");
        // Still only where the channel has GIFs.
        assert!(!decide(data("off"), true).can_send);
    }

    #[test]
    fn emote_only_and_shield_turn_it_off() {
        let ok = decide(data("t2"), false);
        assert!(ok.clone().in_room(RoomGate::default()).can_send);
        let emote_only = ok.clone().in_room(RoomGate {
            emote_only: true,
            shield: false,
        });
        assert!(!emote_only.can_send);
        assert_eq!(emote_only.reason, Some("No GIFs in emote-only mode."));
        let shield = ok.in_room(RoomGate {
            emote_only: false,
            shield: true,
        });
        assert!(!shield.can_send);
        assert_eq!(shield.reason, Some("No GIFs while Shield Mode is on."));
        // The account's own reason stays when the room is fine.
        let t1 = decide(data("t1"), false).in_room(RoomGate::default());
        assert_eq!(t1.reason, Some("Subscribe at Tier 2 or 3 to send GIFs here."));
    }
}
