//! V108-N04 A channel's hype train for viewers: a GQL read for the state when a chat opens
//! and PubSub `hype-train-events-v2.<channel id>` for the updates. Read shape checked
//! 2026/10/29 with an anonymous query (empty login, validation only); the PubSub payload is
//! read from twitch.tv's bundle (handler of `hype-train-progression`), not seen live:
//! `{ "type": "hype-train-progression", "data": { sequence_id, expires_at, progress:
//! { level: { value, goal }, goal, total, value, all_time_high_state } } }`. Other types:
//! `hype-train-approaching`, `hype-train-level-up`, `hype-train-end`,
//! `hype-train-conductor-update` (`data.source` `BITS`/`SUBS`), `hype-train-rewards`.
//! Every field is optional here, so a changed message degrades instead of failing.
//! The subs and bits come from `execution.participations { source action quantity }`
//! (validated anonymously 2026/11/01; values unseen live): the GQL read fills them, the
//! PubSub messages don't carry them.

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

/// A person at the top of the train.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "HypeConductor")]
pub struct Conductor {
    /// `BITS` or `SUBS`.
    pub source: String,
    pub login: String,
    pub name: String,
    /// What they gave (bits, or sub points).
    pub total: u32,
}

/// A running hype train.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "HypeTrain")]
pub struct Train {
    pub id: String,
    pub level: u32,
    /// Points inside this level.
    pub progress: u32,
    /// Points this level needs.
    pub goal: u32,
    /// Points in all.
    pub total: u32,
    /// RFC 3339.
    pub started_at: Option<String>,
    /// RFC 3339; the train ends then unless it fills up.
    pub expires_at: Option<String>,
    /// RFC 3339; set once it has ended.
    pub ended_at: Option<String>,
    pub golden_kappa: bool,
    pub treasure: bool,
    pub conductors: Vec<Conductor>,
    /// Subs given in the train (own and gifted, any tier); 0 when unknown.
    pub subs: u32,
    /// Bits given in the train (cheers, polls, extensions); 0 when unknown.
    pub bits: u32,
}

impl Train {
    /// Still running at `now` (RFC 3339, UTC: compared as text).
    pub fn running(&self, now: &str) -> bool {
        self.ended_at.is_none() && self.expires_at.as_deref().is_none_or(|e| e > now)
    }
}

/// What the topic can tell, per message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A progress or level-up message; merge into the train.
    Progress { sequence: u64, train: Train },
    /// Twitch says one is about to start (`goal` events needed).
    Approaching { goal: u32 },
    /// A conductor changed.
    Conductor(Conductor),
    /// The train ended (`ended_at` may be empty).
    End,
}

/// The topic for a channel id.
pub fn topic(channel_id: &str) -> String {
    format!("hype-train-events-v2.{channel_id}")
}

const READ: &str = "query($login: String!) { user(login: $login) { id channel { id hypeTrain { execution { \
    id startedAt expiresAt endedAt \
    progress { level { value goal } goal progression total } \
    conductors { source participation { quantity } user { login displayName } } \
    participations { source action quantity } \
    isGoldenKappaTrain isTreasureTrain } } } } }";

/// The channel's train now; `None` when none is running.
pub async fn current(http: &Http, auth: Auth<'_>, login: &str, now: &str) -> Result<Option<Train>> {
    Ok(latest(http, auth, login).await?.filter(|t| t.running(now)))
}

/// The channel's last train, ended or not: the closing stats (V108-N04).
pub async fn latest(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<Train>> {
    let data: Value = query(http, auth, READ, json!({ "login": login })).await?;
    Ok(read(&data))
}

fn read(data: &Value) -> Option<Train> {
    let e = data["user"]["channel"]["hypeTrain"]["execution"].as_object()?;
    let e = Value::Object(e.clone());
    let mut t = train_of(&e);
    t.conductors = e["conductors"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some(Conductor {
                source: c["source"].as_str()?.to_owned(),
                login: c["user"]["login"].as_str().unwrap_or_default().to_owned(),
                name: c["user"]["displayName"].as_str().or_else(|| c["user"]["login"].as_str()).unwrap_or_default().to_owned(),
                total: c["participation"]["quantity"].as_u64().unwrap_or(0) as u32,
            })
        })
        .collect();
    (t.subs, t.bits) = given(&e["participations"]);
    (!t.id.is_empty()).then_some(t)
}

/// Subs and bits summed over the train's participations, by `source`.
fn given(list: &Value) -> (u32, u32) {
    let (mut subs, mut bits) = (0u32, 0u32);
    for p in list.as_array().into_iter().flatten() {
        let q = num(&p["quantity"]);
        match p["source"].as_str() {
            Some("SUBS") => subs = subs.saturating_add(q),
            Some("BITS") => bits = bits.saturating_add(q),
            _ => {}
        }
    }
    (subs, bits)
}

fn num(v: &Value) -> u32 {
    v.as_u64().unwrap_or(0).min(u32::MAX as u64) as u32
}

/// The train fields both shapes share. GQL calls the in-level points `progression`, the
/// PubSub message `value`.
fn train_of(e: &Value) -> Train {
    let p = &e["progress"];
    let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    Train {
        id: e["id"].as_str().unwrap_or_default().to_owned(),
        level: num(&p["level"]["value"]),
        progress: num(if p["progression"].is_null() { &p["value"] } else { &p["progression"] }),
        goal: num(if p["level"]["goal"].is_null() { &p["goal"] } else { &p["level"]["goal"] }),
        total: num(&p["total"]),
        started_at: text(&e["started_at"]).or_else(|| text(&e["startedAt"])),
        expires_at: text(&e["expires_at"]).or_else(|| text(&e["expiresAt"])),
        ended_at: text(&e["ended_at"]).or_else(|| text(&e["endedAt"])),
        golden_kappa: e["is_golden_kappa_train"].as_bool().or_else(|| e["isGoldenKappaTrain"].as_bool()).unwrap_or(false),
        treasure: e["is_treasure_train"].as_bool().or_else(|| e["isTreasureTrain"].as_bool()).unwrap_or(false),
        conductors: Vec::new(),
        subs: 0,
        bits: 0,
    }
}

