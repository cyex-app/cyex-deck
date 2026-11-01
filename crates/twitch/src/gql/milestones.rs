//! V108-N23 The account's viewer milestones in one channel (the watch streak among them)
//! and sharing one in chat. Found in twitch.tv's bundles and checked with wrong-typed
//! validation probes 2026/10/29; the mutation was never run.
//! - Read: `user(login) { channel { id self { viewerMilestones { id category value state
//!   shareStatus } } } }` (null signed out).
//! - Share: `shareViewerMilestone(input: {channelID, milestoneID}) { milestone { id }
//!   error { code } }`. Twitch then posts the `viewermilestone` USERNOTICE itself.
//! - Enums (bundle, not seen live): `category` has `WATCH_STREAK`; `shareStatus` is
//!   `CAN_SHARE | CAN_NOT_SHARE | DO_NOT_SHARE | SHARED | UNKNOWN`. `state` is kept as text.
//! - Live: PubSub `viewer-milestones.<user id>` (and `.<user id>.<channel id>`), events
//!   `viewer-milestones-update` (`data.event_type`, `channel_id`, `watch_streak_value`),
//!   `watch-streak`, `streak-recovered`. See [`topic`]. twitch.tv's own left nav only acts on
//!   `event_type` `achieved` (bundle 2026/11/01); a share fires an update too, so other
//!   event types must not re-offer a streak.
//! - `state` is `WatchStreakStateEnum` (validation error 2026/11/01); the bundle's only enum of
//!   that shape is `ACTIVE | NONE | SUSPENDED`. twitch.tv's streak button calls a streak
//!   shareable only once its value reaches the next threshold, so a `CAN_SHARE` milestone alone
//!   isn't proof: [`pick`] takes the smallest one, never above a known current count.

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, query};
use crate::{Error, Http, Result};

/// The category of the watch streak milestone.
pub const WATCH_STREAK: &str = "WATCH_STREAK";

/// One milestone the account has in a channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ViewerMilestone")]
pub struct Milestone {
    pub id: String,
    /// Twitch's category text, e.g. `WATCH_STREAK`.
    pub category: String,
    /// The number it counts (streams in a row for a streak).
    pub value: u32,
    pub state: String,
    /// `CAN_SHARE`, `SHARED`, ...; text on purpose, Twitch adds values.
    pub share_status: String,
}

impl Milestone {
    pub fn is_streak(&self) -> bool {
        self.category == WATCH_STREAK
    }

    /// Twitch offers the share button for this one.
    pub fn can_share(&self) -> bool {
        self.share_status == "CAN_SHARE"
    }
}

/// The PubSub topic for the account's milestones (`viewer-milestones.<user id>`), or the
/// one narrowed to a channel. Unverified live: the family and suffixes come from the bundle.
pub fn topic(user_id: &str, channel_id: Option<&str>) -> String {
    match channel_id {
        Some(c) => format!("viewer-milestones.{user_id}.{c}"),
        None => format!("viewer-milestones.{user_id}"),
    }
}

/// What a milestones message says: a streak was reached or kept in a channel, with the new
/// count when it says one. `None` for anything else, a share's own update among them.
pub fn update_of(payload: &Value) -> Option<(String, Option<u32>)> {
    let data = payload.get("data")?;
    let reached = match payload.get("type").and_then(Value::as_str)? {
        "viewer-milestones-update" => data.get("event_type").and_then(Value::as_str) == Some("achieved"),
        "watch-streak" | "streak-recovered" => true,
        _ => false,
    };
    if !reached {
        return None;
    }
    let channel = data.get("channel_id").and_then(Value::as_str).filter(|c| !c.is_empty())?.to_owned();
    let streak = data.get("watch_streak_value").and_then(Value::as_u64).map(|v| v as u32);
    Some((channel, streak))
}

const READ: &str = "query($login: String!) { user(login: $login) { channel { id self { \
    viewerMilestones { id category value state shareStatus } } } } }";

