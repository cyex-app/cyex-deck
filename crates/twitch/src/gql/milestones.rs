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
//!   `watch-streak`, `streak-recovered`. See [`topic`].

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

/// What a `viewer-milestones-update` message says changed: the channel and, for a streak,
/// the new count. `None` for other events.
pub fn update_of(payload: &Value) -> Option<(String, Option<u32>)> {
    let data = payload.get("data")?;
    let channel = data.get("channel_id").and_then(Value::as_str)?.to_owned();
    let streak = data.get("watch_streak_value").and_then(Value::as_u64).map(|v| v as u32);
    Some((channel, streak))
}

const READ: &str = "query($login: String!) { user(login: $login) { channel { self { \
    viewerMilestones { id category value state shareStatus } } } } }";

/// The account's milestones in a channel; empty signed out or when the channel has none.
pub async fn milestones(http: &Http, auth: Auth<'_>, login: &str) -> Result<Vec<Milestone>> {
    let data: Value = query(http, auth, READ, json!({ "login": login })).await?;
    Ok(read(&data))
}

/// The streak the account can share now, if any.
pub async fn shareable_streak(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<Milestone>> {
    Ok(milestones(http, auth, login).await?.into_iter().find(|m| m.is_streak() && m.can_share()))
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
    }
}
