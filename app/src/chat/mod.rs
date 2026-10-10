//! Chat (1.8, 1.9) wired to Tauri. One IRC session per identity (an account or
//! anonymous), one room per channel and identity, and views that stream a room's events
//! to a window in 50 ms batches. Tabs keep their rooms joined while no view shows them
//! (1.10). Parsing and room logic live in `cyex_chat`.

pub mod catalog;
mod creds;
pub mod feeds;
mod flush;
pub mod identity;
pub mod live;
mod mention;
pub mod peek;
pub mod points;
pub mod powerups;
pub mod record;
pub mod rules;
pub mod run;
mod send;
mod session;
pub mod translate;
pub mod usercard;
pub mod unread;
pub mod votes;

pub use send::send_to;
pub use session::retry_refused;

use unread::Unread;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use cyex_chat::build::Me;
use cyex_chat::filter::Filter;
use cyex_chat::irc::{Client, Window};
use cyex_chat::model::{ChatMessage, ConnStatus, NoticeKind, RoomEvent};
use cyex_chat::{Room, RoomSnapshot};
use cyex_settings::Partition;
use cyex_settings::schema::Chat as ChatSettings;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;
use ts_rs::TS;

use crate::connections::Connections;
use crate::settings::Settings;

/// Who reads a room: a signed-in account (by user id) or nobody.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Identity {
    Anon,
    Account(String),
}

pub type RoomKey = (Identity, String);
type View = Channel<Vec<RoomEvent>>;

/// Lines a view gets when it opens. The UI shows as many (`MAX_LINES`).
const VIEW_LINES: usize = 200;

pub struct Chat {
    inner: Mutex<Inner>,
    /// Wakes the flush task when a room has events waiting.
    wake: Arc<Notify>,
    /// C19 How long views hold messages back, in ms.
    delay_ms: AtomicI64,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<Identity, Session>,
    rooms: HashMap<RoomKey, Entry>,
    /// View id → its room and the window showing it.
    views: HashMap<u32, (RoomKey, String)>,
    next_view: u32,
    /// Rooms the tabs keep joined even when no view shows them.
    held: HashSet<RoomKey>,
}

struct Session {
    client: Client,
    /// Messages sent in the last 30 s (20, or 100 where we're mod or VIP).
    sends: Window,
}

/// A view and the filters it shows messages through (F3).
struct Sink {
    id: u32,
    view: View,
    filters: Vec<Arc<Filter>>,
}

