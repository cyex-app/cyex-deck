//! 1.40 How you show up in chat: the badge you wear (everywhere, or in one channel) and
//! your name color. Needs the account's GQL token. Badge mutation shapes checked against
//! Twitch's schema (2026/10/10); the color one is twitch.tv's own `/color` (V107-Mk).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::chat::{BADGE_FIELDS, RawBadge};
use super::{Auth, query, run_at};
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
    /// V107-Mk Smiley style; no choices when the account has none to pick.
    pub smilies: super::smilies::Smilies,
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
        smilies: super::smilies::Smilies::default(),
    })
}

/// Wears `badge` (`set`, `version`) everywhere, or nothing with `None`. `base`: a mock GQL.
pub async fn pick_global_badge(http: &Http, auth: Auth<'_>, base: Option<&str>, badge: Option<(&str, &str)>) -> Result<()> {
    let (q, vars) = match badge {
        Some((set, version)) => (
            "mutation($input: SelectGlobalBadgeInput!) { selectGlobalBadge(input: $input) { user { id } } }",
            json!({ "input": { "badgeSetID": set, "badgeSetVersion": version } }),
        ),
        None => ("mutation { deselectGlobalBadge { user { id } } }", json!({})),
    };
    run_at(http, auth, base, q, vars).await?;
    Ok(())
}

/// Wears `badge` in one channel, or goes back to the global pick with `None`. `Ok(false)`:
/// Twitch ran it and said no.
pub async fn pick_channel_badge(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    channel_id: &str,
    badge: Option<(&str, &str)>,
) -> Result<bool> {
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
    let v = run_at(http, auth, base, q, vars).await?;
    Ok(v.pointer("/selectChannelBadge/isSuccessful") != Some(&Value::Bool(false)))
}

/// Why Twitch refused a name color (`updateChatColorV2`'s `error.code`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorRefusal {
    Invalid,
    /// Prime or Turbo only.
    TurboOnly,
    Other(String),
}

/// V107-Mk `#RRGGBB` through `updateChatColorV2`, as twitch.tv's `/color`: named ones for
/// everyone, any color with Prime or Turbo. `Ok(Err(_))` is Twitch saying no.
pub async fn set_chat_color(
    http: &Http,
    auth: Auth<'_>,
    base: Option<&str>,
    color: &str,
) -> Result<std::result::Result<(), ColorRefusal>> {
    let v = run_at(
        http,
        auth,
        base,
        "mutation Chat_UpdateChatColorV2($input: UpdateChatColorV2Input!) { updateChatColorV2(input: $input) { user { id chatColor } error { code } } }",
        json!({ "input": { "color": color } }),
    )
    .await?;
    Ok(match v.pointer("/updateChatColorV2/error/code").and_then(Value::as_str) {
        None => Ok(()),
        Some("INVALID_COLOR") => Err(ColorRefusal::Invalid),
        Some("TURBO_ONLY_COLOR") => Err(ColorRefusal::TurboOnly),
        Some(code) => Err(ColorRefusal::Other(code.to_owned())),
    })
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
    /// V107-Mu The badge worn always shows as worn, even a role or sub one (Twitch lets you
    /// wear them); it is listed too, so something in the row is lit. Others stay hidden.
    fn from(s: RawSelf) -> Self {
        let pickable = |b: &super::BadgeInfo| !NOT_PICKABLE.contains(&b.set.as_str());
        let selected = s.selected_badge.map(super::BadgeInfo::from);
        let mut available: Vec<_> = s
            .available_badges
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(super::BadgeInfo::from)
            .filter(|b| pickable(b) || selected.as_ref().is_some_and(|w| w.set == b.set && w.version == b.version))
            .collect();
        if let Some(w) = selected.as_ref().filter(|w| !available.iter().any(|b| b.set == w.set && b.version == w.version)) {
            available.insert(0, w.clone());
        }
        Self { selected, available }
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
    #[test]
    fn worn_sub_badge_shows_as_worn() {
        let sub = r#"{"setID":"subscriber","version":"12","title":"Subscriber","description":null,"clickURL":null,"image1x":"a","image2x":"b","image4x":"c"}"#;
        let other = r#"{"setID":"subscriber","version":"0","title":"Subscriber","description":null,"clickURL":null,"image1x":"a","image2x":"b","image4x":"c"}"#;
        let raw = format!(r#"{{"selectedBadge":{sub},"availableBadges":[{other},{sub}]}}"#);
        let pick = BadgePick::from(serde_json::from_str::<RawSelf>(&raw).unwrap());
        assert_eq!(pick.selected.as_ref().map(|b| b.version.as_str()), Some("12"));
        assert_eq!(pick.available.len(), 1);
        // Not in the list Twitch sent: still listed, so it can be seen and un-picked.
        let raw = format!(r#"{{"selectedBadge":{sub},"availableBadges":[]}}"#);
        let pick = BadgePick::from(serde_json::from_str::<RawSelf>(&raw).unwrap());
        assert_eq!(pick.available, vec![pick.selected.clone().unwrap()]);
    }
    #[tokio::test]
    async fn color_refusals_and_badge_writes() {
        use crate::gql::dashboard::mock;
        let m = mock::serve([
            r##"{"data":{"updateChatColorV2":{"user":{"id":"7","chatColor":"#1E90FF"},"error":null}}}"##,
            r#"{"data":{"updateChatColorV2":{"user":null,"error":{"code":"TURBO_ONLY_COLOR"}}}}"#,
            r#"{"data":{"updateChatColorV2":{"user":null,"error":{"code":"INVALID_COLOR"}}}}"#,
            r#"{"data":{"selectChannelBadge":{"isSuccessful":false,"user":null}}}"#,
            r#"{"data":{"deselectGlobalBadge":{"user":{"id":"7"}}}}"#,
        ]);
        let base = Some(m.base.as_str());
        let (h, a) = (&m.http, mock::AUTH);
        assert_eq!(set_chat_color(h, a, base, "#1E90FF").await.unwrap(), Ok(()));
        assert_eq!(set_chat_color(h, a, base, "#123456").await.unwrap(), Err(ColorRefusal::TurboOnly));
        assert_eq!(set_chat_color(h, a, base, "#12").await.unwrap(), Err(ColorRefusal::Invalid));
        assert!(!pick_channel_badge(h, a, base, "9", Some(("glhf-pledge", "1"))).await.unwrap());
        pick_global_badge(h, a, base, None).await.unwrap();
        let sent = m.sent();
        assert!(mock::query(&sent[0]).contains("updateChatColorV2(input: $input)"));
        assert_eq!(sent[0]["variables"], json!({ "input": { "color": "#1E90FF" } }));
        assert_eq!(
            sent[3]["variables"],
            json!({ "input": { "badgeSetID": "glhf-pledge", "badgeSetVersion": "1", "channelID": "9" } })
        );
        assert!(mock::query(&sent[4]).contains("deselectGlobalBadge"));
    }
}