/// One read of a channel: its id (to check the answer is for that chat) and the account's
/// milestones there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Read {
    pub channel_id: Option<String>,
    pub milestones: Vec<Milestone>,
}

/// The account's milestones in a channel; empty signed out or when the channel has none.
pub async fn milestones(http: &Http, auth: Auth<'_>, login: &str) -> Result<Read> {
    let data: Value = query(http, auth, READ, json!({ "login": login })).await?;
    Ok(Read {
        channel_id: data["user"]["channel"]["id"].as_str().filter(|s| !s.is_empty()).map(str::to_owned),
        milestones: read(&data),
    })
}

/// The streak to offer from a read: a `WATCH_STREAK` that Twitch lets the account share, with
/// a count, not suspended, not `skip`ped (shared or refused) and not above `ceiling` (the
/// current count from the last event, when known). Several left: the smallest, since a
/// higher one may be the next threshold rather than one reached.
pub fn pick(milestones: &[Milestone], ceiling: Option<u32>, skip: impl Fn(&str) -> bool) -> Option<&Milestone> {
    milestones
        .iter()
        .filter(|m| m.is_streak() && m.can_share() && m.value > 0)
        .filter(|m| !matches!(m.state.as_str(), "NONE" | "SUSPENDED"))
        .filter(|m| ceiling.is_none_or(|c| m.value <= c))
        .filter(|m| !skip(&m.id))
        .min_by_key(|m| m.value)
}