struct Entry {
    room: Room,
    views: Vec<Sink>,
    outbox: Vec<RoomEvent>,
    /// C19 Messages held back until their time, oldest first.
    delayed: VecDeque<(i64, RoomEvent)>,
    /// C21 Last text we sent here, to dodge Twitch's duplicate check.
    last_sent: Option<(String, i64)>,
    /// U9 News since a view last showed it.
    unread: Unread,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatOpened {
    pub view: u32,
    /// The account reading this chat, `None` when anonymous.
    pub account: Option<String>,
    pub snapshot: RoomSnapshot,
}

pub fn init(app: &AppHandle) {
    let wake = Arc::new(Notify::new());
    app.manage(Chat {
        inner: Mutex::default(),
        wake: wake.clone(),
        delay_ms: AtomicI64::new(0),
    });
    on_settings(app);
    flush::spawn(app.clone(), wake);
    catalog::init(app);
    live::init(app);
    feeds::init(app);
    rules::init(app);
    run::init(app);
    powerups::init(app);
    record::init(app);
    translate::init(app);
    votes::init(app);
}

impl Chat {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The room a view shows.
    pub fn view_key(&self, view: u32) -> Option<RoomKey> {
        self.lock().views.get(&view).map(|v| v.0.clone())
    }
}

/// Twitch logins: letters, digits, underscores, up to 25.
fn valid_login(login: &str) -> bool {
    (1..=25).contains(&login.len()) && login.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn identity_for(app: &AppHandle, account: Option<&str>) -> (Identity, Option<Me>) {
    let accounts = crate::auth::accounts(app);
    match account.and_then(|id| accounts.get(id)) {
        Some(a) => (
            Identity::Account(a.user_id.clone()),
            Some(Me::new(&a.user_id, &a.login, &a.display_name)),
        ),
        None => (Identity::Anon, None),
    }
}

/// After chat or highlight settings changed.
pub fn on_settings(app: &AppHandle) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let delay = app.state::<Settings>().read::<ChatSettings>(Partition::Chat).delay_tenths;
    chat.delay_ms.store(i64::from(delay) * 100, Ordering::Relaxed);
    if app.try_state::<rules::RuleState>().is_some() {
        rules::rebuild(app);
    }
}

/// F3 The filters with these ids, ready. Ones that don't read are left out.
fn filters(app: &AppHandle, ids: &[String]) -> Vec<Arc<Filter>> {
    if ids.is_empty() {
        return Vec::new();
    }
    let defs = app.state::<Settings>().read::<cyex_settings::schema::Highlights>(Partition::Highlights).filters;
    defs.iter()
        .filter(|d| ids.contains(&d.id))
        .filter_map(|d| Filter::parse(&d.expr).ok().map(Arc::new))
        .collect()
}

/// Whether a view with these filters shows a message.
fn passes(filters: &[Arc<Filter>], m: &ChatMessage, channel: &str, live: bool) -> bool {
    filters.is_empty() || {
        let facts = cyex_chat::filter::Facts { msg: m, channel, live };
        filters.iter().all(|f| f.matches(&facts))
    }
}

/// Opens a view on a channel's chat, joining it if it isn't joined yet. `filter_ids`
/// pick the F3 filters it shows messages through.
pub fn open(
    app: &AppHandle,
    window: &str,
    login: &str,
    account: Option<&str>,
    filter_ids: &[String],
    view: View,
) -> Result<ChatOpened, String> {
    let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
    if !valid_login(&login) {
        return Err("That isn't a Twitch channel name.".into());
    }
    let (identity, me) = identity_for(app, account);
    let filters = filters(app, filter_ids);
    let live = !filters.is_empty() && crate::tabs::live::is_live(app, &login);
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let key = (identity.clone(), login.clone());
    let fresh = ensure_room(app, &mut inner, &key, me);
    inner.next_view += 1;
    let id = inner.next_view;
    inner.views.insert(id, (key.clone(), window.to_owned()));
    let entry = inner.rooms.get_mut(&key).expect("inserted above");
    let mut snapshot = entry.room.snapshot_tail(if filters.is_empty() { VIEW_LINES } else { cyex_chat::BUFFER });
    if !filters.is_empty() {
        snapshot.messages.retain(|m| passes(&filters, m, &login, live));
        let skip = snapshot.messages.len().saturating_sub(VIEW_LINES);
        snapshot.messages.drain(..skip);
    }
    entry.views.push(Sink { id, view, filters });
    let was_unread = std::mem::take(&mut entry.unread) != Unread::Seen;
    let opened = ChatOpened {
        view: id,
        account: match &identity {
            Identity::Account(id) => Some(id.clone()),
            Identity::Anon => None,
        },
        snapshot,
    };
    let budget_full = entry.room.status == ConnStatus::BudgetFull;
    drop(inner);
    if was_unread {
        unread::emit(app, &key, Unread::Seen);
    }
    if fresh {
        after_join(app, key, budget_full);
    } else {
        // Pins are only checked for rooms on screen, so this one may be old.
        catalog::refresh_pin_now(app.clone(), key);
    }
    if let Some(a) = &opened.account {
        crate::emotes::presence::touch(app, a, &login);
    }
    votes::watch(app, &login, opened.account.clone());
    Ok(opened)
}

/// Joins a room unless it's joined already. Returns true if it's new.
fn ensure_room(app: &AppHandle, inner: &mut Inner, key: &RoomKey, me: Option<Me>) -> bool {
    if inner.rooms.contains_key(key) {
        return false;
    }
    let (identity, login) = key;
    let mut room = Room::new(login, me);
    room.set_rules(rules::current(app));
    room.set_hook(crate::plugins::line_hook(app));
    let budget_ok = match identity {
        Identity::Anon => true,
        Identity::Account(id) => budget(app, |b| b.entry(id.clone()).or_default().join(login, false)),
    };
    if budget_ok {
        session::ensure(app, inner, identity).client.join(login);
    } else {
        room.set_status(ConnStatus::BudgetFull);
    }
    inner.rooms.insert(
        key.clone(),
        Entry {
            room,
            views: Vec::new(),
            outbox: Vec::new(),
            delayed: VecDeque::new(),
            last_sent: None,
            unread: Unread::Seen,
        },
    );
    true
}

/// Work for a new room once the lock is released: counter, catalogs, history.
fn after_join(app: &AppHandle, key: RoomKey, budget_full: bool) {
    crate::connections::notify(app);
    let history = app.state::<Settings>().read::<ChatSettings>(Partition::Chat).history;
    catalog::load_for_room(app.clone(), key, history && !budget_full);
}

/// Leaves a room. Returns true if an account's join count changed.
fn leave(app: &AppHandle, inner: &mut Inner, key: &RoomKey) -> bool {
    let Some(entry) = inner.rooms.remove(key) else { return false };
    if entry.unread != Unread::Seen {
        unread::emit(app, key, Unread::Seen);
    }
    let (identity, login) = key;
    if let Some(s) = inner.sessions.get(identity) {
        s.client.part(login);
    }
    let still_used = inner.rooms.keys().any(|(i, _)| i == identity);
    if !still_used && let Some(s) = inner.sessions.remove(identity) {
        s.client.stop();
    }
    if !inner.rooms.keys().any(|(_, l)| l == login) {
        crate::emotes::leave(app, login);
        live::left(app, login);
    }
    let Identity::Account(id) = identity else { return false };
    budget(app, |b| {
        if let Some(b) = b.get_mut(id) {
            b.part(login);
        }
    });
    true
}

/// Closes a view. A room no other view shows and no tab holds is left.
pub fn close(app: &AppHandle, view: u32) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let Some((key, _)) = inner.views.remove(&view) else { return };
    votes::unwatch(app, &key.1);
    let Some(entry) = inner.rooms.get_mut(&key) else { return };
    entry.views.retain(|s| s.id != view);
    if !entry.views.is_empty() || inner.held.contains(&key) {
        return;
    }
    let counted = leave(app, &mut inner, &key);
    drop(inner);
    if counted {
        crate::connections::notify(app);
    }
    // A closed channel may end this account's mod feeds (1.46).
    crate::moderation::feeds::sync(app);
}

