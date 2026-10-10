//! 1.48 The mod view's live data per channel: the activity feed (follows, subs, cheers,
//! raids), the mod-action log, the AutoMod queue (1.56), suspicious-user activity (M6) and
//! new followers. Fed by `feeds` from EventSub; snapshots and updates go to the UI.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// How many of each list to keep in memory.
const KEEP: usize = 200;

/// One line in the activity feed or mod-action log.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FeedItem {
    pub id: String,
    /// `follow`, `sub`, `cheer`, `raid`, `mod_action`, `shield`, `suspicious`.
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
    actions: VecDeque<FeedItem>,
    held: Vec<HeldItem>,
    suspicious: VecDeque<SuspiciousItem>,
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
    pub actions: Vec<FeedItem>,
    pub held: Vec<HeldItem>,
    pub suspicious: Vec<SuspiciousItem>,
}

pub fn snapshot(app: &AppHandle, channel_id: &str) -> Board0 {
    with(app, channel_id, |b| Board0 {
        activity: b.activity.iter().cloned().collect(),
        actions: b.actions.iter().cloned().collect(),
        held: b.held.clone(),
        suspicious: b.suspicious.iter().cloned().collect(),
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

pub fn add_action(app: &AppHandle, channel_id: &str, item: FeedItem) {
    with(app, channel_id, |b| push(&mut b.actions, item));
    emit(app, channel_id);
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
