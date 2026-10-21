//! 1.35 Polls and predictions run from your own channel. Their live state reaches the app
//! through the chat's vote watcher (Hermes), as for viewers.

use serde_json::{Value, json};

use super::channel::write;
use crate::gql::{Auth, query};
use crate::{Http, Result};

const CREATE_POLL: &str =
    "mutation($input: CreatePollInput!) { createPoll(input: $input) { error { code } } }";
const END_POLL: &str =
    "mutation($input: TerminatePollInput!) { terminatePoll(input: $input) { poll { id } } }";
const ARCHIVE_POLL: &str =
    "mutation($input: ArchivePollInput!) { archivePoll(input: $input) { poll { id } } }";
const CREATE_PREDICTION: &str = "mutation($input: CreatePredictionEventInput!) { \
    createPredictionEvent(input: $input) { error { code } } }";
const LOCK: &str =
    "mutation($input: LockPredictionEventInput!) { lockPredictionEvent(input: $input) { error { code } } }";
const RESOLVE: &str = "mutation($input: ResolvePredictionEventInput!) { \
    resolvePredictionEvent(input: $input) { error { code } } }";
const CANCEL: &str = "mutation($input: CancelPredictionEventInput!) { \
    cancelPredictionEvent(input: $input) { error { code } } }";

/// The `CreatePollInput`. Extra votes cost channel points when `points_per_vote` is set.
fn poll_input(
    channel_id: &str,
    title: &str,
    choices: &[String],
    secs: u32,
    points_per_vote: Option<u32>,
) -> Value {
    let choices: Vec<Value> = choices.iter().map(|c| json!({ "title": c })).collect();
    let mut input =
        json!({ "ownedBy": channel_id, "title": title, "durationSeconds": secs, "choices": choices });
    if let Some(cost) = points_per_vote {
        input["isCommunityPointsVotingEnabled"] = true.into();
        input["communityPointsCost"] = cost.into();
    }
    input
}

pub async fn create_poll(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    title: &str,
    choices: &[String],
    secs: u32,
    points_per_vote: Option<u32>,
) -> Result<()> {
    let input = poll_input(channel_id, title, choices, secs, points_per_vote);
    write(http, auth, None, CREATE_POLL, "createPoll", input).await.map(drop)
}

/// Ends the poll; `archive` also takes it off the channel. Neither payload has an error.
pub async fn end_poll(http: &Http, auth: Auth<'_>, poll_id: &str, archive: bool) -> Result<()> {
    let text = if archive { ARCHIVE_POLL } else { END_POLL };
    let _: Value = query(http, auth, text, json!({ "input": { "pollID": poll_id } })).await?;
    Ok(())
}

/// The outcome colors Twitch's dashboard uses: blue against pink for two, all blue above.
fn colors(n: usize) -> impl Iterator<Item = &'static str> {
    (0..n).map(move |i| if n == 2 && i == 1 { "PINK" } else { "BLUE" })
}

fn prediction_input(channel_id: &str, title: &str, outcomes: &[String], window: u32) -> Value {
    let outcomes: Vec<Value> = outcomes
        .iter()
        .zip(colors(outcomes.len()))
        .map(|(t, c)| json!({ "title": t, "color": c }))
        .collect();
    json!({ "channelID": channel_id, "title": title, "predictionWindowSeconds": window, "outcomes": outcomes })
}

pub async fn create_prediction(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    title: &str,
    outcomes: &[String],
    window: u32,
) -> Result<()> {
    let input = prediction_input(channel_id, title, outcomes, window);
    write(http, auth, None, CREATE_PREDICTION, "createPredictionEvent", input).await.map(drop)
}

pub async fn lock_prediction(http: &Http, auth: Auth<'_>, id: &str) -> Result<()> {
    write(http, auth, None, LOCK, "lockPredictionEvent", json!({ "id": id })).await.map(drop)
}

pub async fn resolve_prediction(http: &Http, auth: Auth<'_>, id: &str, winner: &str) -> Result<()> {
    let input = json!({ "eventID": id, "outcomeID": winner });
    write(http, auth, None, RESOLVE, "resolvePredictionEvent", input).await.map(drop)
}

/// Refunds everyone.
pub async fn cancel_prediction(http: &Http, auth: Auth<'_>, id: &str) -> Result<()> {
    write(http, auth, None, CANCEL, "cancelPredictionEvent", json!({ "id": id })).await.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_input_adds_points_only_when_set() {
        let choices = ["a".to_owned(), "b".to_owned()];
        let plain = poll_input("7", "q", &choices, 60, None);
        assert_eq!(
            plain,
            json!({ "ownedBy": "7", "title": "q", "durationSeconds": 60, "choices": [{ "title": "a" }, { "title": "b" }] })
        );
        let paid = poll_input("7", "q", &choices, 60, Some(100));
        assert_eq!(paid["isCommunityPointsVotingEnabled"], true);
        assert_eq!(paid["communityPointsCost"], 100);
    }

    #[test]
    fn outcome_colors_match_twitch() {
        assert_eq!(colors(2).collect::<Vec<_>>(), ["BLUE", "PINK"]);
        assert_eq!(colors(3).collect::<Vec<_>>(), ["BLUE", "BLUE", "BLUE"]);
        let p = prediction_input("7", "q", &["y".into(), "n".into()], 120);
        assert_eq!(p["outcomes"][1], json!({ "title": "n", "color": "PINK" }));
    }
}