/// One PubSub message of the topic. `None` for types this doesn't use.
pub fn event_of(payload: &Value) -> Option<Event> {
    let data = &payload["data"];
    match payload["type"].as_str()? {
        "hype-train-progression" | "hype-train-level-up" => Some(Event::Progress {
            sequence: data["sequence_id"].as_u64().unwrap_or(0),
            train: train_of(data),
        }),
        "hype-train-approaching" => Some(Event::Approaching { goal: num(&data["goal"]) }),
        "hype-train-conductor-update" => Some(Event::Conductor(Conductor {
            source: data["source"].as_str()?.to_owned(),
            login: data["user"]["login"].as_str().or_else(|| data["user_login"].as_str()).unwrap_or_default().to_owned(),
            name: data["user"]["display_name"]
                .as_str()
                .or_else(|| data["user"]["displayName"].as_str())
                .or_else(|| data["user"]["login"].as_str())
                .unwrap_or_default()
                .to_owned(),
            total: num(&data["participations"]["total"]).max(num(&data["participation"]["quantity"])),
        })),
        "hype-train-end" => Some(Event::End),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_graphql_train() {
        let d = json!({ "user": { "id": "1", "channel": { "id": "1", "hypeTrain": { "execution": {
            "id": "t1", "startedAt": "2026-10-29T18:00:00Z", "expiresAt": "2026-10-29T18:05:00Z", "endedAt": null,
            "progress": { "level": { "value": 3, "goal": 1800 }, "goal": 1800, "progression": 700, "total": 2900 },
            "conductors": [
              { "source": "BITS", "participation": { "quantity": 500 }, "user": { "login": "a", "displayName": "A" } },
              { "source": "SUBS", "participation": null, "user": null }],
            "isGoldenKappaTrain": false, "isTreasureTrain": true } } } } });
        let t = read(&d).unwrap();
        assert_eq!((t.level, t.progress, t.goal, t.total), (3, 700, 1800, 2900));
        assert!(t.treasure && !t.golden_kappa);
        assert_eq!(t.conductors.len(), 2);
        assert_eq!(t.conductors[0].name, "A");
        assert!(t.running("2026-10-29T18:01:00Z"));
        assert!(!t.running("2026-10-29T18:06:00Z"));
    }

    #[test]
    fn sums_subs_and_bits_by_source() {
        let d = json!({ "user": { "channel": { "hypeTrain": { "execution": {
            "id": "t1", "endedAt": "2026-10-29T18:09:00Z",
            "progress": { "level": { "value": 4, "goal": 2000 }, "progression": 2100, "total": 9000 },
            "participations": [
              { "source": "SUBS", "action": "TIER_1_SUB", "quantity": 3 },
              { "source": "SUBS", "action": "TIER_1_GIFTED_SUBSCRIPTION", "quantity": 10 },
              { "source": "BITS", "action": "CHEER", "quantity": 1500 },
              { "source": "BITS", "action": "BITS_ON_POLL", "quantity": 200 },
              { "source": "OTHER", "quantity": 7 },
              { "source": "BITS" }] } } } } });
        let t = read(&d).unwrap();
        assert_eq!((t.subs, t.bits), (13, 1700));
        assert!(!t.running("2026-10-29T18:01:00Z"));
        assert_eq!(given(&Value::Null), (0, 0));
    }

    #[test]
    fn no_train_reads_none() {
        assert!(read(&json!({ "user": { "channel": { "hypeTrain": { "execution": null } } } })).is_none());
        assert!(read(&json!({ "user": null })).is_none());
    }

    #[test]
    fn pubsub_progress_uses_value_for_in_level_points() {
        let p = json!({ "type": "hype-train-progression", "data": {
            "sequence_id": 12, "expires_at": "2026-10-29T18:05:00Z",
            "progress": { "level": { "value": 2, "goal": 1200 }, "goal": 1200, "total": 1500, "value": 300 } } });
        match event_of(&p).unwrap() {
            Event::Progress { sequence, train } => {
                assert_eq!(sequence, 12);
                assert_eq!((train.level, train.progress, train.goal, train.total), (2, 300, 1200, 1500));
                assert_eq!(train.expires_at.as_deref(), Some("2026-10-29T18:05:00Z"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pubsub_other_types_and_junk() {
        assert_eq!(event_of(&json!({ "type": "hype-train-end", "data": {} })), Some(Event::End));
        assert_eq!(
            event_of(&json!({ "type": "hype-train-approaching", "data": { "goal": 4 } })),
            Some(Event::Approaching { goal: 4 })
        );
        assert!(matches!(
            event_of(&json!({ "type": "hype-train-conductor-update", "data": { "source": "BITS", "user": { "login": "x" } } })),
            Some(Event::Conductor(c)) if c.login == "x" && c.name == "x"
        ));
        assert_eq!(event_of(&json!({ "type": "hype-train-rewards", "data": {} })), None);
        assert_eq!(event_of(&json!({})), None);
        assert_eq!(topic("7"), "hype-train-events-v2.7");
    }
}