/// 1.10 The rooms the tabs want joined, as (account, login). Joins new ones and leaves
/// the ones no tab holds and no view shows anymore.
pub fn hold(app: &AppHandle, wanted: &[(Option<String>, String)]) {
    let mut keys: HashMap<RoomKey, Option<Me>> = HashMap::new();
    for (account, login) in wanted {
        if !valid_login(login) {
            continue;
        }
        let (identity, me) = identity_for(app, account.as_deref());
        keys.entry((identity, login.clone())).or_insert(me);
    }
    let Some(chat) = app.try_state::<Chat>() else { return };
    let mut inner = chat.lock();
    let old = std::mem::replace(&mut inner.held, keys.keys().cloned().collect());
    let mut counted = false;
    for key in old.iter().filter(|k| !keys.contains_key(*k)) {
        if inner.rooms.get(key).is_some_and(|e| e.views.is_empty()) {
            counted |= leave(app, &mut inner, key);
        }
    }
    let mut joined = Vec::new();
    for (key, me) in keys {
        if ensure_room(app, &mut inner, &key, me) {
            let full = inner.rooms.get(&key).is_some_and(|e| e.room.status == ConnStatus::BudgetFull);
            joined.push((key, full));
        }
    }
    drop(inner);
    if counted && joined.is_empty() {
        crate::connections::notify(app);
    }
    for (key, full) in joined {
        after_join(app, key, full);
    }
    crate::moderation::feeds::sync(app);
}

