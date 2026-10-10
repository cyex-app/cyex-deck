//! 1.40 How you show up in chat: the badge you wear (everywhere, or in one channel) and
//! your name color. Needs the account's GQL token. Mutation shapes checked against
//! Twitch's schema without running them (2026/10/10).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::chat::{BADGE_FIELDS, RawBadge};
use super::{Auth, query};
use crate::{Error, Http, Result};

const READ: &str = "query($login: String!) { \
    currentUser { id chatColor hasPrime hasTurbo selectedBadge { BADGE } availableBadges { BADGE } } \
    user(login: $login) { id self { selectedBadge { BADGE } availableBadges { BADGE } } } }";

/// Badges nobody picks: roles and subs show on their own.
const NOT_PICKABLE: &[&str] = &[
    "broadcaster",
    "moderator",
    "lead_moderator",
    "vip",
    "subscriber",
    "founder",
    "staff",
    "admin",
    "global_mod",
    "predictions",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatIdentity {
    /// `#RRGGBB`, `None` if never set.
    pub color: Option<String>,
    /// Prime or Turbo can pick any color; others pick from Twitch's named ones.
    pub any_color: bool,
    /// Shown in every channel.
    pub global: BadgePick,
    /// Shown in this channel instead of the global pick.
    pub channel: BadgePick,
    pub channel_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BadgePick {
    pub selected: Option<super::BadgeInfo>,
    pub available: Vec<super::BadgeInfo>,
}

/// The account's color and badge picks, globally and in `login`'s channel.
pub async fn chat_identity(http: &Http, auth: Auth<'_>, login: &str) -> Result<ChatIdentity> {
    let q = READ.replace("BADGE", BADGE_FIELDS);
    let data: RawRead = query(http, auth, &q, json!({ "login": login })).await?;
    let me = data.current_user.ok_or_else(signed_out)?;
    let (channel_id, channel) = match data.user {
        Some(u) => (Some(u.id), u.self_.map(BadgePick::from).unwrap_or_default()),
        None => (None, BadgePick::default()),
    };
    Ok(ChatIdentity {
        color: me.chat_color.filter(|c| !c.is_empty()),
        any_color: me.has_prime || me.has_turbo,
        global: BadgePick::from(RawSelf {
            selected_badge: me.selected_badge,
            available_badges: me.available_badges,
        }),
        channel,
        channel_id,
    })
}

/// Wears `badge` (`set`, `version`) everywhere, or nothing with `None`.
pub async fn pick_global_badge(http: &Http, auth: Auth<'_>, badge: Option<(&str, &str)>) -> Result<()> {
    let (q, vars) = match badge {
        Some((set, version)) => (
            "mutation($input: SelectGlobalBadgeInput!) { selectGlobalBadge(input: $input) { user { id } } }",
            json!({ "input": { "badgeSetID": set, "badgeSetVersion": version } }),
        ),
        None => ("mutation { deselectGlobalBadge { user { id } } }", json!({})),
    };
    let _: Value = query(http, auth, q, vars).await?;
    Ok(())
}

/// Wears `badge` in one channel, or goes back to the global pick with `None`.
pub async fn pick_channel_badge(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    badge: Option<(&str, &str)>,
) -> Result<()> {
    let (q, vars) = match badge {
        Some((set, version)) => (
            "mutation($input: SelectChannelBadgeInput!) { selectChannelBadge(input: $input) { isSuccessful user { id } } }",
            json!({ "input": { "badgeSetID": set, "badgeSetVersion": version, "channelID": channel_id } }),
        ),
        None => (
            "mutation($input: DeselectChannelBadgeInput!) { deselectChannelBadge(input: $input) { user { id } } }",
            json!({ "input": { "channelID": channel_id } }),
        ),
    };
    let v: Value = query(http, auth, q, vars).await?;
    if v.pointer("/selectChannelBadge/isSuccessful") == Some(&Value::Bool(false)) {
        return Err(Error::Api {
            status: 200,
            message: "Twitch didn't take that badge.".into(),
        });
    }
    Ok(())
}

/// `#RRGGBB`. Twitch refuses colors outside its named ones without Prime or Turbo.
pub async fn set_chat_color(http: &Http, auth: Auth<'_>, color: &str) -> Result<()> {
    let _: Value = query(
        http,
        auth,
        "mutation($input: UpdateChatColorInput!) { updateChatColor(input: $input) { user { id } } }",
        json!({ "input": { "color": color } }),
    )
    .await?;
    Ok(())
}

/// Anyone's chat color, readable signed out. `None` if they never set one.
pub async fn chat_color(http: &Http, user_id: &str) -> Result<Option<String>> {
    let v: Value = query(
        http,
        Auth::PUBLIC,
        "query($id: ID!) { user(id: $id) { chatColor } }",
        json!({ "id": user_id }),
    )
    .await?;
    Ok(v.pointer("/user/chatColor")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty())
        .map(str::to_owned))
}

fn signed_out() -> Error {
    Error::Api {
        status: 401,
        message: "not signed in".into(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRead {
    current_user: Option<RawMe>,
    user: Option<RawUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMe {
    chat_color: Option<String>,
    #[serde(default)]
    has_prime: bool,
    #[serde(default)]
    has_turbo: bool,
    selected_badge: Option<RawBadge>,
    #[serde(default)]
    available_badges: Option<Vec<Option<RawBadge>>>,
}

#[derive(Deserialize)]
struct RawUser {
    id: String,
    #[serde(rename = "self")]
    self_: Option<RawSelf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSelf {
    selected_badge: Option<RawBadge>,
    #[serde(default)]
    available_badges: Option<Vec<Option<RawBadge>>>,
}

impl From<RawSelf> for BadgePick {
    fn from(s: RawSelf) -> Self {
        let pickable = |b: &super::BadgeInfo| !NOT_PICKABLE.contains(&b.set.as_str());
        Self {
            selected: s.selected_badge.map(super::BadgeInfo::from).filter(pickable),
            available: s
                .available_badges
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .map(super::BadgeInfo::from)
                .filter(pickable)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_not_offered() {
        let raw = r#"{"selectedBadge":null,"availableBadges":[
            {"setID":"moderator","version":"1","title":"Moderator","description":null,"clickURL":null,"image1x":"a","image2x":"b","image4x":"c"},
            {"setID":"glhf-pledge","version":"1","title":"GLHF Pledge","description":"","clickURL":null,"image1x":"a","image2x":"b","image4x":"c"},
            null]}"#;
        let pick = BadgePick::from(serde_json::from_str::<RawSelf>(raw).unwrap());
        assert_eq!(pick.available.len(), 1);
        assert_eq!(pick.available[0].set, "glhf-pledge");
    }
}
