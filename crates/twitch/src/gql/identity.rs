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
    user(login: $login) { id broadcastBadges { setID } self { selectedBadge { BADGE } availableBadges { BADGE } } } }";

/// Badges nobody picks everywhere: roles and subs show on their own.
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

/// V107-Mx Roles show on their own in a channel too; its sub, founder, Bits and event badges
/// (`mafiathon-3-badge`, `speed-does-america`) are what a channel badge is for.
const ROLES: &[&str] = &[
    "broadcaster",
    "moderator",
    "lead_moderator",
    "vip",
    "staff",
    "admin",
    "global_mod",
    "predictions",
];

/// V107-Mx Sets worn per channel though Twitch lists them as global ones.
const CHANNEL_KIND: &[&str] = &[
    "subscriber",
    "founder",
    "bits",
    "bits-leader",
    "sub-gifter",
    "sub-gift-leader",
    "hype-train",
    "moments",
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

/// The account's color and badge picks, globally and in `login`'s channel. `base`: a mock GQL.
pub async fn chat_identity(http: &Http, auth: Auth<'_>, base: Option<&str>, login: &str) -> Result<ChatIdentity> {
    let q = READ.replace("BADGE", BADGE_FIELDS);
    let data = run_at(http, auth, base, &q, json!({ "login": login })).await?;
    let data: RawRead = serde_json::from_value(data).map_err(Error::Decode)?;
    let me = data.current_user.ok_or_else(signed_out)?;
    let global = RawSelf {
        selected_badge: me.selected_badge,
        available_badges: me.available_badges,
    };
    let (channel_id, channel) = match data.user {
        Some(u) => {
            let sets: Vec<String> = u.broadcast_badges.unwrap_or_default().into_iter().flatten().map(|b| b.set_id).collect();
            (Some(u.id), u.self_.map(|s| channel_pick(s, &global, &sets)).unwrap_or_default())
        }
        None => (None, BadgePick::default()),
    };
    Ok(ChatIdentity {
        color: me.chat_color.filter(|c| !c.is_empty()),
        any_color: me.has_prime || me.has_turbo,
        global: BadgePick::from(global),
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
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    /// The channel's own badge sets (subs, Bits, events).
    #[serde(default)]
    broadcast_badges: Option<Vec<Option<RawSet>>>,
    #[serde(rename = "self")]
    self_: Option<RawSelf>,
}

#[derive(Deserialize)]
struct RawSet {
    #[serde(rename = "setID")]
    set_id: String,
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
        pick(s, |b| !NOT_PICKABLE.contains(&b.set.as_str()))
    }
}

fn same(a: &super::BadgeInfo, b: &super::BadgeInfo) -> bool {
    a.set == b.set && a.version == b.version
}

/// V107-Mu The badge worn always shows as worn, even a role or sub one (Twitch lets you wear
/// them); it is listed too, so something in the row is lit. Others pass `keep` or stay hidden.
fn pick(s: RawSelf, keep: impl Fn(&super::BadgeInfo) -> bool) -> BadgePick {
    let selected = s.selected_badge.map(super::BadgeInfo::from);
    let mut available: Vec<_> = s
        .available_badges
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(super::BadgeInfo::from)
        .filter(|b| keep(b) || selected.as_ref().is_some_and(|w| same(w, b)))
        .collect();
    if let Some(w) = selected.as_ref().filter(|w| !available.iter().any(|b| same(w, b))) {
        available.insert(0, w.clone());
    }
    BadgePick { selected, available }
}

/// V107-Mx What can be worn only in this channel: everything Twitch offers there but roles,
/// subs and founder included. A global badge Twitch repeats here is left to "Everywhere",
/// unless the set is the channel's own (`sets`, its `broadcastBadges`) or a per-channel kind.
fn channel_pick(s: RawSelf, global: &RawSelf, sets: &[String]) -> BadgePick {
    let everywhere: Vec<(&str, &str)> = global
        .available_badges
        .iter()
        .flatten()
        .flatten()
        .map(|b| (b.set_id.as_str(), b.version.as_str()))
        .collect();
    pick(s, |b| {
        let set = b.set.as_str();
        let own = CHANNEL_KIND.contains(&set) || sets.iter().any(|s| s == set);
        !ROLES.contains(&set) && (own || !everywhere.contains(&(set, b.version.as_str())))
    })
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
    fn badge(set: &str, version: &str, title: &str) -> String {
        format!(
            r#"{{"setID":"{set}","version":"{version}","title":"{title}","description":null,"clickURL":null,"image1x":"a","image2x":"b","image4x":"c"}}"#
        )
    }

    fn sets(p: &BadgePick) -> Vec<&str> {
        p.available.iter().map(|b| b.set.as_str()).collect()
    }

    /// V107-Mx Subs, founder, Bits and the channel's event badges are offered; roles and
    /// global badges Twitch repeats here are not, unless the set is the channel's own.
    #[test]
    fn channel_list_keeps_event_badges() {
        let event = badge("mafiathon-3-badge", "1", "Mafiathon 3 Badge");
        let pledge = badge("glhf-pledge", "1", "GLHF Pledge");
        let global: RawSelf =
            serde_json::from_str(&format!(r#"{{"selectedBadge":null,"availableBadges":[{pledge},{event}]}}"#)).unwrap();
        let here = [
            badge("moderator", "1", "Moderator"),
            badge("subscriber", "3012", "Subscriber"),
            badge("founder", "0", "Founder"),
            badge("bits", "1000", "cheer 1000"),
            event.clone(),
            badge("speed-does-america", "1", "Speed Does America Badge"),
            pledge,
        ]
        .join(",");
        let s: RawSelf = serde_json::from_str(&format!(r#"{{"selectedBadge":null,"availableBadges":[{here},null]}}"#)).unwrap();
        let own = ["subscriber".to_owned(), "mafiathon-3-badge".to_owned(), "speed-does-america".to_owned()];
        let pick = channel_pick(s, &global, &own);
        assert_eq!(
            sets(&pick),
            ["subscriber", "founder", "bits", "mafiathon-3-badge", "speed-does-america"]
        );
        // The global list is unchanged: subs and roles stay out of it.
        assert_eq!(sets(&BadgePick::from(global)), ["glhf-pledge", "mafiathon-3-badge"]);
    }

    /// V107-Mx A worn global badge in a channel still shows as worn there.
    #[test]
    fn worn_repeat_still_listed() {
        let pledge = badge("glhf-pledge", "1", "GLHF Pledge");
        let global: RawSelf = serde_json::from_str(&format!(r#"{{"selectedBadge":null,"availableBadges":[{pledge}]}}"#)).unwrap();
        let s: RawSelf =
            serde_json::from_str(&format!(r#"{{"selectedBadge":{pledge},"availableBadges":[{pledge}]}}"#)).unwrap();
        let pick = channel_pick(s, &global, &[]);
        assert_eq!(sets(&pick), ["glhf-pledge"]);
        assert!(pick.selected.is_some());
    }

    #[tokio::test]
    async fn read_asks_for_channel_sets() {
        use crate::gql::dashboard::mock;
        let event = badge("speed-does-america", "1", "Speed Does America Badge");
        let answer = format!(
            r#"{{"data":{{"currentUser":{{"id":"7","chatColor":"","hasPrime":false,"hasTurbo":true,"selectedBadge":null,"availableBadges":[]}},
            "user":{{"id":"9","broadcastBadges":[{{"setID":"subscriber"}},{{"setID":"speed-does-america"}}],
            "self":{{"selectedBadge":{event},"availableBadges":[{event}]}}}}}}}}"#
        );
        let m = mock::serve([answer]);
        let me = chat_identity(&m.http, mock::AUTH, Some(m.base.as_str()), "ishowspeed").await.unwrap();
        assert!(mock::query(&m.sent()[0]).contains("broadcastBadges { setID }"));
        assert_eq!((me.color, me.any_color, me.channel_id.as_deref()), (None, true, Some("9")));
        assert_eq!(sets(&me.channel), ["speed-does-america"]);
        assert_eq!(me.channel.selected.map(|b| b.title), Some("Speed Does America Badge".into()));
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
