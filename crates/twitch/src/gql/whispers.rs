//! 1.26 Whispers with history: threads, messages, sending, read state and thread options,
//! all GQL with the account's own token (Helix has no history). Shapes checked against
//! Twitch's schema 2026/10/10 (`research/twitch-api.md`, phase 12); mutations never run.
//! Live updates come from Hermes `whispers.<id>`, read by [`from_hermes`].

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Auth, query};
use crate::{Error, Http, Result};

const MESSAGE: &str = "fragment M on WhisperMessage { id nonce sentAt editedAt deletedAt \
    from { id login displayName chatColor } content { content emotes { emoteID from to } } }";
const THREAD: &str = "fragment T on WhisperThread { id unreadMessagesCount isArchived isMuted \
    spamInfo { lastMarkedNotSpamAt likelihood } lastMessage { ...M } \
    participants { id login displayName chatColor profileImageURL(width: 70) \
    self { whisperPermissions { receive } } } }";

/// Threads per page; twitch.tv asks for 10.
pub const THREADS_PAGE: u32 = 30;
/// Messages per page; twitch.tv asks for 20.
pub const MESSAGES_PAGE: u32 = 50;

/// Someone in a thread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub login: String,
    pub display: String,
    /// `#rrggbb`, or empty when never set.
    pub color: String,
    pub avatar: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmoteRange {
    pub id: String,
    /// Code point offsets, both inclusive, like the IRC `emotes` tag.
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// A uuid.
    pub id: String,
    /// What the sender's client set, so a sent whisper's echo can be matched.
    pub nonce: Option<String>,
    /// RFC 3339.
    pub sent_at: String,
    pub from: Person,
    pub text: String,
    pub emotes: Vec<EmoteRange>,
    pub edited: bool,
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    /// `<lower id>_<higher id>`.
    pub id: String,
    /// The other person.
    pub other: Person,
    pub unread: u32,
    pub archived: bool,
    pub muted: bool,
    /// Twitch thinks it's spam and nobody said otherwise.
    pub spam: bool,
    /// The other person can whisper you for 48 hours because you whispered them.
    pub temporary: bool,
    pub last: Option<Message>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Threads {
    pub threads: Vec<Thread>,
    /// For the next page; `None` at the end.
    pub cursor: Option<String>,
    pub block_strangers: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages {
    /// Newest first.
    pub messages: Vec<Message>,
    pub cursor: Option<String>,
}

/// Twitch's thread id for two users: the smaller id first, compared as numbers.
pub fn thread_id(a: &str, b: &str) -> String {
    let key = |s: &str| (s.len(), s.to_owned());
    if key(a) <= key(b) {
        format!("{a}_{b}")
    } else {
        format!("{b}_{a}")
    }
}

pub async fn threads(http: &Http, auth: Auth<'_>, me: &str, after: Option<&str>) -> Result<Threads> {
    let q = format!(
        "query($after: Cursor) {{ currentUser {{ id whisperSettings {{ isBlockingWhispersFromStrangers }} \
         whisperThreads(first: {THREADS_PAGE}, after: $after) {{ pageInfo {{ hasNextPage }} \
         edges {{ cursor node {{ ...T }} }} }} }} }} {THREAD} {MESSAGE}"
    );
    let data: ThreadsData = query(http, auth, &q, json!({ "after": after })).await?;
    let me_data = data.current_user.ok_or_else(signed_out)?;
    let conn = me_data.whisper_threads.unwrap_or_default();
    let cursor = more(&conn.page_info, conn.edges.last().map(|e| e.cursor.clone()));
    Ok(Threads {
        threads: conn
            .edges
            .into_iter()
            .filter_map(|e| e.node.and_then(|t| t.into_thread(me)))
            .collect(),
        cursor,
        block_strangers: me_data
            .whisper_settings
            .is_some_and(|s| s.is_blocking_whispers_from_strangers),
    })
}

/// A thread and a page of `first` messages. `None` when the two never whispered.
pub async fn thread(
    http: &Http,
    auth: Auth<'_>,
    me: &str,
    id: &str,
    first: u32,
    after: Option<&str>,
) -> Result<Option<(Thread, Messages)>> {
    let q = format!(
        "query($id: ID!, $first: Int, $after: Cursor) {{ whisperThread(id: $id) {{ ...T \
         messages(first: $first, after: $after) {{ pageInfo {{ hasNextPage }} \
         edges {{ cursor node {{ ...M }} }} }} }} }} {THREAD} {MESSAGE}"
    );
    let vars = json!({ "id": id, "first": first, "after": after });
    let data: ThreadData = query(http, auth, &q, vars).await?;
    let Some(raw) = data.whisper_thread else {
        return Ok(None);
    };
    let conn = raw.messages.clone().unwrap_or_default();
    let cursor = more(&conn.page_info, conn.edges.last().map(|e| e.cursor.clone()));
    let messages = conn
        .edges
        .into_iter()
        .filter_map(|e| e.node.map(RawMessage::into_message))
        .collect();
    Ok(raw.into_thread(me).map(|t| (t, Messages { messages, cursor })))
}

/// Sends a whisper; Twitch answers with the message as stored. Errors carry Twitch's code,
/// e.g. `TARGET_RESTRICTED` or `SENDER_NOT_VERIFIED`.
pub async fn send(
    http: &Http,
    auth: Auth<'_>,
    to_id: &str,
    text: &str,
    nonce: &str,
) -> Result<Option<Message>> {
    let q = format!(
        "mutation($input: SendWhisperInput!) {{ sendWhisper(input: $input) {{ \
         message {{ ...M }} error {{ code }} }} }} {MESSAGE}"
    );
    let vars = json!({ "input": { "message": text, "nonce": nonce, "recipientUserID": to_id } });
    let data: SendData = query(http, auth, &q, vars).await?;
    let payload = data.send_whisper.ok_or_else(|| api("NOT_DELIVERED"))?;
    if let Some(e) = payload.error {
        return Err(api(&e.code));
    }
    Ok(payload.message.map(RawMessage::into_message))
}

/// A change to one thread. Only the fields set are sent.
#[derive(Clone, Debug, Default)]
pub struct ThreadEdit<'a> {
    /// Marks everything up to this message (its uuid) read.
    pub read_up_to: Option<&'a str>,
    pub archived: Option<bool>,
    pub muted: Option<bool>,
    /// `false`: "this isn't spam"; `true`: report it as spam.
    pub spam: Option<bool>,
    /// Takes back the 48 hours a stranger got when you whispered them.
    pub revoke_access: bool,
}

pub async fn edit_thread(http: &Http, auth: Auth<'_>, id: &str, edit: &ThreadEdit<'_>) -> Result<()> {
    let mut input = json!({ "threadID": id });
    if let Some(m) = edit.read_up_to {
        input["lastReadMessageID"] = m.into();
    }
    if let Some(v) = edit.archived {
        input["isArchived"] = v.into();
    }
    if let Some(v) = edit.muted {
        input["isMuted"] = v.into();
    }
    if let Some(v) = edit.spam {
        input["isSpam"] = v.into();
    }
    if edit.revoke_access {
        input["removeWhitelist"] = true.into();
    }
    let q = "mutation($input: UpdateWhisperThreadInput!) { updateWhisperThread(input: $input) { thread { id } } }";
    query::<Value>(http, auth, q, json!({ "input": input }))
        .await
        .map(drop)
}

pub async fn mark_all_read(http: &Http, auth: Auth<'_>) -> Result<()> {
    let q = "mutation { updateAllWhisperThreads(input: {markAllAsRead: true}) { count } }";
    query::<Value>(http, auth, q, json!({})).await.map(drop)
}

/// Only people you follow or whispered can whisper you.
pub async fn block_strangers(http: &Http, auth: Auth<'_>, on: bool) -> Result<()> {
    let q = "mutation($on: Boolean!) { updateWhisperSettings(input: {isBlockingWhispersFromStrangers: $on}) { user { id } } }";
    query::<Value>(http, auth, q, json!({ "on": on })).await.map(drop)
}

/// A Hermes `whispers.<id>` message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Live {
    /// Received, or sent from any of your devices. `to` is set on sent ones when Twitch
    /// names the recipient.
    Message {
        thread_id: String,
        message: Box<Message>,
        to: Option<Person>,
    },
    Edited {
        thread_id: String,
        message_id: String,
        text: String,
    },
    Deleted {
        thread_id: String,
        message_id: String,
    },
    /// Read, archived or muted changed, here or on another device.
    Thread {
        thread_id: String,
    },
    AllRead,
}

