//! 1.48 Chat modes (`moderator:manage:chat_settings`), announcements and shoutouts
//! (1.49). Followers-only and slow mode carry their minutes and seconds.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use ts_rs::TS;

use super::{Auth, Data, Room, get, post, request};
use crate::http::{empty, json};
use crate::{Http, Result};

/// Twitch's slow mode range, in seconds.
pub const SLOW_RANGE: (u32, u32) = (3, 120);
/// Followers-only, in minutes: up to three months.
pub const MAX_FOLLOW_MINUTES: u32 = 129_600;

/// The modes Twitch has on for a channel. Fields Twitch leaves out mean off.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all(serialize = "camelCase"))]
#[ts(rename_all = "camelCase", export)]
pub struct ChatModes {
    #[serde(default)]
    pub emote_mode: bool,
    #[serde(default)]
    pub follower_mode: bool,
    /// Minutes.
    #[serde(default)]
    pub follower_mode_duration: Option<u32>,
    #[serde(default)]
    pub slow_mode: bool,
    /// Seconds.
    #[serde(default)]
    pub slow_mode_wait_time: Option<u32>,
    #[serde(default)]
    pub subscriber_mode: bool,
    #[serde(default)]
    pub unique_chat_mode: bool,
    /// Only sent to mods: messages from others show this late.
    #[serde(default)]
    pub non_moderator_chat_delay: Option<bool>,
    /// 2, 4 or 6 seconds.
    #[serde(default)]
    pub non_moderator_chat_delay_duration: Option<u32>,
}

/// One change to the chat modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeChange {
    Emote(bool),
    /// Minimum follow age in minutes; `None` = off.
    Followers(Option<u32>),
    /// Seconds between messages; `None` = off.
    Slow(Option<u32>),
    Subs(bool),
    Unique(bool),
    /// Seconds (2, 4, 6) others' messages are held; `None` = off.
    Delay(Option<u32>),
}

impl ModeChange {
    fn body(self) -> Map<String, Value> {
        let mut m = Map::new();
        let mut put = |k: &str, v: Value| {
            m.insert(k.to_owned(), v);
        };
        match self {
            ModeChange::Emote(on) => put("emote_mode", on.into()),
            ModeChange::Subs(on) => put("subscriber_mode", on.into()),
            ModeChange::Unique(on) => put("unique_chat_mode", on.into()),
            ModeChange::Followers(min) => {
                put("follower_mode", min.is_some().into());
                if let Some(m) = min {
                    put("follower_mode_duration", m.min(MAX_FOLLOW_MINUTES).into());
                }
            }
            ModeChange::Slow(secs) => {
                put("slow_mode", secs.is_some().into());
                if let Some(s) = secs {
                    put("slow_mode_wait_time", s.clamp(SLOW_RANGE.0, SLOW_RANGE.1).into());
                }
            }
            ModeChange::Delay(secs) => {
                put("non_moderator_chat_delay", secs.is_some().into());
                if let Some(s) = secs {
                    let s = [2, 4, 6].into_iter().min_by_key(|v: &u32| v.abs_diff(s)).unwrap_or(2);
                    put("non_moderator_chat_delay_duration", s.into());
                }
            }
        }
        m
    }
}

pub async fn modes(http: &Http, auth: Auth<'_>, room: Room<'_>) -> Result<ChatModes> {
    let req = get(http, auth, "/chat/settings").query(&room.query());
    let d: Data<ChatModes> = json(req).await?;
    Ok(d.data.into_iter().next().unwrap_or_default())
}

pub async fn set_mode(http: &Http, auth: Auth<'_>, room: Room<'_>, change: ModeChange) -> Result<ChatModes> {
    let req = request(http, auth, Method::PATCH, "/chat/settings")
        .query(&room.query())
        .json(&Value::Object(change.body()));
    let d: Data<ChatModes> = json(req).await?;
    Ok(d.data.into_iter().next().unwrap_or_default())
}

/// Announcement colors Twitch knows.
pub const ANNOUNCE_COLORS: [&str; 5] = ["primary", "blue", "green", "orange", "purple"];

pub async fn announce(http: &Http, auth: Auth<'_>, room: Room<'_>, message: &str, color: &str) -> Result<()> {
    let color = if ANNOUNCE_COLORS.contains(&color) { color } else { "primary" };
    let req = post(http, auth, "/chat/announcements")
        .query(&room.query())
        .json(&json!({ "message": message, "color": color }));
    empty(req).await
}

pub async fn shoutout(http: &Http, auth: Auth<'_>, room: Room<'_>, to_id: &str) -> Result<()> {
    let req = post(http, auth, "/chat/shoutouts").query(&[
        ("from_broadcaster_id", room.broadcaster_id),
        ("to_broadcaster_id", to_id),
        ("moderator_id", room.moderator_id),
    ]);
    empty(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_parse() {
        let d: Data<ChatModes> = serde_json::from_str(
            r#"{"data":[{"broadcaster_id":"1","slow_mode":true,"slow_mode_wait_time":30,"follower_mode":true,
            "follower_mode_duration":0,"subscriber_mode":false,"emote_mode":false,"unique_chat_mode":false,
            "non_moderator_chat_delay":true,"non_moderator_chat_delay_duration":4}]}"#,
        )
        .unwrap();
        let m = &d.data[0];
        assert!(m.slow_mode && m.follower_mode && !m.emote_mode);
        assert_eq!((m.slow_mode_wait_time, m.follower_mode_duration), (Some(30), Some(0)));
        assert_eq!(m.non_moderator_chat_delay_duration, Some(4));
    }

    #[test]
    fn changes_clamp() {
        let b = ModeChange::Slow(Some(500)).body();
        assert_eq!(b["slow_mode_wait_time"], 120);
        let b = ModeChange::Followers(None).body();
        assert_eq!(b["follower_mode"], false);
        assert!(!b.contains_key("follower_mode_duration"));
        assert_eq!(ModeChange::Delay(Some(5)).body()["non_moderator_chat_delay_duration"], 4);
    }
}