/// Closes every view a window had, e.g. a popped-out chat that was closed.
pub fn close_window(app: &AppHandle, window: &str) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let views: Vec<u32> = chat
        .lock()
        .views
        .iter()
        .filter(|(_, (_, w))| w == window)
        .map(|(id, _)| *id)
        .collect();
    for v in views {
        close(app, v);
    }
}

/// A user's last 10 messages in a channel, from whichever open room has the most
/// (usercard, 1.36).
pub fn recent_from(app: &AppHandle, channel: &str, user_id: &str) -> Vec<Arc<ChatMessage>> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter(|((_, login), _)| login == channel)
        .map(|(_, e)| e.room.recent_from(user_id, 10))
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

/// A line only the user sees, in every view of one room.
pub fn local_line(app: &AppHandle, key: &RoomKey, kind: NoticeKind, text: &str) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let mut inner = chat.lock();
    if let Some(entry) = inner.rooms.get_mut(key) {
        let ev = entry.room.system(kind, text, now_ms());
        queue(entry, [ev], &chat.wake);
    }
}

/// 1.26 A whisper shown inline in every open room (Settings → Notifications).
pub fn whisper_line(app: &AppHandle, m: Arc<ChatMessage>) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let mut inner = chat.lock();
    for entry in inner.rooms.values_mut() {
        let ev = entry.room.local(m.clone());
        queue(entry, [ev], &chat.wake);
    }
}

/// 1.57 The account's own role flags in a channel, from its open room's USERSTATE.
pub fn own_state(app: &AppHandle, account: &str, login: &str) -> Option<cyex_chat::model::SelfState> {
    let chat = app.try_state::<Chat>()?;
    let key = (Identity::Account(account.to_owned()), login.to_ascii_lowercase());
    chat.lock().rooms.get(&key)?.room.state.own.clone()
}

/// The channel id for a login, from whichever open room knows it.
pub fn channel_id_of(app: &AppHandle, login: &str) -> Option<String> {
    let chat = app.try_state::<Chat>()?;
    let login = login.to_ascii_lowercase();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .find(|((_, l), _)| *l == login)
        .and_then(|(_, e)| e.room.state.channel_id.clone())
}

/// (account id, login) for every open room where the account is broadcaster or mod (1.46).
pub fn moderated_open(app: &AppHandle) -> Vec<(String, String)> {
    let Some(chat) = app.try_state::<Chat>() else { return Vec::new() };
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter_map(|((identity, login), e)| {
            let Identity::Account(acc) = identity else { return None };
            e.room.state.own.as_ref().filter(|o| o.privileged()).map(|_| (acc.clone(), login.clone()))
        })
        .collect()
}

/// A channel's buffered messages, from whichever open room has the most (for `/nuke`).
pub fn messages_of(app: &AppHandle, login: &str) -> Vec<Arc<ChatMessage>> {
    let Some(chat) = app.try_state::<Chat>() else { return Vec::new() };
    let login = login.to_ascii_lowercase();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter(|((_, l), _)| *l == login)
        .map(|(_, e)| e.room.snapshot().messages)
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

/// The room for `channel`, read by `identity` if it's joined that way, else any.
pub fn room_for(app: &AppHandle, identity: &Identity, channel: &str) -> Option<RoomKey> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    let own = (identity.clone(), channel.to_owned());
    if inner.rooms.contains_key(&own) {
        return Some(own);
    }
    inner.rooms.keys().find(|(_, l)| l == channel).cloned()
}