fn read(data: &Value) -> Vec<Milestone> {
    data["user"]["channel"]["self"]["viewerMilestones"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            Some(Milestone {
                id: m["id"].as_str().filter(|s| !s.is_empty())?.to_owned(),
                category: m["category"].as_str().unwrap_or_default().to_owned(),
                value: m["value"].as_u64().unwrap_or(0) as u32,
                state: m["state"].as_str().unwrap_or_default().to_owned(),
                share_status: m["shareStatus"].as_str().unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

/// Posts the milestone to the channel's chat as Twitch's own notice. Only from the user's
/// click. [`Error::Refused`] carries Twitch's error code (see [`explain`]).
pub async fn share(http: &Http, auth: Auth<'_>, channel_id: &str, milestone_id: &str) -> Result<()> {
    const Q: &str = "mutation($input: ShareViewerMilestoneInput!) { \
        shareViewerMilestone(input: $input) { milestone { id } error { code } } }";
    let vars = json!({ "input": { "channelID": channel_id, "milestoneID": milestone_id } });
    let data: Value = query(http, auth, Q, vars).await?;
    shared(&data)
}

fn shared(data: &Value) -> Result<()> {
    let made = &data["shareViewerMilestone"];
    if let Some(code) = made["error"]["code"].as_str() {
        return Err(Error::Refused(code.to_owned()));
    }
    if made["milestone"]["id"].as_str().is_some() {
        Ok(())
    } else {
        Err(Error::Refused("UNKNOWN".into()))
    }
}

/// A short human line for Twitch's error code (codes unverified; unknown ones get a plain line).
pub fn explain(code: &str) -> &'static str {
    let c = code.to_ascii_uppercase();
    if c.contains("ALREADY") || c.contains("SHARED") {
        "Already shared."
    } else if c.contains("EXPIRED") || c.contains("NOT_FOUND") || c.contains("INVALID") {
        "That streak is gone."
    } else if c.contains("RATE") || c.contains("THROTTLE") {
        "Slow down a moment."
    } else if c.contains("BANNED") || c.contains("TIMEOUT") || c.contains("UNAUTHORIZED") || c.contains("FORBIDDEN") {
        "Twitch won’t let you post here right now."
    } else {
        "Twitch didn’t share it."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_milestones_and_picks_the_shareable_streak() {
        let d = json!({ "user": { "channel": { "self": { "viewerMilestones": [
            { "id": "m1", "category": "WATCH_STREAK", "value": 5, "state": "ACHIEVED", "shareStatus": "CAN_SHARE" },
            { "id": "m2", "category": "SUBSCRIBER_ANNIVERSARY", "value": 12, "state": "ACHIEVED", "shareStatus": "CAN_SHARE" },
            { "id": "m3", "category": "WATCH_STREAK", "value": 3, "state": "ACHIEVED", "shareStatus": "SHARED" },
            { "id": "", "category": "WATCH_STREAK" },
            { "id": "m4", "category": null, "value": null }
        ] } } } });
        let all = read(&d);
        assert_eq!(all.len(), 4);
        let pick = all.iter().find(|m| m.is_streak() && m.can_share()).unwrap();
        assert_eq!((pick.id.as_str(), pick.value), ("m1", 5));
        assert!(!all[2].can_share());
        assert_eq!(all[3].value, 0);
    }

    fn streak(id: &str, value: u32, state: &str, share: &str) -> Milestone {
        Milestone {
            id: id.into(),
            category: WATCH_STREAK.into(),
            value,
            state: state.into(),
            share_status: share.into(),
        }
    }

    #[test]
    fn pick_takes_the_smallest_shareable_streak_under_the_ceiling() {
        let all = [
            streak("s5", 5, "ACTIVE", "CAN_SHARE"),
            streak("s3", 3, "ACTIVE", "CAN_SHARE"),
            streak("old", 2, "ACTIVE", "SHARED"),
            streak("sus", 1, "SUSPENDED", "CAN_SHARE"),
            streak("zero", 0, "ACTIVE", "CAN_SHARE"),
        ];
        let none = |_: &str| false;
        assert_eq!(pick(&all, None, none).map(|m| m.value), Some(3));
        // 3 shared: 5 is left, but not above a known current count of 4.
        let shared = |id: &str| id == "s3";
        assert_eq!(pick(&all, Some(4), shared), None);
        assert_eq!(pick(&all, Some(5), shared).map(|m| m.value), Some(5));
        assert_eq!(pick(&all, Some(2), none), None);
    }

    #[test]
    fn signed_out_or_no_channel_reads_empty() {
        assert!(read(&json!({ "user": { "channel": { "self": null } } })).is_empty());
        assert!(read(&json!({ "user": null })).is_empty());
        assert!(read(&json!({})).is_empty());
    }

    #[test]
    fn share_answer_is_ok_or_twitchs_code() {
        assert!(shared(&json!({ "shareViewerMilestone": { "milestone": { "id": "m1" }, "error": null } })).is_ok());
        let bad = shared(&json!({ "shareViewerMilestone": { "milestone": null, "error": { "code": "ALREADY_SHARED" } } }));
        assert!(matches!(bad, Err(Error::Refused(c)) if c == "ALREADY_SHARED"));
        assert!(matches!(shared(&json!({ "shareViewerMilestone": null })), Err(Error::Refused(c)) if c == "UNKNOWN"));
        assert_eq!(explain("ALREADY_SHARED"), "Already shared.");
        assert_eq!(explain("whatever"), "Twitch didn’t share it.");
    }

    #[test]
    fn topics_and_updates() {
        assert_eq!(topic("7", None), "viewer-milestones.7");
        assert_eq!(topic("7", Some("9")), "viewer-milestones.7.9");
        let p = json!({ "type": "viewer-milestones-update",
            "data": { "event_type": "achieved", "channel_id": "9", "watch_streak_value": 6 } });
        assert_eq!(update_of(&p), Some(("9".into(), Some(6))));
        assert_eq!(update_of(&json!({ "data": {} })), None);
        // A share's own update (or anything not reached) offers nothing new.
        let shared = json!({ "type": "viewer-milestones-update",
            "data": { "event_type": "shared", "channel_id": "9", "watch_streak_value": 5 } });
        assert_eq!(update_of(&shared), None);
        let kept = json!({ "type": "watch-streak", "data": { "channel_id": "9" } });
        assert_eq!(update_of(&kept), Some(("9".into(), None)));
        assert_eq!(update_of(&json!({ "type": "other", "data": { "channel_id": "9" } })), None);
    }
}
