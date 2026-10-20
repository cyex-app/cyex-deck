//! Chat (1.8, 1.9) wired to Tauri. One IRC session per identity (an account or
//! anonymous), one room per channel and identity, and views that stream a room's events
//! to a window in 50 ms batches. Tabs keep their rooms joined while no view shows them
//! (1.10). Parsing and room logic live in `cyex_chat`.

pub mod catalog;
mod creds;
pub mod feeds;
mod flush;
pub mod global;
pub mod identity;
pub mod live;
mod mention;
pub mod peek;
pub mod points;
pub mod powerups;
pub mod record;
mod refill;
pub mod roles;
pub mod rules;
pub mod run;
pub mod search;
mod send;
mod session;
pub mod shown;
pub mod translate;
pub mod typed;
pub mod usercard;
pub mod unread;
pub mod votes;
mod writer;

use writer::Writer;

pub use send::send_to;
pub use session::retry_refused;

use unread::Unread;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};
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

use crate::settings::Settings;

/// Who reads a room: a signed-in account (by user id) or nobody.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Identity {
    Anon,
    Account(String),
}

pub type RoomKey = (Identity, String);
type View = Channel<Vec<RoomEvent>>;

pub struct Chat {
    inner: Mutex<Inner>,
    /// Wakes the flush task when a room has events waiting.
    wake: Arc<Notify>,
    /// C19 How long views hold messages back, in ms.
    delay_ms: AtomicI64,
    /// V101-02 Messages each chat keeps; a view gets as many when it opens.
    scrollback: AtomicUsize,
    /// C37 Joins and leaves as lines.
    joins: AtomicBool,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<Identity, Session>,
    rooms: HashMap<RoomKey, Entry>,
    /// View id → its room and the window showing it.
    views: HashMap<u32, (RoomKey, String)>,
    /// V103-22 Views that are a mod view's or the stream manager's own chat: they follow the
    /// regular chats (`shown`) and don't count as one.
    follows: HashSet<u32>,
    next_view: u32,
    /// Rooms the tabs keep joined even when no view shows them.
    held: HashSet<RoomKey>,
}

