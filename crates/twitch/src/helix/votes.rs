//! 1.35 Polls and predictions the broadcaster runs. Creating and ending them is Helix
//! (`channel:manage:polls`, `channel:manage:predictions`); the live state and viewer votes
//! keep coming through the GQL + Hermes path (`chat::votes`). Returns the new id so the
//! caller can manage it.

use reqwest::Method;
use serde::Deserialize;
use serde_json::json;

use super::{Auth, Data, post, request};
use crate::http::{empty, json};
use crate::{Http, Result};

/// Twitch's limits.
pub const POLL_DURATION: (u32, u32) = (15, 1800);
pub const PREDICTION_WINDOW: (u32, u32) = (30, 1800);

#[derive(Deserialize)]
struct Created {
    id: String,
}

/// Starts a poll with 2-5 choices. `points_per_vote` lets viewers buy extra votes.
pub async fn create_poll(
    http: &Http,
    auth: Auth<'_>,
    broadcaster_id: &str,
    title: &str,
    choices: &[String],
    duration: u32,
    points_per_vote: Option<u32>,
) -> Result<String> {
    let choices: Vec<_> = choices
        .iter()
        .take(5)
        .map(|c| json!({ "title": clip(c, 25) }))
        .collect();
    let mut body = json!({
        "broadcaster_id": broadcaster_id,
        "title": clip(title, 60),
        "choices": choices,
        "duration": duration.clamp(POLL_DURATION.0, POLL_DURATION.1),
    });
    if let Some(cost) = points_per_vote {
        body["channel_points_voting_enabled"] = true.into();
        body["channel_points_per_vote"] = cost.into();
    }
    let d: Data<Created> = json(post(http, auth, "/polls").json(&body)).await?;
    Ok(d.data.into_iter().next().map(|c| c.id).unwrap_or_default())
}

/// Ends a poll now. `archive` hides it; otherwise it ends showing the result.
pub async fn end_poll(
    http: &Http,
    auth: Auth<'_>,
    broadcaster_id: &str,
    id: &str,
    archive: bool,
) -> Result<()> {
    let status = if archive { "ARCHIVED" } else { "TERMINATED" };
    let req = request(http, auth, Method::PATCH, "/polls")
        .json(&json!({ "broadcaster_id": broadcaster_id, "id": id, "status": status }));
    empty(req).await
}

/// Starts a prediction with 2-10 outcomes; `window` is how long betting stays open.
pub async fn create_prediction(
    http: &Http,
    auth: Auth<'_>,
    broadcaster_id: &str,
    title: &str,
    outcomes: &[String],
    window: u32,
) -> Result<String> {
    let outcomes: Vec<_> = outcomes
        .iter()
        .take(10)
        .map(|o| json!({ "title": clip(o, 25) }))
        .collect();
    let body = json!({
        "broadcaster_id": broadcaster_id,
        "title": clip(title, 45),
        "outcomes": outcomes,
        "prediction_window": window.clamp(PREDICTION_WINDOW.0, PREDICTION_WINDOW.1),
    });
    let d: Data<Created> = json(post(http, auth, "/predictions").json(&body)).await?;
    Ok(d.data.into_iter().next().map(|c| c.id).unwrap_or_default())
}

/// Locks, resolves (to `winner`) or cancels a prediction. Status is `LOCKED`, `RESOLVED`
/// or `CANCELED`; resolving needs the winning outcome id.
pub async fn end_prediction(
    http: &Http,
    auth: Auth<'_>,
    broadcaster_id: &str,
    id: &str,
    status: &str,
    winner: Option<&str>,
) -> Result<()> {
    let mut body = json!({ "broadcaster_id": broadcaster_id, "id": id, "status": status });
    if let Some(w) = winner {
        body["winning_outcome_id"] = w.into();
    }
    let req = request(http, auth, Method::PATCH, "/predictions").json(&body);
    empty(req).await
}

/// Twitch rejects over-long titles; keep to the character cap.
fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_id_parses() {
        let d: Data<Created> =
            serde_json::from_str(r#"{"data":[{"id":"poll-1","title":"x","status":"ACTIVE"}]}"#).unwrap();
        assert_eq!(d.data[0].id, "poll-1");
    }

    #[test]
    fn long_titles_clip() {
        assert_eq!(clip(&"a".repeat(80), 60).chars().count(), 60);
        assert_eq!(clip("short", 60), "short");
    }
}
