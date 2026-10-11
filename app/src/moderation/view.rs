//! 1.48 The mod view's live data per channel: the activity feed (follows, unban requests,
//! suspicious users), the mod actions (V103-15, also the stream manager's), the AutoMod
//! queue (1.56), suspicious-user activity (M6) and new followers. Fed by `feeds` from
//! EventSub or Hermes and by Twitch's history; snapshots and updates go to the UI.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use cyex_moderation::actions::{self, ModLine};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// How many of each list to keep in memory.
const KEEP: usize = 200;
/// V103-15 Mod actions kept per channel, live and from the history together.
const KEEP_ACTIONS: usize = 300;

/// One line in the activity feed.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FeedItem {
    pub id: String,
    /// `follow`, `unban_request`.
    pub kind: String,
    pub text: String,
    #[ts(type = "number")]
    pub ts: i64,
    /// The user it's about, for click-through.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub user: Option<String>,
}

/// A message AutoMod is holding (1.56).
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeldItem {
    pub message_id: String,
    pub user_id: String,
    pub login: String,
    pub display: String,
    pub text: String,
    /// `automod` category, or `blocked term`.
    pub reason: String,
    #[ts(type = "number")]
    pub ts: i64,
}

/// A monitored or restricted user's recent message (M6).
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SuspiciousItem {
    pub user_id: String,
    pub login: String,
    pub display: String,
    /// `active_monitoring` or `restricted`.
    pub status: String,
    pub text: String,
    #[ts(type = "number")]
    pub ts: i64,
}

#[derive(Default)]
struct Board {
    activity: VecDeque<FeedItem>,
    actions: VecDeque<ModLine>,
    held: Vec<HeldItem>,
    suspicious: VecDeque<SuspiciousItem>,
    history: History,
}

/// V103-15 How far Twitch's history of the channel's mod actions has been read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum History {
    #[default]
    Unread,
    Reading,
    /// More, older pages from this cursor.
    More(String),
    Done,
}

/// All channels' boards. One per channel id with a mod view or mod feeds open.
#[derive(Default)]
pub struct Boards(Mutex<HashMap<String, Board>>);

pub fn init(app: &AppHandle) {
    app.manage(Boards::default());
}

fn with<R>(app: &AppHandle, channel_id: &str, f: impl FnOnce(&mut Board) -> R) -> Option<R> {
    let boards = app.try_state::<Boards>()?;
    let mut map = boards.0.lock().unwrap_or_else(|p| p.into_inner());
    Some(f(map.entry(channel_id.to_owned()).or_default()))
}

fn push<T: Clone>(q: &mut VecDeque<T>, item: T) {
    q.push_front(item);
    q.truncate(KEEP);
}

/// A snapshot of one channel's board for the mod view when it opens.
#[derive(Clone, Debug, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Board0 {
    pub activity: Vec<FeedItem>,
    /// V103-15 Newest first.
    pub actions: Vec<ModLine>,
    pub held: Vec<HeldItem>,
    pub suspicious: Vec<SuspiciousItem>,
    /// V103-15 Twitch has older mod actions than these.
    pub older: bool,
}

pub fn snapshot(app: &AppHandle, channel_id: &str) -> Board0 {
    with(app, channel_id, |b| Board0 {
        activity: b.activity.iter().cloned().collect(),
        actions: b.actions.iter().cloned().collect(),
        held: b.held.clone(),
        suspicious: b.suspicious.iter().cloned().collect(),
        older: matches!(b.history, History::More(_)),
    })
    .unwrap_or_default()
}

fn emit(app: &AppHandle, channel_id: &str) {
    let _ = app.emit(&format!("mod:view:{channel_id}"), snapshot(app, channel_id));
}

pub fn add_activity(app: &AppHandle, channel_id: &str, item: FeedItem) {
    with(app, channel_id, |b| push(&mut b.activity, item));
    emit(app, channel_id);
}

/// V103-15 A live mod action; the same one from a second source shows once.
pub fn add_action(app: &AppHandle, channel_id: &str, line: ModLine) {
    if with(app, channel_id, |b| actions::push(&mut b.actions, line, KEEP_ACTIONS)) == Some(true) {
        emit(app, channel_id);
    }
}

/// V103-15 Where the next page of history starts: `Some(None)` for the first, `None` when
/// there's nothing (more) to read or a read is running. Marks it as reading.
pub fn history_next(app: &AppHandle, channel_id: &str, older: bool) -> Option<Option<String>> {
    with(app, channel_id, |b| {
        let next = match (&b.history, older) {
            (History::Unread, _) => Some(None),
            (History::More(c), true) => Some(Some(c.clone())),
            _ => None,
        };
        if next.is_some() {
            b.history = History::Reading;
        }
        next
    })
    .flatten()
}

/// V103-15 A page of history read (`cursor` for the next), or a failed read (`None`: it
/// can be asked again).
pub fn add_history(app: &AppHandle, channel_id: &str, page: Option<(Vec<ModLine>, Option<String>)>, was: Option<String>) {
    with(app, channel_id, |b| match page {
        Some((lines, cursor)) => {
            actions::merge(&mut b.actions, lines, KEEP_ACTIONS);
            b.history = cursor.map_or(History::Done, History::More);
        }
        None => b.history = was.map_or(History::Unread, History::More),
    });
    emit(app, channel_id);
}

/// 1.56 A held message's sender and text, for the decision on it.
pub fn held_one(app: &AppHandle, channel_id: &str, message_id: &str) -> Option<HeldItem> {
    with(app, channel_id, |b| b.held.iter().find(|h| h.message_id == message_id).cloned()).flatten()
}

pub fn add_held(app: &AppHandle, channel_id: &str, item: HeldItem) {
    with(app, channel_id, |b| {
        b.held.retain(|h| h.message_id != item.message_id);
        b.held.insert(0, item);
        b.held.truncate(KEEP);
    });
    emit(app, channel_id);
}

/// A held message was allowed, denied or expired; drop it from the queue.
pub fn resolve_held(app: &AppHandle, channel_id: &str, message_id: &str) {
    with(app, channel_id, |b| b.held.retain(|h| h.message_id != message_id));
    emit(app, channel_id);
}

pub fn add_suspicious(app: &AppHandle, channel_id: &str, item: SuspiciousItem) {
    with(app, channel_id, |b| push(&mut b.suspicious, item));
    emit(app, channel_id);
}