struct Session {
    /// The reader: joins every channel and gets every line.
    client: Client,
    /// V105-06 Sends for an account go out here, so the reading connection gets our lines back
    /// from Twitch with every tag. `None` for anonymous reading.
    writer: Option<Arc<Writer>>,
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
    /// V105-05 When our JOIN for this room last went out (ms), to time Twitch's answer.
    join_sent: Option<i64>,
    /// V107-M1 Views opened on it so far, so a shrink planned at a close knows it's stale.
    shown: u64,
    /// V107-M1 Its first history is in (or failed): a refill won't race it.
    loaded: bool,
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
        scrollback: AtomicUsize::new(cyex_chat::BUFFER),
        joins: AtomicBool::new(false),
    });
    on_settings(app);
    shown::init(app);
    search::init(app);
    flush::spawn(app.clone(), wake);
    catalog::init(app);
    live::init(app);
    feeds::init(app);
    rules::init(app);
    global::init(app);
    run::init(app);
    powerups::init(app);
    record::init(app);
    translate::init(app);
    votes::init(app);
    roles::init(app);
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
    let c = app.state::<Settings>().read::<ChatSettings>(Partition::Chat);
    chat.delay_ms.store(i64::from(c.delay_tenths) * 100, Ordering::Relaxed);
    translate::on_settings(app, &c.translate_chats);
    // C37 Connections ask for them when they log in. Asked on an open connection, Twitch
    // sent nothing for the chats already joined (V104-04), so open ones log in again.
    if chat.joins.swap(c.joins_leaves, Ordering::Relaxed) != c.joins_leaves && c.joins_leaves {
        for s in chat.lock().sessions.values() {
            s.client.reconnect();
        }
    }
    let keep = c.scrollback as usize;
    if chat.scrollback.swap(keep, Ordering::Relaxed) != keep {
        for entry in chat.lock().rooms.values_mut() {
            entry.room.set_keep(keep);
        }
    }
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
/// pick the F3 filters it shows messages through. `follows`: a view's own chat panel
/// (V103-22), not a regular chat.
pub fn open(
    app: &AppHandle,
    window: &str,
    login: &str,
    account: Option<&str>,
    filter_ids: &[String],
    follows: bool,
    view: View,
) -> Result<ChatOpened, String> {
    let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
    if !valid_login(&login) {
        return Err("That isn’t a Twitch channel name.".into());
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
    if follows {
        inner.follows.insert(id);
    }
    let entry = inner.rooms.get_mut(&key).expect("inserted above");
    // V107-M1 Shown again after the background: the kept lines now, older ones bit by bit.
    let refill = refill::on_open(fresh, entry.loaded, &entry.room);
    entry.room.set_background(false);
    entry.shown += 1;
    let lines = chat.scrollback.load(Ordering::Relaxed);
    let mut snapshot = entry.room.snapshot_tail(if filters.is_empty() { lines } else { entry.room.keep() });
    if !filters.is_empty() {
        snapshot.messages.retain(|m| passes(&filters, m, &login, live));
        let skip = snapshot.messages.len().saturating_sub(lines);
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
    drop(inner);
    if !follows {
        shown::changed(app);
    }
    if was_unread {
        unread::emit(app, &key, Unread::Seen);
    }
    if refill {
        refill::start(app, key.clone());
    }
    if fresh {
        after_join(app, key);
    } else {
        // Pins are only checked for rooms on screen, so this one may be old.
        catalog::refresh_pin_now(app.clone(), key);
    }
    if let Some(a) = &opened.account {
        crate::emotes::presence::touch(app, a, &login, crate::emotes::presence::Why::Opened, &[]);
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
    room.set_keep(app.state::<Chat>().scrollback.load(Ordering::Relaxed));
    // V107-M1 Small until a view opens.
    room.set_background(true);
    room.set_rules(rules::current(app));
    room.set_hook(crate::plugins::line_hook(app));
    session::ensure(app, inner, identity).client.join(login);
    inner.rooms.insert(
        key.clone(),
        Entry {
            room,
            views: Vec::new(),
            outbox: Vec::new(),
            delayed: VecDeque::new(),
            last_sent: None,
            unread: Unread::Seen,
            join_sent: None,
            shown: 0,
            loaded: false,
        },
    );
    true
}

/// Work for a new room once the lock is released: counter, catalogs, history.
fn after_join(app: &AppHandle, key: RoomKey) {
    crate::connections::notify(app);
    roles::emit(app);
    roles::check_soon(app);
    let c = app.state::<Settings>().read::<ChatSettings>(Partition::Chat);
    let history = c.history.then_some(c.history_lines);
    catalog::load_for_room(app.clone(), key, history);
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
        if let Some(w) = &s.writer {
            w.client.stop();
        }
    }
    if !inner.rooms.keys().any(|(_, l)| l == login) {
        channel_gone(app, inner, login);
    }
    matches!(identity, Identity::Account(_))
}

/// V107-R1 The last room of `login` is gone: what was joined for the channel goes too.
fn channel_gone(app: &AppHandle, inner: &Inner, login: &str) {
    crate::emotes::leave(app, login);
    live::left(app, login);
    catalog::retain(app, |l| inner.rooms.keys().any(|(_, open)| open == l));
}

/// V107-R1 After a join's loads came back: whether the room is still open. A channel whose
/// last room closed meanwhile is left again, since the loads joined it after its leave.
fn still_joined(app: &AppHandle, key: &RoomKey) -> bool {
    let Some(chat) = app.try_state::<Chat>() else { return false };
    let inner = chat.lock();
    if !inner.rooms.keys().any(|(_, l)| *l == key.1) {
        channel_gone(app, &inner, &key.1);
        return false;
    }
    inner.rooms.contains_key(key)
}

/// Closes a view. A room no other view shows and no tab holds is left.
pub fn close(app: &AppHandle, view: u32) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let Some((key, _)) = inner.views.remove(&view) else { return };
    if !inner.follows.remove(&view) {
        shown::changed(app);
    }
    votes::unwatch(app, &key.1);
    let Some(entry) = inner.rooms.get_mut(&key) else { return };
    entry.views.retain(|s| s.id != view);
    if !entry.views.is_empty() {
        return;
    }
    let shown = entry.shown;
    if inner.held.contains(&key) {
        drop(inner);
        refill::later(app, key, shown);
        return;
    }
    let counted = leave(app, &mut inner, &key);
    drop(inner);
    if counted {
        crate::connections::notify(app);
    }
    // A closed channel may end this account's mod feeds (1.46).
    crate::moderation::feeds::sync(app);
    roles::emit(app);
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
            joined.push(key);
        }
    }
    drop(inner);
    if counted && joined.is_empty() {
        crate::connections::notify(app);
    }
    for key in joined {
        after_join(app, key);
    }
    crate::moderation::feeds::sync(app);
    roles::emit(app);
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

