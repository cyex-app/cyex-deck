//! GQL-only mod reads and writes: the viewer card's mod history and comments (M3), a
//! user's messages from the mod logs (M8), the unban request queue (M2), blocked terms
//! (1.48) and the chatter list (1.48). Raw operations checked against Twitch's schema
//! (research, phase 10).

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
    /// V101-05 Their bans, timeouts and warnings here, newest first.
    pub actions: Vec<ModAction>,
    /// V101-05 Banned or timed out right now.
    pub ban: Option<BanStatus>,
}

/// V101-05 One ban, timeout or warning from the channel's mod logs.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModAction {
    pub id: String,
    /// `BAN`, `TIMEOUT` or `WARN`.
    pub kind: String,
    /// RFC 3339.
    pub at: String,
    /// Twitch's own wording, joined from its label.
    pub text: String,
}

/// V101-05 A ban or timeout in force.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BanStatus {
    /// RFC 3339; `None` for a ban.
    pub until: Option<String>,
    pub by: Option<String>,
    pub reason: String,
    /// V108-C36 When it was given, RFC 3339 (Twitch's `createdAt`).
    #[ts(optional = nullable)]
    pub since: Option<String>,
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

/// V101-05 The lists as the 7TV extension reads them (schema-checked 2026/10/11).
const VIEWER_CARD: &str = "query Card($channelID: ID!, $targetID: ID!) {
  ban: chatRoomBanStatus(channelID: $channelID, userID: $targetID) {
    createdAt expiresAt isPermanent reason moderator { login displayName } }
  viewerCardModLogs(channelID: $channelID, targetID: $targetID) {
    messages(first: 1000) { ... on ViewerCardModLogsMessagesConnection { count } }
    bans: targetedActions(first: 99, type: BAN) { ...Actions }
    timeouts: targetedActions(first: 99, type: TIMEOUT) { ...Actions }
    warns: targetedActions(first: 99, type: WARN) { ...Actions }
    comments(first: 100) { ... on ModLogsCommentConnection {
      edges { node { id text timestamp author { login displayName } } } } }
    sharedComments(first: 50) { ... on SharedModLogsCommentConnection {
      edges { node { id text timestamp author { login displayName } } } } }
  }
}
fragment Actions on ModLogsTargetedActionsResult {
  ... on ModLogsTargetedActionsConnection { count edges { node { id timestamp type
    localizedLabel { fallbackString localizedStringFragments { token {
      ... on ModActionsLocalizedTextToken { text } ... on User { login displayName } } } } } } }
}";

/// M8 A page of someone's messages from the channel's mod logs, newest first. Twitch keeps
/// them for mods even where logs.cyex.app has none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModLogPage {
    pub messages: Vec<ModLogMessage>,
    /// For the next, older page.
    pub cursor: Option<String>,
    pub more: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModLogMessage {
    /// Twitch's chat message id, the same as in logs.cyex.app.
    pub id: String,
    /// RFC 3339, with nanoseconds.
    pub sent_at: String,
    pub user_id: String,
    pub login: String,
    pub display: String,
    pub color: Option<String>,
    /// `(set, version)`.
    pub badges: Vec<(String, String)>,
    /// Text pieces, each with the Twitch emote id it shows, if any.
    pub pieces: Vec<(String, Option<String>)>,
    pub deleted: bool,
}

/// M8 `ViewerCardModLogsMessagesBySender`, as Twitch's viewer card asks it (research,
/// v1.01 batch 7). Only chat messages: actions have their own tabs.
const MESSAGES: &str = "query Messages($channelID: ID!, $senderID: ID!, $cursor: Cursor) {
  viewerCardModLogs(channelID: $channelID, targetID: $senderID) {
    messages(first: 50, after: $cursor) {
      ... on ViewerCardModLogsMessagesConnection {
        edges { cursor node { __typename
          ... on ViewerCardModLogsChatMessage { id sentAt isDeleted
            sender { id login displayName chatColor }
            senderBadges(channelID: $channelID) { setID version }
            content { text fragments { text content { __typename ... on Emote { emoteID: id } } } } } } }
        pageInfo { hasNextPage }
      }
    }
  }
}";

