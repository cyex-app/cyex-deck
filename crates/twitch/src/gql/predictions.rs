//! C4 Predictions as a viewer: the open or locked one, your bet, betting. Live changes
//! come over Hermes (`predictions-channel-v1.<channel id>`), out of order, so updates
//! older than what we have are dropped by the caller.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, persisted, query};
use crate::random::hex;
use crate::{Http, Result};

const READ: &str = "query($login: String!) { channel(name: $login) { id \
    self { communityPoints { balance } } \
    activePredictionEvents { ...ev } lockedPredictionEvents { ...ev } } \
    currentUser { predictionsSettings { hasAcceptedTOS } } } \
    fragment ev on PredictionEvent { id status title createdAt lockedAt predictionWindowSeconds \
    outcomes { id title color totalPoints totalUsers topPredictors { points user { login displayName } } } \
    winningOutcome { id } self { prediction { points outcome { id } result pointsWon } } }";

/// The miner's persisted `MakePrediction` (still served 2026/10/10).
const MAKE: (&str, &str) = (
    "MakePrediction",
    "b44682ecc88358817009f20e69d75081b1e58825bb40aa53d5dbadcc17c881d8",
);
const ACCEPT: &str =
    "mutation { updateUserPredictionSettings(input: {hasAcceptedTOS: true}) { error { code } } }";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Prediction {
    pub id: String,
    pub title: String,
    /// ACTIVE, LOCKED, RESOLVE_PENDING, RESOLVED, CANCEL_PENDING, CANCELED.
    pub status: String,
    /// RFC 3339.
    pub created_at: String,
    pub window_secs: u32,
    pub outcomes: Vec<Outcome>,
    pub winner: Option<String>,
    pub mine: Option<Bet>,
    /// Hermes time of the data, to drop late updates.
    pub updated: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Outcome {
    pub id: String,
    pub title: String,
    /// BLUE, PINK, …
    pub color: String,
    #[ts(type = "number")]
    pub points: u64,
    #[ts(type = "number")]
    pub users: u64,
    /// Biggest bet: name and points.
    #[ts(type = "[string, number] | null")]
    pub top: Option<(String, u64)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Bet {
    pub outcome: String,
    #[ts(type = "number")]
    pub points: u64,
    /// WIN, LOSE, REFUND once settled.
    pub result: Option<String>,
    #[ts(type = "number | null")]
    pub won: Option<u64>,
}

/// What the viewer needs to bet.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PredictionState {
    pub prediction: Option<Prediction>,
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
    /// Twitch's prediction terms are accepted (asked before the first bet).
    pub accepted: bool,
}

impl Prediction {
    /// A Hermes `event-created` or `event-updated`, keeping `mine` from what we knew.
    pub fn from_hermes(payload: &Value, mine: Option<Bet>) -> Option<Self> {
        let d = payload.get("data")?;
        let e = d.get("event")?;
        let s = |v: &Value| v.as_str().unwrap_or_default().to_owned();
        Some(Self {
            id: s(&e["id"]),
            title: s(&e["title"]),
            status: s(&e["status"]),
            created_at: s(&e["created_at"]),
            window_secs: e["prediction_window_seconds"].as_u64().unwrap_or(0) as u32,
            outcomes: e["outcomes"]
                .as_array()?
                .iter()
                .map(|o| Outcome {
                    id: s(&o["id"]),
                    title: s(&o["title"]),
                    color: s(&o["color"]),
                    points: o["total_points"].as_u64().unwrap_or(0),
                    users: o["total_users"].as_u64().unwrap_or(0),
                    top: o["top_predictors"]
                        .as_array()
                        .and_then(|t| t.first())
                        .map(|t| (s(&t["user_display_name"]), t["points"].as_u64().unwrap_or(0))),
                })
                .collect(),
            winner: e["winning_outcome_id"].as_str().map(str::to_owned),
            mine,
            updated: s(&d["timestamp"]),
        })
    }
}

pub async fn prediction(http: &Http, auth: Auth<'_>, login: &str) -> Result<PredictionState> {
    let d: RawData = query(http, auth, READ, json!({ "login": login })).await?;
    let accepted = d
        .current_user
        .and_then(|u| u.predictions_settings)
        .is_some_and(|s| s.has_accepted_tos);
    let Some(c) = d.channel else {
        return Ok(PredictionState::default());
    };
    let ev = c
        .active_prediction_events
        .into_iter()
        .chain(c.locked_prediction_events)
        .next();
    Ok(PredictionState {
        prediction: ev.map(Prediction::from),
        balance: c.me.and_then(|s| s.community_points).map(|p| p.balance),
        accepted,
    })
}

/// Bets `points` on an outcome. `Some(code)` when Twitch said no, e.g. `NOT_ENOUGH_POINTS`.
pub async fn predict(
    http: &Http,
    auth: Auth<'_>,
    event_id: &str,
    outcome_id: &str,
    points: u64,
) -> Result<Option<String>> {
    let input =
        json!({ "eventID": event_id, "outcomeID": outcome_id, "points": points, "transactionID": hex(16) });
    let v: Value = persisted(http, auth, MAKE.0, MAKE.1, json!({ "input": input })).await?;
    Ok(v.pointer("/makePrediction/error/code")
        .and_then(Value::as_str)
        .map(str::to_owned))
}

/// Twitch's prediction terms, asked once per account.
pub async fn accept_terms(http: &Http, auth: Auth<'_>) -> Result<Option<String>> {
    let v: Value = query(http, auth, ACCEPT, json!({})).await?;
    Ok(v.pointer("/updateUserPredictionSettings/error/code")
        .and_then(Value::as_str)
        .map(str::to_owned))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawData {
    channel: Option<RawChannel>,
    current_user: Option<RawMe>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMe {
    predictions_settings: Option<RawTerms>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTerms {
    #[serde(rename = "hasAcceptedTOS")]
    has_accepted_tos: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannel {
    #[serde(rename = "self")]
    me: Option<RawChannelSelf>,
    #[serde(default)]
    active_prediction_events: Vec<RawEvent>,
    #[serde(default)]
    locked_prediction_events: Vec<RawEvent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChannelSelf {
    community_points: Option<RawPoints>,
}

#[derive(Deserialize)]
struct RawPoints {
    balance: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEvent {
    id: String,
    status: String,
    title: String,
    created_at: String,
    #[serde(default)]
    prediction_window_seconds: u32,
    outcomes: Vec<RawOutcome>,
    winning_outcome: Option<RawId>,
    #[serde(rename = "self")]
    me: Option<RawEventSelf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOutcome {
    id: String,
    title: String,
    color: String,
    total_points: u64,
    total_users: u64,
    #[serde(default)]
    top_predictors: Vec<RawTop>,
}

#[derive(Deserialize)]
struct RawTop {
    points: u64,
    user: Option<RawUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    display_name: String,
}

#[derive(Deserialize)]
struct RawId {
    id: String,
}

#[derive(Deserialize)]
struct RawEventSelf {
    prediction: Option<RawBet>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBet {
    points: u64,
    outcome: RawId,
    result: Option<Value>,
    points_won: Option<u64>,
}

impl From<RawEvent> for Prediction {
    fn from(e: RawEvent) -> Self {
        Self {
            id: e.id,
            title: e.title,
            status: e.status,
            created_at: e.created_at,
            window_secs: e.prediction_window_seconds,
            outcomes: e
                .outcomes
                .into_iter()
                .map(|o| Outcome {
                    id: o.id,
                    title: o.title,
                    color: o.color,
                    points: o.total_points,
                    users: o.total_users,
                    top: o
                        .top_predictors
                        .into_iter()
                        .next()
                        .and_then(|t| Some((t.user?.display_name, t.points))),
                })
                .collect(),
            winner: e.winning_outcome.map(|w| w.id),
            mine: e.me.and_then(|s| s.prediction).map(|b| Bet {
                outcome: b.outcome.id,
                points: b.points,
                result: b.result.and_then(|r| {
                    r.get("type")
                        .or(Some(&r))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
                won: b.points_won,
            }),
            updated: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermes_event() {
        let v: Value = serde_json::from_str(
            r#"{"type":"event-updated","data":{"timestamp":"2026-10-10T08:59:33Z","event":{"id":"e","created_at":"x",
            "outcomes":[{"id":"a","color":"BLUE","title":"NRG","total_points":1904030,"total_users":440,
            "top_predictors":[{"points":250000,"user_display_name":"arnoldbvb"}]},
            {"id":"b","color":"PINK","title":"LOUD","total_points":5,"total_users":1,"top_predictors":[]}],
            "prediction_window_seconds":1800,"status":"ACTIVE","title":"Who wins?","winning_outcome_id":null}}}"#,
        )
        .unwrap();
        let p = Prediction::from_hermes(&v, None).unwrap();
        assert_eq!(p.outcomes[0].top, Some(("arnoldbvb".into(), 250_000)));
        assert_eq!(
            (p.window_secs, p.updated.as_str(), p.winner),
            (1800, "2026-10-10T08:59:33Z", None)
        );
    }
}
