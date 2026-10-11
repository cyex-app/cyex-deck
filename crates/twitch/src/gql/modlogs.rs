//! GQL-only mod reads and writes: the viewer card's mod history and comments (M3), the
//! unban request queue (M2), blocked terms (1.48) and the chatter list (1.48). Raw
//! operations checked against Twitch's schema (research, phase 10).

use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use super::{Auth, query};
use crate::{Http, Result};

/// M3 A user's mod history in a channel, for the usercard and mod view.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModHistory {
    /// Messages the channel has logged from them (`1000+` shown as 1000).
    pub message_count: u32,
    pub bans: u32,
    pub timeouts: u32,
    pub warnings: u32,
    pub comments: Vec<ModComment>,
}

/// M3 A moderator's note on a user.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModComment {
    pub id: String,
    pub text: String,
    pub author: String,
    /// RFC 3339.
    pub at: String,
    /// From another channel that shares its comments.
    pub shared: bool,
}

const VIEWER_CARD: &str = "query Card($channelID: ID!, $targetID: ID!) {
  viewerCardModLogs(channelID: $channelID, targetID: $targetID) {
    messages(first: 1) { ... on ViewerCardModLogsMessagesConnection { count } }
    bans: targetedActions(first: 1, type: BAN) { ... on ModLogsTargetedActionsConnection { count } }
    timeouts: targetedActions(first: 1, type: TIMEOUT) { ... on ModLogsTargetedActionsConnection { count } }
    warns: targetedActions(first: 1, type: WARN) { ... on ModLogsTargetedActionsConnection { count } }
    comments(first: 100) { ... on ModLogsCommentConnection {
      edges { node { id text timestamp author { login displayName } } } } }
    sharedComments(first: 50) { ... on SharedModLogsCommentConnection {
      edges { node { id text timestamp author { login displayName } } } } }
  }
}";