/// M8 A page of `sender_id`'s messages in the channel; `cursor` from the page before.
pub async fn messages(
    http: &Http,
    auth: Auth<'_>,
    channel_id: &str,
    sender_id: &str,
    cursor: Option<&str>,
) -> Result<ModLogPage> {
    let vars = json!({ "channelID": channel_id, "senderID": sender_id, "cursor": cursor });
    let data: Value = query(http, auth, MESSAGES, vars).await?;
    Ok(page_of(&data))
}

fn page_of(data: &Value) -> ModLogPage {
    let conn = &data["viewerCardModLogs"]["messages"];
    let edges = conn["edges"].as_array().map(Vec::as_slice).unwrap_or_default();
    let str_of = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    let messages = edges
        .iter()
        .filter_map(|e| {
            let n = &e["node"];
            if n["__typename"] != "ViewerCardModLogsChatMessage" {
                return None;
            }
            let sender = &n["sender"];
            let fragments = n["content"]["fragments"].as_array().map(Vec::as_slice).unwrap_or_default();
            let mut pieces: Vec<(String, Option<String>)> = fragments
                .iter()
                .map(|f| {
                    let emote = (f["content"]["__typename"] == "Emote")
                        .then(|| f["content"]["emoteID"].as_str().map(str::to_owned))
                        .flatten();
                    (str_of(&f["text"]), emote)
                })
                .collect();
            if pieces.is_empty() {
                pieces.push((str_of(&n["content"]["text"]), None));
            }
            Some(ModLogMessage {
                id: n["id"].as_str()?.to_owned(),
                sent_at: str_of(&n["sentAt"]),
                user_id: str_of(&sender["id"]),
                login: str_of(&sender["login"]),
                display: sender["displayName"].as_str().unwrap_or(sender["login"].as_str().unwrap_or("?")).to_owned(),
                color: sender["chatColor"].as_str().filter(|c| !c.is_empty()).map(str::to_owned),
                badges: n["senderBadges"]
                    .as_array()
                    .map(|b| b.iter().map(|b| (str_of(&b["setID"]), str_of(&b["version"]))).collect())
                    .unwrap_or_default(),
                pieces,
                deleted: n["isDeleted"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    // Every edge on a page carries the same cursor; the last one is safe either way.
    let cursor = edges.last().and_then(|e| e["cursor"].as_str()).map(str::to_owned);
    ModLogPage {
        messages,
        more: conn["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false) && cursor.is_some(),
        cursor,
    }
}

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

fn actions_of(v: &Value, field: &str) -> Vec<ModAction> {
    let edges = v.get(field).and_then(|c| c.get("edges")).and_then(Value::as_array);
    edges
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let n = e.get("node")?;
            let label = n.get("localizedLabel");
            let joined: String = label
                .and_then(|l| l.get("localizedStringFragments"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    let t = f.get("token")?;
                    t.get("text").or_else(|| t.get("displayName")).or_else(|| t.get("login"))?.as_str()
                })
                .collect();
            let text = if joined.trim().is_empty() {
                label.and_then(|l| l.get("fallbackString")).and_then(Value::as_str).unwrap_or_default().to_owned()
            } else {
                joined
            };
            Some(ModAction {
                id: n.get("id")?.as_str()?.to_owned(),
                kind: n.get("type").and_then(Value::as_str).unwrap_or_default().to_owned(),
                at: n.get("timestamp").and_then(Value::as_str).unwrap_or_default().to_owned(),
                text: text.trim().to_owned(),
            })
        })
        .collect()
}

fn ban_of(data: &Value) -> Option<BanStatus> {
    let b = data.get("ban").filter(|b| b.is_object())?;
    let permanent = b.get("isPermanent").and_then(Value::as_bool).unwrap_or(false);
    Some(BanStatus {
        until: if permanent { None } else { b.get("expiresAt").and_then(Value::as_str).map(str::to_owned) },
        by: b
            .get("moderator")
            .and_then(|m| m.get("displayName").or_else(|| m.get("login")))
            .and_then(Value::as_str)
            .map(str::to_owned),
        reason: b.get("reason").and_then(Value::as_str).unwrap_or_default().to_owned(),
        since: b.get("createdAt").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned),
    })
}

/// V108-N16 One channel's ban, timeout and warning counts for a user (read only).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModCounts {
    pub channel_id: String,
    pub bans: u32,
    pub timeouts: u32,
    pub warnings: u32,
}

impl ModCounts {
    pub fn any(&self) -> bool {
        self.bans + self.timeouts + self.warnings > 0
    }
}

/// Counts only, so it stays cheap over many channels. Same schema as [`VIEWER_CARD`].
const COUNTS: &str = "query Counts($channelID: ID!, $targetID: ID!) {
  viewerCardModLogs(channelID: $channelID, targetID: $targetID) {
    bans: targetedActions(first: 1, type: BAN) { ...N }
    timeouts: targetedActions(first: 1, type: TIMEOUT) { ...N }
    warns: targetedActions(first: 1, type: WARN) { ...N }
  }
}
fragment N on ModLogsTargetedActionsResult { ... on ModLogsTargetedActionsConnection { count } }";

/// V108-N16 `target_id`'s ban, timeout and warning counts in each of `channel_ids` (the
/// channels the account moderates), batched [`super::BATCH_MAX`] per request. Channels where
/// Twitch refuses, or with nothing on them, are left out, and so is a request that fails (it
/// is logged); only when every request fails is that an error. Needs a token that moderates them.
pub async fn counts_in(http: &Http, auth: Auth<'_>, channel_ids: &[String], target_id: &str) -> Result<Vec<ModCounts>> {
    let mut out = Vec::new();
    let mut failed = None;
    let mut answered = false;
    for chunk in channel_ids.chunks(super::BATCH_MAX) {
        let ops: Vec<(&str, Value)> =
            chunk.iter().map(|c| (COUNTS, json!({ "channelID": c, "targetID": target_id }))).collect();
        let answers = match super::batch(http, auth, &ops).await {
            Ok(a) => a,
            Err(e) => {
                eprintln!("mod counts: {e}");
                failed = Some(e);
                continue;
            }
        };
        answered = true;
        for (id, answer) in chunk.iter().zip(answers) {
            if let Ok(data) = answer {
                let c = counts_of(id, &data);
                if c.any() {
                    out.push(c);
                }
            }
        }
    }
    // Every request failed: say so, rather than "no history anywhere".
    if let Some(e) = failed
        && !answered
    {
        return Err(e);
    }
    Ok(out)
}

fn counts_of(channel_id: &str, data: &Value) -> ModCounts {
    let logs = &data["viewerCardModLogs"];
    ModCounts {
        channel_id: channel_id.to_owned(),
        bans: count_of(logs, "bans"),
        timeouts: count_of(logs, "timeouts"),
        warnings: count_of(logs, "warns"),
    }
}

/// Needs a token that moderates the channel; errors or empty otherwise.
pub async fn history(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str) -> Result<ModHistory> {
    let vars = json!({ "channelID": channel_id, "targetID": target_id });
    let data: Value = query(http, auth, VIEWER_CARD, vars).await?;
    let logs = data.get("viewerCardModLogs").cloned().unwrap_or(Value::Null);
    let mut comments = comments_of(&logs, "comments", false);
    comments.extend(comments_of(&logs, "sharedComments", true));
    let mut actions: Vec<ModAction> = ["bans", "timeouts", "warns"].iter().flat_map(|f| actions_of(&logs, f)).collect();
    // RFC 3339 in UTC sorts as text.
    actions.sort_by(|a, b| b.at.cmp(&a.at));
    Ok(ModHistory {
        message_count: count_of(&logs, "messages"),
        bans: count_of(&logs, "bans"),
        timeouts: count_of(&logs, "timeouts"),
        warnings: count_of(&logs, "warns"),
        comments,
        actions,
        ban: ban_of(&data),
    })
}

/// V107-M7 The ban or timeout on a user right now, `None` when there is none. Mods only.
pub async fn ban_status(http: &Http, auth: Auth<'_>, channel_id: &str, target_id: &str) -> Result<Option<BanStatus>> {
    const Q: &str = "query Ban($channelID: ID!, $targetID: ID!) {
  ban: chatRoomBanStatus(channelID: $channelID, userID: $targetID) {
    createdAt expiresAt isPermanent reason moderator { login displayName } } }";
    let vars = json!({ "channelID": channel_id, "targetID": target_id });
    let data: Value = query(http, auth, Q, vars).await?;
    Ok(ban_of(&data))
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

/// V108-C36 One of a user's unban requests in a channel, any status, for the request's
/// "Earlier requests".
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PastUnban {
    pub id: String,
    /// RFC 3339.
    pub created_at: String,
    pub text: String,
    /// `PENDING`, `APPROVED`, `DENIED`, `ACKNOWLEDGED` or `CANCELED`.
    pub status: String,
    /// RFC 3339; `None` while pending.
    pub resolved_at: Option<String>,
    /// The mod who resolved it, display name first.
    pub resolved_by: Option<String>,
    /// The note they left for the user.
    pub note: String,
}

/// V108-C36 `options.userID` narrows Twitch's list to one requester; no status: every one
/// (validated against the schema 2026/11/03).
const UNBAN_PAST: &str = "query P($id: ID!, $userID: ID!) { channel(id: $id) {
  unbanRequests(first: 20, options: { order: NEWEST, userID: $userID }) {
    edges { node { id createdAt requesterMessage status resolvedAt resolverMessage resolvedBy { login displayName } } }
  } } }";

