//! Chat (1.8, 1.9) wired to Tauri. One IRC session per identity (an account or
//! anonymous), one room per channel and identity, and views that stream a room's events
//! to a window in 50 ms batches. Parsing and room logic live in `cyex_chat`.

pub mod catalog;
mod creds;
mod flush;
mod mention;
pub mod points;
mod send;
mod session;

pub use send::send;
pub use session::retry_refused;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use cyex_chat::build::Me;
use cyex_chat::irc::{Client, Window};
use cyex_chat::model::{ChatMessage, ConnStatus, RoomEvent};
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

type RoomKey = (Identity, String);
type View = Channel<Vec<RoomEvent>>;

pub struct Chat {
    inner: Mutex<Inner>,
    /// Wakes the flush task when a room has events waiting.
    wake: Arc<Notify>,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<Identity, Session>,
    rooms: HashMap<RoomKey, Entry>,
    /// View id → its room and the window showing it.
    views: HashMap<u32, (RoomKey, String)>,
    next_view: u32,
}

struct Session {
    client: Client,
    /// Messages sent in the last 30 s (20, or 100 where we're mod or VIP).
    sends: Window,
}

struct Entry {
    room: Room,
    views: Vec<(u32, View)>,
    outbox: Vec<RoomEvent>,
    /// C21 Last text we sent here, to dodge Twitch's duplicate check.
    last_sent: Option<(String, i64)>,
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
    });
    flush::spawn(app.clone(), wake);
    catalog::init(app);
}

impl Chat {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
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

/// Opens a view on a channel's chat, joining it if no other view has it open.
pub fn open(
    app: &AppHandle,
    window: &str,
    login: &str,
    account: Option<&str>,
    view: View,
) -> Result<ChatOpened, String> {
    let login = login.trim().trim_start_matches('#').to_ascii_lowercase();
    if !valid_login(&login) {
        return Err("That isn't a Twitch channel name.".into());
    }
    let (identity, me) = identity_for(app, account);
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let key = (identity.clone(), login.clone());
    let fresh = !inner.rooms.contains_key(&key);
    if fresh {
        let mut room = Room::new(&login, me);
        let budget_ok = match &identity {
            Identity::Anon => true,
            Identity::Account(id) => budget(app, |b| {
                b.entry(id.clone()).or_default().join(&login, false)
            }),
        };
        if budget_ok {
            session::ensure(app, &mut inner, &identity).client.join(&login);
        } else {
            room.set_status(ConnStatus::BudgetFull);
        }
        inner.rooms.insert(
            key.clone(),
            Entry {
                room,
                views: Vec::new(),
                outbox: Vec::new(),
                last_sent: None,
            },
        );
    }
    inner.next_view += 1;
    let id = inner.next_view;
    inner.views.insert(id, (key.clone(), window.to_owned()));
    let entry = inner.rooms.get_mut(&key).expect("inserted above");
    entry.views.push((id, view));
    let opened = ChatOpened {
        view: id,
        account: match &identity {
            Identity::Account(id) => Some(id.clone()),
            Identity::Anon => None,
        },
        snapshot: entry.room.snapshot(),
    };
    let budget_full = entry.room.status == ConnStatus::BudgetFull;
    drop(inner);
    if fresh {
        crate::connections::notify(app);
        catalog::load_for_room(app.clone(), key.clone());
        let history = app.state::<Settings>().read::<ChatSettings>(Partition::Chat).history;
        if history && !budget_full {
            session::load_history(app.clone(), key);
        }
    }
    Ok(opened)
}

/// Closes a view. The last view on a room leaves the channel.
pub fn close(app: &AppHandle, view: u32) {
    let chat = app.state::<Chat>();
    let mut inner = chat.lock();
    let Some((key, _)) = inner.views.remove(&view) else { return };
    let Some(entry) = inner.rooms.get_mut(&key) else { return };
    entry.views.retain(|(id, _)| *id != view);
    if !entry.views.is_empty() {
        return;
    }
    inner.rooms.remove(&key);
    let (identity, login) = key;
    if let Some(s) = inner.sessions.get(&identity) {
        s.client.part(&login);
    }
    let still_used = inner.rooms.keys().any(|(i, _)| *i == identity);
    if !still_used && let Some(s) = inner.sessions.remove(&identity) {
        s.client.stop();
    }
    drop(inner);
    if let Identity::Account(id) = &identity {
        budget(app, |b| {
            if let Some(b) = b.get_mut(id) {
                b.part(&login);
            }
        });
        crate::connections::notify(app);
    }
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
fn now_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}