/// Reads one Hermes whispers payload: `{type, data: "<json>", data_object: {…}}`.
pub fn from_hermes(payload: &Value) -> Option<Live> {
    let kind = payload.get("type")?.as_str()?;
    let data = match payload.get("data_object") {
        Some(d) if d.is_object() => d.clone(),
        _ => match payload.get("data")? {
            Value::String(s) => serde_json::from_str(s).ok()?,
            d => d.clone(),
        },
    };
    let s = |v: &Value| match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    };
    let thread_id = s(&data["thread_id"]);
    match kind {
        "whisper_received" | "whisper_sent" => {
            let tags = &data["tags"];
            let emotes = tags["emotes"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .filter_map(|e| {
                            let id = e.get("emote_id").or_else(|| e.get("id")).map(s)?;
                            Some(EmoteRange {
                                id,
                                from: e["start"].as_u64()? as usize,
                                to: e["end"].as_u64()? as usize,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let sent_at = data["sent_ts"]
                .as_i64()
                .map(crate::clock::rfc3339)
                .unwrap_or_default();
            let message = Message {
                id: s(&data["message_id"]),
                nonce: data["nonce"]
                    .as_str()
                    .filter(|n| !n.is_empty())
                    .map(str::to_owned),
                sent_at,
                from: Person {
                    id: s(&data["from_id"]),
                    login: s(&tags["login"]),
                    display: s(&tags["display_name"]),
                    color: s(&tags["color"]),
                    avatar: None,
                },
                text: s(&data["body"]),
                emotes,
                edited: false,
                deleted: false,
            };
            let r = &data["recipient"];
            let to = r.is_object().then(|| Person {
                id: s(&r["id"]),
                login: s(&r["username"]),
                display: s(&r["display_name"]),
                color: s(&r["color"]),
                avatar: r["profile_image"].as_str().map(str::to_owned),
            });
            (!thread_id.is_empty() && !message.id.is_empty()).then(|| Live::Message {
                thread_id,
                message: Box::new(message),
                to,
            })
        }
        "whisper_edited" => Some(Live::Edited {
            thread_id,
            message_id: s(&data["message_id"]),
            text: s(&data["body"]),
        }),
        "whisper_deleted" => Some(Live::Deleted {
            thread_id,
            message_id: s(&data["message_id"]),
        }),
        "thread" => Some(Live::Thread {
            thread_id: s(&data["id"]),
        }),
        "threads" => Some(Live::AllRead),
        _ => None,
    }
}

fn signed_out() -> Error {
    api("unauthenticated")
}

fn api(code: &str) -> Error {
    Error::Api {
        status: 200,
        message: code.to_owned(),
    }
}

fn more(info: &Option<PageInfo>, last: Option<String>) -> Option<String> {
    info.as_ref()
        .is_some_and(|p| p.has_next_page)
        .then_some(last)
        .flatten()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadsData {
    current_user: Option<MeData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeData {
    whisper_settings: Option<Settings>,
    whisper_threads: Option<Connection<RawThread>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    is_blocking_whispers_from_strangers: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadData {
    whisper_thread: Option<RawThread>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendData {
    send_whisper: Option<SendPayload>,
}

#[derive(Deserialize)]
struct SendPayload {
    message: Option<RawMessage>,
    error: Option<Code>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Connection<T> {
    #[serde(default = "Vec::new")]
    edges: Vec<Edge<T>>,
    page_info: Option<PageInfo>,
}

impl<T> Default for Connection<T> {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            page_info: None,
        }
    }
}

#[derive(Clone, Deserialize)]
struct Edge<T> {
    #[serde(default)]
    cursor: String,
    node: Option<T>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    #[serde(default)]
    has_next_page: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawThread {
    id: String,
    #[serde(default)]
    unread_messages_count: u32,
    #[serde(default)]
    is_archived: bool,
    #[serde(default)]
    is_muted: bool,
    spam_info: Option<SpamInfo>,
    last_message: Option<RawMessage>,
    #[serde(default)]
    participants: Vec<Option<RawPerson>>,
    messages: Option<Connection<RawMessage>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpamInfo {
    last_marked_not_spam_at: Option<String>,
    likelihood: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPerson {
    id: String,
    #[serde(default)]
    login: String,
    #[serde(default)]
    display_name: String,
    chat_color: Option<String>,
    #[serde(rename = "profileImageURL")]
    profile_image_url: Option<String>,
    #[serde(rename = "self")]
    me: Option<PersonSelf>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersonSelf {
    whisper_permissions: Option<Permissions>,
}

#[derive(Clone, Deserialize)]
struct Permissions {
    receive: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMessage {
    id: String,
    nonce: Option<String>,
    #[serde(default)]
    sent_at: String,
    edited_at: Option<String>,
    deleted_at: Option<String>,
    from: Option<RawPerson>,
    content: Option<Content>,
}

#[derive(Clone, Deserialize)]
struct Content {
    #[serde(default)]
    content: String,
    #[serde(default)]
    emotes: Vec<Option<RawEmote>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEmote {
    #[serde(rename = "emoteID")]
    emote_id: Option<String>,
    from: Option<usize>,
    to: Option<usize>,
}

impl RawPerson {
    fn into_person(self) -> Person {
        Person {
            id: self.id,
            login: self.login,
            display: self.display_name,
            color: self.chat_color.unwrap_or_default(),
            avatar: self.profile_image_url,
        }
    }
}

impl RawMessage {
    fn into_message(self) -> Message {
        let content = self.content.unwrap_or(Content {
            content: String::new(),
            emotes: Vec::new(),
        });
        Message {
            id: self.id,
            nonce: self.nonce.filter(|n| !n.is_empty()),
            sent_at: self.sent_at,
            from: self.from.map(RawPerson::into_person).unwrap_or_default(),
            text: content.content,
            emotes: content
                .emotes
                .into_iter()
                .flatten()
                .filter_map(|e| {
                    Some(EmoteRange {
                        id: e.emote_id?,
                        from: e.from?,
                        to: e.to?,
                    })
                })
                .collect(),
            edited: self.edited_at.is_some(),
            deleted: self.deleted_at.is_some(),
        }
    }
}

impl RawThread {
    /// `None` when the other person is gone (deleted account).
    fn into_thread(self, me: &str) -> Option<Thread> {
        let other = self.participants.into_iter().flatten().find(|p| p.id != me)?;
        let temporary = other
            .me
            .as_ref()
            .and_then(|s| s.whisper_permissions.as_ref())
            .and_then(|p| p.receive.as_deref())
            == Some("TEMPORARY");
        let spam = self.spam_info.is_some_and(|s| {
            s.last_marked_not_spam_at.is_none() && s.likelihood.as_deref().is_some_and(|l| l != "LOW")
        });
        Some(Thread {
            id: self.id,
            other: other.into_person(),
            unread: self.unread_messages_count,
            archived: self.is_archived,
            muted: self.is_muted,
            spam,
            temporary,
            last: self.last_message.map(RawMessage::into_message),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_ids_sort_numerically() {
        assert_eq!(thread_id("9", "10"), "9_10");
        assert_eq!(thread_id("10", "9"), "9_10");
        assert_eq!(thread_id("123", "45"), "45_123");
    }

    #[test]
    fn threads_parse() {
        let d: ThreadsData = serde_json::from_str(
            r##"{"currentUser":{"id":"1","whisperSettings":{"isBlockingWhispersFromStrangers":true},
              "whisperThreads":{"pageInfo":{"hasNextPage":true},"edges":[{"cursor":"c1","node":{
                "id":"1_2","unreadMessagesCount":3,"isArchived":false,"isMuted":true,
                "spamInfo":{"lastMarkedNotSpamAt":null,"likelihood":"HIGH"},
                "lastMessage":{"id":"m1","nonce":"n","sentAt":"2026-10-10T10:00:00Z","editedAt":null,"deletedAt":null,
                  "from":{"id":"2","login":"bob","displayName":"Bob","chatColor":"#FF0000"},
                  "content":{"content":"hi Kappa","emotes":[{"emoteID":"25","from":3,"to":7}]}},
                "participants":[{"id":"1","login":"me","displayName":"Me"},
                  {"id":"2","login":"bob","displayName":"Bob","chatColor":null,"profileImageURL":"p",
                   "self":{"whisperPermissions":{"receive":"TEMPORARY"}}}]}}]}}}"##,
        )
        .unwrap();
        let me = d.current_user.unwrap();
        assert!(me.whisper_settings.unwrap().is_blocking_whispers_from_strangers);
        let conn = me.whisper_threads.unwrap();
        assert_eq!(more(&conn.page_info, Some("c1".into())).as_deref(), Some("c1"));
        let t = conn
            .edges
            .into_iter()
            .next()
            .unwrap()
            .node
            .unwrap()
            .into_thread("1")
            .unwrap();
        assert_eq!(
            (t.other.login.as_str(), t.unread, t.muted, t.spam, t.temporary),
            ("bob", 3, true, true, true)
        );
        let last = t.last.unwrap();
        assert_eq!(last.text, "hi Kappa");
        assert_eq!(
            last.emotes,
            [EmoteRange {
                id: "25".into(),
                from: 3,
                to: 7
            }]
        );
        assert_eq!(last.from.color, "#FF0000");
    }

    #[test]
    fn hermes_received_and_thread_events() {
        let p: Value = serde_json::from_str(
            r##"{"type":"whisper_received","data":"{}","data_object":{"id":41,"message_id":"u-1",
              "thread_id":"1_2","body":"yo 4Head","sent_ts":1760090400,"from_id":2,"nonce":"",
              "tags":{"login":"bob","display_name":"Bob","color":"#00FF00","emotes":[{"emote_id":"354","start":3,"end":7}]}}}"##,
        )
        .unwrap();
        let Some(Live::Message {
            thread_id,
            message,
            to,
        }) = from_hermes(&p)
        else {
            panic!()
        };
        assert_eq!(
            (thread_id.as_str(), message.from.id.as_str(), to),
            ("1_2", "2", None)
        );
        assert_eq!(
            (message.nonce, message.sent_at.as_str()),
            (None, "2025-10-10T10:00:00Z")
        );
        assert_eq!(message.emotes[0].id, "354");
        let t: Value = serde_json::from_str(
            r#"{"type":"thread","data":"{\"id\":\"1_2\",\"last_read\":41,\"archived\":false,\"muted\":false}"}"#,
        )
        .unwrap();
        assert_eq!(
            from_hermes(&t),
            Some(Live::Thread {
                thread_id: "1_2".into()
            })
        );
        assert_eq!(
            from_hermes(&json!({"type":"threads","data_object":{"mark_all_read":true}})),
            Some(Live::AllRead)
        );
        assert_eq!(from_hermes(&json!({"type":"other","data_object":{}})), None);
    }

    #[test]
    fn send_errors_carry_the_code() {
        let d: SendData =
            serde_json::from_str(r#"{"sendWhisper":{"message":null,"error":{"code":"TARGET_RESTRICTED"}}}"#)
                .unwrap();
        assert_eq!(d.send_whisper.unwrap().error.unwrap().code, "TARGET_RESTRICTED");
    }
}