/// V108-C36 Every unban request `user_id` made in the channel, newest first.
pub async fn unban_past(http: &Http, auth: Auth<'_>, channel_id: &str, user_id: &str) -> Result<Vec<PastUnban>> {
    let data: Value = query(http, auth, UNBAN_PAST, json!({ "id": channel_id, "userID": user_id })).await?;
    Ok(past_of(&data))
}

fn past_of(data: &Value) -> Vec<PastUnban> {
    let edges = data.pointer("/channel/unbanRequests/edges").and_then(Value::as_array);
    let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    edges
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let n = e.get("node")?;
            Some(PastUnban {
                id: n.get("id")?.as_str()?.to_owned(),
                created_at: text(&n["createdAt"]),
                text: text(&n["requesterMessage"]),
                status: text(&n["status"]),
                resolved_at: n["resolvedAt"].as_str().filter(|s| !s.is_empty()).map(str::to_owned),
                resolved_by: ["displayName", "login"]
                    .iter()
                    .find_map(|k| n["resolvedBy"][k].as_str().filter(|s| !s.is_empty()))
                    .map(str::to_owned),
                note: text(&n["resolverMessage"]),
            })
        })
        .collect()
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

/// V103-15 One line of a channel's mod-action history (`moderationActionLogs`), read
/// signed in with the TV client (checked 2026/10/11 on two channels). Twitch words each
/// line itself: the title names the target (or says what changed), the content says what
/// was done and by whom ("Banned by @mod"), the body holds a reason or a note.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChannelAction {
    pub id: String,
    /// RFC 3339, with nanoseconds.
    pub at: String,
    /// `BAN`, `SWORD`, `HALT`, `AUTOMOD`, …
    pub icon: String,
    /// `BANS_AND_UNBANS`, `MODS_AND_VIPS`, `CHAT_MODE_CHANGES`, `AUTOMOD_LEVEL_CHANGES`, …
    pub category: String,
    /// Who did it: `(login, display name)`.
    pub moderator: Option<(String, String)>,
    /// The title's user, when it names one: `(login, display name)`.
    pub target: Option<(String, String)>,
    /// The title's own text when it names no user, e.g. "Clear Chat".
    pub title: String,
    /// The content without its trailing "by @mod", e.g. "Banned".
    pub done: String,
    pub reason: String,
    pub note: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChannelActionsPage {
    pub actions: Vec<ChannelAction>,
    /// For the next, older page.
    pub cursor: Option<String>,
}