/// F1 Messages matching a search, oldest first. `view` searches its own chat; `None`
/// searches every joined chat (C29).
pub fn search(app: &AppHandle, view: Option<u32>, query: &str) -> Result<Vec<feeds::FeedLine>, String> {
    const MAX: usize = 300;
    let q = cyex_chat::search::Query::parse(query);
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    let only = match view {
        Some(v) => Some(inner.views.get(&v).map(|v| v.0.clone()).ok_or("This chat is closed.")?),
        None => None,
    };
    let mut out: Vec<feeds::FeedLine> = Vec::new();
    let mut seen = HashSet::new();
    for (key, entry) in &inner.rooms {
        if only.as_ref().is_some_and(|o| o != key) {
            continue;
        }
        for m in entry.room.snapshot().messages {
            if q.matches(&m, &key.1) && seen.insert(m.id.clone()) {
                out.push(feeds::FeedLine { channel: key.1.clone(), message: m });
            }
        }
    }
    out.sort_by_key(|l| l.message.ts);
    let skip = out.len().saturating_sub(MAX);
    out.drain(..skip);
    Ok(out)
}

/// Every joined channel, A-Z.
pub fn open_channels(app: &AppHandle) -> Vec<String> {
    let chat = app.state::<Chat>();
    let mut out: Vec<String> = chat.lock().rooms.keys().map(|(_, l)| l.clone()).collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// C10 A system line in every room of a channel, e.g. "x added the 7TV emote y."
pub fn system_line(app: &AppHandle, login: &str, text: &str) {
    notice_line(app, login, NoticeKind::Info, text);
}

/// A line of `kind` in every room of a channel.
pub fn notice_line(app: &AppHandle, login: &str, kind: NoticeKind, text: &str) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let mut inner = chat.lock();
    let now = now_ms();
    for ((_, l), entry) in inner.rooms.iter_mut() {
        if l == login {
            let ev = entry.room.system(kind, text, now);
            queue(entry, [ev], &chat.wake);
        }
    }
}

/// Every room of a channel, by login.
pub fn rooms_of(app: &AppHandle, login: &str) -> Vec<RoomKey> {
    let Some(chat) = app.try_state::<Chat>() else { return Vec::new() };
    let inner = chat.lock();
    inner.rooms.keys().filter(|(_, l)| l == login).cloned().collect()
}

/// C9 People's personal emotes just arrived: their last few seconds of messages are
/// read again, since they often chat right before their emotes show up.
pub fn retokenize_users(app: &AppHandle, users: &[String]) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let since = now_ms() - 15_000;
    let keys: Vec<RoomKey> = chat.lock().rooms.keys().cloned().collect();
    for key in keys {
        let emotes = crate::emotes::room(app, account_of(&key.0), &key.1);
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else { continue };
        for u in users {
            let events = entry.room.retokenize_from(u, since, &emotes);
            if !events.is_empty() {
                queue(entry, events, &chat.wake);
            }
        }
    }
}

/// The account reading a room, if any.
fn account_of(identity: &Identity) -> Option<&str> {
    match identity {
        Identity::Account(id) => Some(id),
        Identity::Anon => None,
    }
}

/// Queues events for a room's views and wakes the flush task.
fn queue(entry: &mut Entry, events: impl IntoIterator<Item = RoomEvent>, wake: &Notify) {
    let before = entry.outbox.len();
    entry.outbox.extend(events);
    if before == 0 && !entry.outbox.is_empty() {
        wake.notify_one();
    }
}

fn budget<T>(app: &AppHandle, f: impl FnOnce(&mut HashMap<String, cyex_chat::JoinBudget>) -> T) -> T {
    let conns = app.state::<Connections>();
    let mut map = conns.0.lock().unwrap_or_else(|p| p.into_inner());
    f(&mut map)
}

/// Unix milliseconds.
pub(crate) fn now_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}
