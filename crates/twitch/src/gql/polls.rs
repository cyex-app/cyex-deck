//! C4 Polls as a viewer: the running poll, your votes, voting. Live changes come over
//! Hermes (`polls.<channel id>`) in snake_case; `Poll::from_hermes` reads those.
//! Research: `docs/research/twitch-api.md` (phase 8).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, query};
use crate::random::hex;
use crate::{Http, Result};

const READ: &str = "query($login: String!) { user(login: $login) { id viewablePoll { \
    id title status durationSeconds remainingDurationMilliseconds totalVoters \
    settings { multichoice { isEnabled } communityPointsVotes { isEnabled cost } } \
    votes { total } \
    choices { id title totalVoters votes { total } } \
    self { voter { choices { pollChoice { id } } } } } } }";

const VOTE: &str = "mutation($input: VoteInPollInput!) { voteInPoll(input: $input) { error { code } } }";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Poll {
    pub id: String,
    pub title: String,
    /// ACTIVE, COMPLETED, TERMINATED, ARCHIVED, MODERATED.
    pub status: String,
    pub duration_secs: u32,
    /// Left when this was read, in ms.
    #[ts(type = "number")]
    pub remaining_ms: i64,
    pub multi: bool,
    /// Points per extra vote, when extra votes are on.
    pub points_cost: Option<u32>,
    #[ts(type = "number")]
    pub total_votes: u64,
    #[ts(type = "number")]
    pub total_voters: u64,
    pub choices: Vec<PollChoice>,
    /// Choices you voted for. Hermes doesn't say; kept from the last read.
    pub mine: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PollChoice {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub votes: u64,
    #[ts(type = "number")]
    pub voters: u64,
}

impl Poll {
    /// A Hermes `polls` message, keeping `mine` from what we knew.
    pub fn from_hermes(payload: &Value, mine: Vec<String>) -> Option<Self> {
        let p = payload.pointer("/data/poll")?;
        let num = |v: &Value| v.as_u64().unwrap_or(0);
        let points = &p["settings"]["channel_points_votes"];
        Some(Self {
            id: p["poll_id"].as_str()?.to_owned(),
            title: p["title"].as_str().unwrap_or_default().to_owned(),
            status: p["status"].as_str().unwrap_or("ACTIVE").to_owned(),
            duration_secs: num(&p["duration_seconds"]) as u32,
            remaining_ms: p["remaining_duration_milliseconds"].as_i64().unwrap_or(0),
            multi: p["settings"]["multi_choice"]["is_enabled"]
                .as_bool()
                .unwrap_or(false),
            points_cost: points["is_enabled"]
                .as_bool()
                .unwrap_or(false)
                .then(|| num(&points["cost"]) as u32),
            total_votes: num(&p["votes"]["total"]),
            total_voters: num(&p["total_voters"]),
            choices: p["choices"]
                .as_array()
                .map(|cs| {
                    cs.iter()
                        .map(|c| PollChoice {
                            id: c["choice_id"].as_str().unwrap_or_default().to_owned(),
                            title: c["title"].as_str().unwrap_or_default().to_owned(),
                            votes: num(&c["votes"]["total"]),
                            voters: num(&c["total_voters"]),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            mine,
        })
    }
}

/// The channel's id and its poll, if one is up. `auth` with a token also reads your
/// votes. `None` if there's no such channel.
pub async fn poll(http: &Http, auth: Auth<'_>, login: &str) -> Result<Option<(String, Option<Poll>)>> {
    let data: RawData = query(http, auth, READ, json!({ "login": login })).await?;
    Ok(data.user.map(|u| (u.id, u.viewable_poll.map(Poll::from))))
}

/// One free vote, or with `points` an extra vote bought with channel points.
pub async fn vote(
    http: &Http,
    auth: Auth<'_>,
    poll_id: &str,
    choice_id: &str,
    user_id: &str,
    points: Option<u32>,
) -> Result<Option<String>> {
    let mut input = json!({ "pollID": poll_id, "choiceID": choice_id, "userID": user_id, "voteID": hex(16) });
    if let Some(p) = points {
        input["tokens"] = json!({ "channelPoints": p });
    }
    let v: Value = query(http, auth, VOTE, json!({ "input": input })).await?;
    Ok(v.pointer("/voteInPoll/error/code")
        .and_then(Value::as_str)
        .map(str::to_owned))
}

#[derive(Deserialize)]
struct RawData {
    user: Option<RawUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawUser {
    id: String,
    viewable_poll: Option<RawPoll>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPoll {
    id: String,
    title: String,
    status: String,
    duration_seconds: u32,
    remaining_duration_milliseconds: i64,
    #[serde(default)]
    total_voters: u64,
    settings: RawSettings,
    votes: RawVotes,
    choices: Vec<RawChoice>,
    #[serde(rename = "self")]
    me: Option<RawSelf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSettings {
    multichoice: Option<RawEnabled>,
    community_points_votes: Option<RawEnabled>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEnabled {
    is_enabled: bool,
    #[serde(default)]
    cost: u32,
}

#[derive(Deserialize)]
struct RawVotes {
    total: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChoice {
    id: String,
    title: String,
    #[serde(default)]
    total_voters: u64,
    votes: RawVotes,
}

#[derive(Deserialize)]
struct RawSelf {
    voter: Option<RawVoter>,
}

#[derive(Deserialize)]
struct RawVoter {
    #[serde(default)]
    choices: Vec<RawVoterChoice>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVoterChoice {
    poll_choice: RawId,
}

#[derive(Deserialize)]
struct RawId {
    id: String,
}

impl From<RawPoll> for Poll {
    fn from(p: RawPoll) -> Self {
        Self {
            id: p.id,
            title: p.title,
            status: p.status,
            duration_secs: p.duration_seconds,
            remaining_ms: p.remaining_duration_milliseconds,
            multi: p.settings.multichoice.is_some_and(|m| m.is_enabled),
            points_cost: p
                .settings
                .community_points_votes
                .filter(|v| v.is_enabled)
                .map(|v| v.cost),
            total_votes: p.votes.total,
            total_voters: p.total_voters,
            choices: p
                .choices
                .into_iter()
                .map(|c| PollChoice {
                    id: c.id,
                    title: c.title,
                    votes: c.votes.total,
                    voters: c.total_voters,
                })
                .collect(),
            mine: p
                .me
                .and_then(|s| s.voter)
                .map(|v| v.choices.into_iter().map(|c| c.poll_choice.id).collect())
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_hermes() {
        let d: RawData = serde_json::from_str(
            r#"{"user":{"id":"1","viewablePoll":{"id":"p","title":"Who?","status":"ACTIVE","durationSeconds":300,
            "remainingDurationMilliseconds":69224,"totalVoters":26,"settings":{"multichoice":{"isEnabled":true},
            "communityPointsVotes":{"isEnabled":true,"cost":1}},"votes":{"total":186},
            "choices":[{"id":"a","title":"A","totalVoters":19,"votes":{"total":121}}],
            "self":{"voter":{"choices":[{"pollChoice":{"id":"a"}}]}}}}}"#,
        )
        .unwrap();
        let p = Poll::from(d.user.unwrap().viewable_poll.unwrap());
        assert_eq!(
            (p.points_cost, p.multi, p.mine.as_slice()),
            (Some(1), true, &["a".to_owned()][..])
        );
        let h: Value = serde_json::from_str(
            r#"{"type":"POLL_UPDATE","data":{"poll":{"poll_id":"p","title":"Who?","duration_seconds":300,
            "settings":{"multi_choice":{"is_enabled":true},"channel_points_votes":{"is_enabled":true,"cost":1}},
            "status":"ACTIVE","choices":[{"choice_id":"a","title":"A","votes":{"total":130},"total_voters":20}],
            "votes":{"total":190},"total_voters":27,"remaining_duration_milliseconds":5000}}}"#,
        )
        .unwrap();
        let p2 = Poll::from_hermes(&h, p.mine.clone()).unwrap();
        assert_eq!(
            (p2.choices[0].votes, p2.total_voters, p2.remaining_ms),
            (130, 27, 5000)
        );
        assert_eq!(p2.mine, ["a"]);
    }
}