const CHANNEL_ACTIONS: &str = "query Actions($id: ID!, $after: Cursor) { channel(id: $id) {
  moderationActionLogs(first: 50, after: $after) {
    ... on ModerationActionLogsError { code }
    ... on ModerationActionLogsConnection { pageInfo { hasNextPage } edges { cursor node {
      id createdAt icon filterCategoryID
      details { ... on ModerationActionBanDetails { reason } }
      user { ... on User { login displayName } }
      title { ...Text } content { ...Text } contentBody { ...Text } } } }
  } } }
fragment Text on ModActionsLocalizedText { localizedStringFragments { token {
  __typename ... on ModActionsLocalizedTextToken { text } ... on User { login displayName } } } }";

/// V103-15 A page of the channel's mod actions, newest first; `after` pages back.
pub async fn channel_actions(http: &Http, auth: Auth<'_>, channel_id: &str, after: Option<&str>) -> Result<ChannelActionsPage> {
    let data: Value = query(http, auth, CHANNEL_ACTIONS, json!({ "id": channel_id, "after": after })).await?;
    read_channel_actions(&data)
}

/// A user token as `(login, display name)`.
fn person(t: &Value) -> Option<(String, String)> {
    let login = t.get("login")?.as_str()?.to_owned();
    let name = t.get("displayName").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or(&login).to_owned();
    Some((login, name))
}