fn count_of(v: &Value, field: &str) -> u32 {
    v.get(field)
        .and_then(|c| c.get("count"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32
}

fn comments_of(v: &Value, field: &str, shared: bool) -> Vec<ModComment> {
    v.get(field)
        .and_then(|c| c.get("edges"))
        .and_then(Value::as_array)
        .map(|edges| {
            edges
                .iter()
                .filter_map(|e| {
                    let n = e.get("node")?;
                    Some(ModComment {
                        id: n.get("id")?.as_str()?.to_owned(),
                        text: n.get("text")?.as_str().unwrap_or_default().to_owned(),
                        author: n
                            .get("author")
                            .and_then(|a| a.get("displayName").or_else(|| a.get("login")))
                            .and_then(Value::as_str)
                            .unwrap_or("?")
                            .to_owned(),
                        at: n.get("timestamp").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        shared,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Needs a token that moderates the channel; errors or empty otherwise.
pub async fn history(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str) -> Result<ModHistory> {
    let vars = json!({ "channelID": channel_id, "targetID": target_id });
    let data: Value = query(http, auth, VIEWER_CARD, vars).await?;
    let logs = data.get("viewerCardModLogs").cloned().unwrap_or(Value::Null);
    let mut comments = comments_of(&logs, "comments", false);
    comments.extend(comments_of(&logs, "sharedComments", true));
    Ok(ModHistory {
        message_count: count_of(&logs, "messages"),
        bans: count_of(&logs, "bans"),
        timeouts: count_of(&logs, "timeouts"),
        warnings: count_of(&logs, "warns"),
        comments,
    })
}

/// M3 Writes a mod comment on a user; `shareable` lets channels that share see it.
pub async fn add_comment(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str, text: &str, shareable: bool) -> Result<()> {
    const Q: &str = "mutation C($input: CreateModeratorCommentInput!) {
      createModeratorComment(input: $input) { comment { id } }
    }";
    let vars = json!({ "input": { "channelID": channel_id, "targetID": target_id, "text": text, "isShareable": shareable } });
    let _: Value = query(http, auth, Q, vars).await?;
    Ok(())
}

pub async fn delete_comment(http: &Http, auth: Auth<'_>, channel_id: &str, id: &str) -> Result<()> {
    const Q: &str = "mutation D($input: DeleteModeratorCommentInput!) {
      deleteModeratorComment(input: $input) { comment { id } }
    }";
    let vars = json!({ "input": { "ID": id, "channelID": channel_id } });
    let _: Value = query(http, auth, Q, vars).await?;
    Ok(())
}

/// M2 One unban request in the queue.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UnbanRequest {
    pub id: String,
    pub user_id: String,
    pub login: String,
    pub display: String,
    pub text: String,
    /// RFC 3339.
    pub created_at: String,
}

const UNBAN_QUEUE: &str = "query Q($name: String!) {
  channel(name: $name) { unbanRequests(first: 50, options: { order: NEWEST, status: PENDING }) {
    edges { node { id createdAt requesterMessage requester { id login displayName } } }
  } }
}";

/// M2 Pending unban requests, newest first. `name` is the channel login.
pub async fn unban_requests(http: &Http, auth: Auth<'_>, name: &str) -> Result<Vec<UnbanRequest>> {
    let data: Value = query(http, auth, UNBAN_QUEUE, json!({ "name": name })).await?;
    let edges = data
        .pointer("/channel/unbanRequests/edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(edges
        .iter()
        .filter_map(|e| {
            let n = e.get("node")?;
            let r = n.get("requester")?;
            Some(UnbanRequest {
                id: n.get("id")?.as_str()?.to_owned(),
                user_id: r.get("id")?.as_str()?.to_owned(),
                login: r.get("login")?.as_str().unwrap_or_default().to_owned(),
                display: r.get("displayName").and_then(Value::as_str).unwrap_or_default().to_owned(),
                text: n.get("requesterMessage").and_then(Value::as_str).unwrap_or_default().to_owned(),
                created_at: n.get("createdAt").and_then(Value::as_str).unwrap_or_default().to_owned(),
            })
        })
        .collect())
}

/// M2 Approves or denies an unban request, with a note the viewer sees.
pub async fn resolve_unban(http: &Http, auth: Auth<'_>, id: &str, approve: bool, note: &str) -> Result<()> {
    let op = if approve {
        "mutation A($input: ApproveUnbanRequestInput!) { approveUnbanRequest(input: $input) { unbanRequest { id } } }"
    } else {
        "mutation D($input: DenyUnbanRequestInput!) { denyUnbanRequest(input: $input) { unbanRequest { id } } }"
    };
    let _: Value = query(http, auth, op, json!({ "input": { "id": id, "resolverMessage": note } })).await?;
    Ok(())
}

/// 1.48, V101-35 The chatters in a channel, grouped, with the total Twitch reports.
#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Chatters {
    #[ts(type = "number")]
    pub count: u64,
    pub broadcasters: Vec<String>,
    pub moderators: Vec<String>,
    pub vips: Vec<String>,
    pub staff: Vec<String>,
    /// A random sample, A-Z, capped at 100 by Twitch, unless `complete`.
    pub viewers: Vec<String>,
    /// Everyone in chat is listed: a small chat, or the whole list a moderator gets.
    pub complete: bool,
}

impl Chatters {
    /// Names listed across the groups.
    pub fn listed(&self) -> usize {
        [&self.broadcasters, &self.moderators, &self.vips, &self.staff, &self.viewers]
            .iter()
            .map(|g| g.len())
            .sum()
    }
}

const CHATTERS: &str = "query C($name: String!) {
  channel(name: $name) { chatters {
    count
    broadcasters { login }
    moderators { login }
    vips { login }
    staff { login }
    viewers { login }
  } }
}";

fn logins(v: &Value, field: &str) -> Vec<String> {
    v.get(field)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|u| u.get("login")?.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// Works signed out with the Android TV client (the web client fails the integrity check).
/// `None` if the channel doesn't exist.
pub async fn chatters(http: &Http, auth: Auth<'_>, name: &str) -> Result<Option<Chatters>> {
    let data: Value = query(http, auth, CHATTERS, json!({ "name": name })).await?;
    Ok(read_chatters(&data))
}

fn read_chatters(data: &Value) -> Option<Chatters> {
    let channel = data.get("channel").filter(|c| !c.is_null())?;
    let c = channel.get("chatters").cloned().unwrap_or(Value::Null);
    let mut out = Chatters {
        count: c.get("count").and_then(Value::as_u64).unwrap_or(0),
        broadcasters: logins(&c, "broadcasters"),
        moderators: logins(&c, "moderators"),
        vips: logins(&c, "vips"),
        staff: logins(&c, "staff"),
        viewers: logins(&c, "viewers"),
        complete: false,
    };
    out.complete = out.listed() as u64 >= out.count;
    Some(out)
}

/// Reads whether this account may see the viewer-card mod logs for a channel.
pub async fn can_see_mod_logs(http: &Http, auth: Auth<'_>, name: &str) -> Result<bool> {
    const Q: &str = "query A($name: String!) { channel(name: $name) { moderationSettings { canAccessViewerCardModLogs } } }";
    let data: Value = query::<Value>(http, auth, Q, json!({ "name": name })).await?;
    Ok(data
        .pointer("/channel/moderationSettings/canAccessViewerCardModLogs")
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_counts_and_comments() {
        let data = json!({ "viewerCardModLogs": {
            "messages": { "count": 1200 },
            "bans": { "count": 2 },
            "timeouts": { "count": 5 },
            "warns": { "count": 1 },
            "comments": { "edges": [{ "node": { "id": "c1", "text": "watch this one", "timestamp": "2024-01-01T00:00:00Z",
              "author": { "login": "mod1", "displayName": "Mod1" } } }] },
            "sharedComments": { "edges": [{ "node": { "id": "c2", "text": "banned elsewhere", "timestamp": "2024-02-01T00:00:00Z",
              "author": { "login": "mod2", "displayName": "Mod2" } } }] }
        }});
        let logs = data.get("viewerCardModLogs").cloned().unwrap();
        assert_eq!(count_of(&logs, "messages"), 1200);
        assert_eq!(count_of(&logs, "bans"), 2);
        let mut comments = comments_of(&logs, "comments", false);
        comments.extend(comments_of(&logs, "sharedComments", true));
        assert_eq!(comments.len(), 2);
        assert_eq!((comments[0].author.as_str(), comments[0].shared), ("Mod1", false));
        assert!(comments[1].shared);
    }

    #[test]
    fn unban_queue_parses() {
        let data = json!({ "channel": { "unbanRequests": { "edges": [
            { "node": { "id": "r1", "createdAt": "2024-05-01T00:00:00Z", "requesterMessage": "sorry",
              "requester": { "id": "3", "login": "u", "displayName": "U" } } }
        ]}}});
        let edges = data.pointer("/channel/unbanRequests/edges").and_then(Value::as_array).unwrap();
        assert_eq!(edges.len(), 1);
        let n = &edges[0]["node"];
        assert_eq!(n["requester"]["login"], "u");
    }

    #[test]
    fn chatters_group() {
        let data = json!({ "channel": { "chatters": {
            "count": 37308, "broadcasters": [{ "login": "b" }], "moderators": [{ "login": "m1" }, { "login": "m2" }],
            "vips": [], "staff": [], "viewers": [{ "login": "v1" }]
        }}});
        let c = read_chatters(&data).unwrap();
        assert_eq!(c.count, 37308);
        assert_eq!(c.moderators, ["m1", "m2"]);
        assert!(c.vips.is_empty());
        assert_eq!(c.listed(), 4);
        assert!(!c.complete);
        let small = json!({ "channel": { "chatters": { "count": 1, "broadcasters": [{ "login": "b" }] } } });
        assert!(read_chatters(&small).unwrap().complete);
        assert_eq!(read_chatters(&json!({ "channel": null })), None);
    }
}