/// A user's messages still loaded in a channel, from whichever open room has the most
/// (usercard, 1.36).
pub fn recent_from(app: &AppHandle, channel: &str, user_id: &str) -> Vec<Arc<ChatMessage>> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    inner
        .rooms
        .iter()
        .filter(|((_, login), _)| login == channel)
        // V101-05 All of theirs still loaded; the card scrolls.
        .map(|(_, e)| e.room.recent_from(user_id, usize::MAX))
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

/// S7 A plugin's line in every room of the channel, anonymous and signed in. `false`:
/// none is open.
pub fn plugin_line(app: &AppHandle, login: &str, m: Arc<ChatMessage>) -> bool {
    each_room(app, login, |room| vec![room.local(m.clone())])
}

/// S7 A plugin's line swapped for a new one. `false`: it's gone.
pub fn plugin_replace(app: &AppHandle, login: &str, m: Arc<ChatMessage>) -> bool {
    each_room(app, login, |room| room.replace(m.clone()).into_iter().collect())
}

/// S7 A plugin's change to any line. `false`: it's gone.
pub fn plugin_change(app: &AppHandle, login: &str, id: &str, change: &cyex_chat::LineChange) -> bool {
    let emotes = crate::emotes::room(app, None, login);
    each_room(app, login, |room| room.change(id, change, &emotes))
}

/// S14 A plugin's own line taken back. `false`: it's gone.
pub fn plugin_remove(app: &AppHandle, login: &str, id: &str) -> bool {
    each_room(app, login, |room| room.remove(id).into_iter().collect())
}

/// S14 A chat emptied on screen by a plugin, with a line saying which. `false`: not open.
pub fn plugin_clear(app: &AppHandle, login: &str, text: &str) -> bool {
    let now = now_ms();
    each_room(app, login, |room| vec![room.wipe(), room.system(NoticeKind::Info, text, now)])
}

/// Runs `f` on every room of a channel and queues what comes back. `true` when any did.
fn each_room(app: &AppHandle, login: &str, mut f: impl FnMut(&mut cyex_chat::Room) -> Vec<RoomEvent>) -> bool {
    let Some(chat) = app.try_state::<Chat>() else { return false };
    let mut inner = chat.lock();
    let mut any = false;
    for ((_, l), entry) in inner.rooms.iter_mut() {
        if l != login {
            continue;
        }
        let events = f(&mut entry.room);
        any |= !events.is_empty();
        queue(entry, events, &chat.wake);
    }
    any
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
            e.room.role().map(|_| (acc.clone(), login.clone()))
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

/// V107-M7 A mod-action timeout line, stacked with that user's timeouts just before it.
pub fn timeout_line(app: &AppHandle, login: &str, user_id: &str, text: &str) {
    let Some(chat) = app.try_state::<Chat>() else { return };
    let mut inner = chat.lock();
    let now = now_ms();
    for ((_, l), entry) in inner.rooms.iter_mut() {
        if l == login {
            // A second delivery (another mod account's feed) brings nothing.
            let ev = entry.room.timeout_line(user_id, text, now);
            queue(entry, ev, &chat.wake);
        }
    }
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

/// 1.13 Per account: chats open (refused ones not) and the logins Twitch refused (V105-05).
/// Takes the chat lock: callers must not hold it.
pub(crate) fn account_rooms(app: &AppHandle) -> HashMap<String, (u32, Vec<String>)> {
    let mut out: HashMap<String, (u32, Vec<String>)> = HashMap::new();
    let Some(chat) = app.try_state::<Chat>() else { return out };
    for ((identity, login), e) in &chat.lock().rooms {
        let Identity::Account(id) = identity else { continue };
        let n = out.entry(id.clone()).or_default();
        if e.room.status == ConnStatus::Refused {
            n.1.push(login.clone());
        } else {
            n.0 += 1;
        }
    }
    for n in out.values_mut() {
        n.1.sort();
    }
    out
}

/// Unix milliseconds.
pub(crate) fn now_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

/// V107-R2 Test builds only: the chat stores' sizes for the memory probe (`perf_probe`).
#[cfg(feature = "mocks")]
pub fn probe(app: &AppHandle) -> serde_json::Value {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    let (mut outbox, mut delayed, mut sinks) = (0, 0, 0);
    for e in inner.rooms.values() {
        outbox += e.outbox.capacity();
        delayed += e.delayed.len();
        sinks += e.views.len();
    }
    serde_json::json!({
        "rooms": inner.rooms.len(), "sessions": inner.sessions.len(), "views": inner.views.len(),
        "held": inner.held.len(), "sinks": sinks, "outboxCap": outbox, "delayed": delayed,
    })
}
