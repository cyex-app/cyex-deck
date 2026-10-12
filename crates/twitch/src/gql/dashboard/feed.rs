//! D1, D4 The dashboard's activity feed: the last 100 events when the stream manager opens
//! (`ActivityListContextV2Query` cut to what the app shows; the broadcaster's own token), and
//! replaying or skipping an alert in Twitch's alert box. Events stay GQL JSON for
//! `cyex_eventsub::alerts`, which reads the live alerts in the same shape. Checked against
//! the schema 2026/10/11; no write was run.

use serde_json::{Value, json};

use crate::gql::{Auth, query};
use crate::{Error, Http, Result};

/// Twitch's own dashboard asks for this many, the most it gives.
pub const HISTORY_LIMIT: u32 = 100;

const HISTORY: &str = "query Feed($channelID: ID!, $limit: Int!) {
  channel(id: $channelID) { activityFeed { events(first: $limit) { edges { node {
    id type timestamp title { ...T } content { ...T } userContent { ...T } alert { ...A } } } } } } }
fragment U on User { userID: id displayName login }
fragment M on ActivityFeedAlertMessageContent { fragments { __typename
  ... on ActivityFeedAlertMessageTextFragment { text }
  ... on ActivityFeedAlertMessageCheermoteFragment { cheermote { prefix bitsAmount } } } }
fragment T on ActivityFeedText { fragments { token { __typename
  ... on ActivityFeedTextToken { text } ... on ActivityFeedIntegerToken { number } ... on User { ...U } } } }
fragment A on ActivityFeedAlert { __typename id status createdAt
  ... on ActivityFeedFollowAlert { follower { ...U } }
  ... on ActivityFeedSubscriptionAlert { subscriber { ...U } tier }
  ... on ActivityFeedPrimeSubscriptionAlert { subscriber { ...U } }
  ... on ActivityFeedResubscriptionAlert { subscriber { ...U } tier totalDuration streakDuration messageContent { ...M } }
  ... on ActivityFeedPrimeResubscriptionAlert { subscriber { ...U } totalDuration messageContent { ...M } }
  ... on ActivityFeedIndividualGiftSubscriptionAlert { recipient { ...U } tier }
  ... on ActivityFeedCommunityGiftSubscriptionAlert { gifter { ...U } isAnonymous quantity tier }
  ... on ActivityFeedCheerAlert { cheerer { ...U } amount isAnonymous messageContent { ...M } }
  ... on ActivityFeedBitsSpendAlert { fromUser { ...U } bitsAmount featureType featureData {
    ... on BitsSpendCheerData { messageContent { ...M } }
    ... on BitsSpendAutomaticPowerUpData { messageContent { ...M } reward { powerUpType } }
    ... on BitsSpendCustomPowerUpData { messageContent { ...M } reward { title } } } }
  ... on ActivityFeedRaidAlert { raider { ...U } partySize }
  ... on ActivityFeedChannelPointsRedemptionAlert { redeemer { ...U } pointsSpent reward { title } messageContent { ...M } }
  ... on ActivityFeedHypeTrainAlert { level hypeTrainState }
  ... on ActivityFeedCreatorGoalAlert { goalState goalType description contributions {
    ... on ChannelGoalIntegerContributions { currentIntegerContributions: currentContributions targetIntegerContributions: targetContributions } } } }";

const STATUS: &str = "mutation S($input: UpdateActivityFeedAlertStatusInput!) {
  updateActivityFeedAlertStatus(input: $input) { alert { id status } error } }";

/// D1 The channel's last events, newest first, as GQL JSON nodes.
pub async fn history(http: &Http, auth: Auth<'_>, channel_id: &str) -> Result<Vec<Value>> {
    let vars = json!({ "channelID": channel_id, "limit": HISTORY_LIMIT });
    let data: Value = query(http, auth, HISTORY, vars).await?;
    Ok(nodes(data))
}

fn nodes(mut data: Value) -> Vec<Value> {
    match data["channel"]["activityFeed"]["events"]["edges"].take() {
        Value::Array(edges) => edges.into_iter().map(|mut e| e["node"].take()).filter(Value::is_object).collect(),
        _ => Vec::new(),
    }
}

/// D4 Plays an alert again (`replay`) or skips it, in Twitch's alert box.
pub async fn set_alert(http: &Http, auth: Auth<'_>, alert_id: &str, replay: bool) -> Result<()> {
    let status = if replay { "QUEUED" } else { "SKIPPED" };
    let vars = json!({ "input": { "eventID": alert_id, "status": status } });
    let data: Value = query(http, auth, STATUS, vars).await?;
    // The error is an enum of its own, without `code`.
    match data["updateActivityFeedAlertStatus"]["error"].as_str() {
        Some(code) => Err(Error::Refused(code.to_owned())),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_takes_the_nodes() {
        let data = json!({ "channel": { "activityFeed": { "events": { "edges": [
            { "node": { "id": "FOLLOW:a", "type": "FOLLOW" } }, { "node": null } ] } } } });
        let n = nodes(data);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0]["id"], "FOLLOW:a");
        assert!(nodes(json!({ "channel": null })).is_empty());
    }
}