fn tokens(v: &Value) -> &[Value] {
    v.get("localizedStringFragments").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn text_of(ts: &[Value]) -> String {
    ts.iter()
        .filter_map(|f| {
            let t = f.get("token")?;
            t.get("text").or_else(|| t.get("displayName")).or_else(|| t.get("login"))?.as_str()
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn read_channel_actions(data: &Value) -> Result<ChannelActionsPage> {
    let logs = data.get("channel").and_then(|c| c.get("moderationActionLogs")).unwrap_or(&Value::Null);
    if let Some(code) = logs.get("code").and_then(Value::as_str) {
        return Err(crate::Error::Refused(code.to_owned()));
    }
    let edges = logs.get("edges").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    let more = logs.pointer("/pageInfo/hasNextPage").and_then(Value::as_bool).unwrap_or(false);
    let actions = edges
        .iter()
        .filter_map(|e| {
            let n = e.get("node")?;
            let text = |k: &str| n.get(k).map(tokens).unwrap_or_default();
            let title = text("title");
            let content = text("content");
            let target = title.iter().find_map(|f| person(f.get("token")?));
            // "Banned by @mod": the words before the last user.
            let by = content.iter().rposition(|f| f.pointer("/token/login").is_some());
            let done = text_of(&content[..by.unwrap_or(content.len())]);
            let done = done.strip_suffix(" by").unwrap_or(&done).trim().to_owned();
            Some(ChannelAction {
                id: n.get("id")?.as_str()?.to_owned(),
                at: n.get("createdAt").and_then(Value::as_str).unwrap_or_default().to_owned(),
                icon: n.get("icon").and_then(Value::as_str).unwrap_or_default().to_owned(),
                category: n.get("filterCategoryID").and_then(Value::as_str).unwrap_or_default().to_owned(),
                moderator: n.get("user").and_then(person),
                title: if target.is_some() { String::new() } else { text_of(title) },
                target,
                done,
                reason: n.pointer("/details/reason").and_then(Value::as_str).unwrap_or_default().trim().to_owned(),
                note: text_of(text("contentBody")),
            })
        })
        .collect();
    let cursor = edges.last().and_then(|e| e.get("cursor")).and_then(Value::as_str).filter(|_| more).map(str::to_owned);
    Ok(ChannelActionsPage { actions, cursor })
}

#[cfg(test)]
mod tests {
    #[test]
    fn v108_n16_counts_read_per_channel() {
        let d = json!({ "viewerCardModLogs": { "bans": { "count": 1 }, "timeouts": { "count": 4 }, "warns": {} } });
        let c = counts_of("77", &d);
        assert_eq!((c.bans, c.timeouts, c.warnings), (1, 4, 0));
        assert!(c.any());
        assert!(!counts_of("77", &json!({ "viewerCardModLogs": null })).any());
    }

    use super::*;

    #[test]
    fn mod_log_messages_read_newest_first_with_emotes() {
        let data = json!({ "viewerCardModLogs": { "messages": {
            "edges": [
                { "cursor": "c1", "node": { "__typename": "ViewerCardModLogsChatMessage", "id": "m2",
                    "sentAt": "2026-08-22T10:16:21.180548189Z", "isDeleted": true,
                    "sender": { "id": "7", "login": "vay", "displayName": "Vay", "chatColor": null },
                    "senderBadges": [{ "setID": "subscriber", "version": "12" }],
                    "content": { "text": "hi Kappa", "fragments": [
                        { "text": "hi ", "content": null },
                        { "text": "Kappa", "content": { "__typename": "Emote", "emoteID": "25" } } ] } } },
                { "cursor": "c1", "node": { "__typename": "ViewerCardModLogsModActionsMessage" } },
                { "cursor": "c1", "node": { "__typename": "ViewerCardModLogsChatMessage", "id": "m1",
                    "sentAt": "2026-08-22T10:00:00Z", "isDeleted": false,
                    "sender": { "id": "7", "login": "vay", "displayName": null, "chatColor": "#FF0000" },
                    "senderBadges": null, "content": { "text": "first", "fragments": null } } }
            ],
            "pageInfo": { "hasNextPage": true } } } });
        let p = page_of(&data);
        assert_eq!(p.messages.len(), 2);
        let m = &p.messages[0];
        assert_eq!((m.id.as_str(), m.deleted, m.color.as_deref()), ("m2", true, None));
        assert_eq!(m.pieces, [("hi ".to_owned(), None), ("Kappa".to_owned(), Some("25".to_owned()))]);
        assert_eq!(m.badges, [("subscriber".to_owned(), "12".to_owned())]);
        let old = &p.messages[1];
        assert_eq!((old.display.as_str(), old.color.as_deref()), ("vay", Some("#FF0000")));
        assert_eq!(old.pieces, [("first".to_owned(), None)]);
        assert_eq!((p.cursor.as_deref(), p.more), (Some("c1"), true));
        // No access or nothing logged: an empty connection, no error.
        let none = page_of(&json!({ "viewerCardModLogs": { "messages": { "edges": [], "pageInfo": { "hasNextPage": false } } } }));
        assert_eq!(none, ModLogPage::default());
    }

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
    fn actions_read_twitchs_wording() {
        let logs = json!({ "timeouts": { "count": 1, "edges": [{ "node": { "id": "a1", "timestamp": "2026-01-02T00:00:00Z",
            "type": "TIMEOUT", "localizedLabel": { "fallbackString": "{0} timed out {1}", "localizedStringFragments": [
              { "token": { "displayName": "Mod1", "login": "mod1" } },
              { "token": { "text": " timed out " } },
              { "token": { "displayName": "U", "login": "u" } },
              { "token": { "text": " for 10m" } }] } } }] },
          "bans": { "count": 1, "edges": [{ "node": { "id": "a2", "timestamp": "2026-01-01T00:00:00Z", "type": "BAN",
            "localizedLabel": { "fallbackString": "Banned", "localizedStringFragments": [] } } }] } });
        let t = actions_of(&logs, "timeouts");
        assert_eq!((t[0].kind.as_str(), t[0].text.as_str()), ("TIMEOUT", "Mod1 timed out U for 10m"));
        assert_eq!(actions_of(&logs, "bans")[0].text, "Banned");
        assert!(actions_of(&logs, "warns").is_empty());
    }

    #[test]
    fn ban_status_reads_timeouts_and_bans() {
        let t = json!({ "ban": { "expiresAt": "2026-01-01T00:10:00Z", "isPermanent": false, "reason": "",
            "moderator": { "login": "m", "displayName": "M" } } });
        let b = ban_of(&t).unwrap();
        assert_eq!((b.until.as_deref(), b.by.as_deref()), (Some("2026-01-01T00:10:00Z"), Some("M")));
        let p = json!({ "ban": { "expiresAt": null, "isPermanent": true, "reason": "spam", "moderator": null } });
        assert!(ban_of(&p).unwrap().until.is_none());
        assert!(ban_of(&json!({ "ban": null })).is_none());
        // V108-C36 When it was given.
        let s = json!({ "ban": { "createdAt": "2026-11-01T10:00:00Z", "isPermanent": true, "reason": "" } });
        assert_eq!(ban_of(&s).unwrap().since.as_deref(), Some("2026-11-01T10:00:00Z"));
        assert!(ban_of(&p).unwrap().since.is_none());
    }

    #[test]
    fn v108_c36_past_unban_requests_read_every_status() {
        let data = json!({ "channel": { "unbanRequests": { "edges": [
            { "node": { "id": "r2", "createdAt": "2026-11-02T00:00:00Z", "requesterMessage": "again",
              "status": "PENDING", "resolvedAt": null, "resolverMessage": null, "resolvedBy": null } },
            { "node": { "id": "r1", "createdAt": "2026-10-01T00:00:00Z", "requesterMessage": "sorry",
              "status": "DENIED", "resolvedAt": "2026-10-02T00:00:00Z", "resolverMessage": "no",
              "resolvedBy": { "login": "m", "displayName": "" } } },
            { "node": null }
        ]}}});
        let p = past_of(&data);
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].status.as_str(), p[0].resolved_at.as_deref(), p[0].resolved_by.as_deref()), ("PENDING", None, None));
        assert_eq!(p[0].note, "");
        // An empty display name falls back to the login.
        assert_eq!((p[1].resolved_by.as_deref(), p[1].note.as_str()), (Some("m"), "no"));
        assert!(past_of(&json!({ "channel": null })).is_empty());
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

    #[test]
    fn channel_actions_read_twitchs_wording() {
        // V103-15 As Twitch answered on 2026/10/11, names changed.
        let user = |l: &str| json!({ "token": { "__typename": "User", "login": l, "displayName": l.to_uppercase() } });
        let word = |t: &str| json!({ "token": { "__typename": "ModActionsLocalizedTextToken", "text": t } });
        let data = json!({ "channel": { "moderationActionLogs": {
            "pageInfo": { "hasNextPage": true },
            "edges": [
                { "cursor": "c1", "node": { "id": "1", "createdAt": "2026-10-11T17:12:28.25Z", "icon": "BAN",
                  "filterCategoryID": "BANS_AND_UNBANS", "details": { "reason": " spam " },
                  "user": { "login": "kuni", "displayName": "Kuni" },
                  "title": { "localizedStringFragments": [user("x")] },
                  "content": { "localizedStringFragments": [word("Banned by "), user("kuni")] },
                  "contentBody": { "localizedStringFragments": [] } } },
                { "cursor": "c2", "node": { "id": "2", "createdAt": "2026-08-19T11:56:51Z", "icon": "HALT",
                  "filterCategoryID": "CHAT_MODE_CHANGES", "details": null,
                  "user": { "login": "kuni", "displayName": "" },
                  "title": { "localizedStringFragments": [word("Clear Chat")] },
                  "content": { "localizedStringFragments": [word("Done by "), user("kuni")] },
                  "contentBody": { "localizedStringFragments": [word("Chat was cleared.")] } } }
            ] } } });
        let page = read_channel_actions(&data).unwrap();
        assert_eq!(page.cursor.as_deref(), Some("c2"));
        let [ban, clear] = &page.actions[..] else { panic!() };
        assert_eq!((ban.done.as_str(), ban.reason.as_str()), ("Banned", "spam"));
        assert_eq!(ban.target, Some(("x".into(), "X".into())));
        assert_eq!(ban.moderator, Some(("kuni".into(), "Kuni".into())));
        assert_eq!((clear.title.as_str(), clear.done.as_str(), clear.note.as_str()), ("Clear Chat", "Done", "Chat was cleared."));
        assert_eq!(clear.moderator, Some(("kuni".into(), "kuni".into())));
        assert!(clear.target.is_none());
        // The last page has no cursor; Twitch's own refusal comes back as one.
        let last = json!({ "channel": { "moderationActionLogs": { "pageInfo": { "hasNextPage": false }, "edges": [] } } });
        assert_eq!(read_channel_actions(&last).unwrap(), ChannelActionsPage::default());
        let no = json!({ "channel": { "moderationActionLogs": { "code": "FORBIDDEN" } } });
        assert!(matches!(read_channel_actions(&no), Err(crate::Error::Refused(c)) if c == "FORBIDDEN"));
    }
}
