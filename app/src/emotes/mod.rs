//! Third-party emotes, badges and paints (1.37-1.39) wired to chat. Sets load when a
//! channel's chat joins and go when its last room leaves; live changes come over the
//! providers' sockets (`live`). Chat tokenizes with [`room`]; the UI looks up who wears
//! what through `looks`.

mod card;
pub mod edit;
mod live;
mod load;
mod looks;
pub mod menu;
pub mod presence;
mod providers;
mod ttl;
mod twitch_state;

pub use card::{EmoteCard, card};
pub use load::twitch_sets;
pub use looks::{Worn, looks_for, wear};

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_chat::model::Provider;
use cyex_emotes::index::ordered;
use cyex_emotes::seventv::socket::{EventApi, Topic as SevenTopic};
use cyex_emotes::{Cosmetics, EmoteSet, Lookup, RoomEmotes};
use cyex_settings::Partition;
use cyex_settings::schema::{Chat as ChatSettings, EmoteSource};
use cyex_twitch::ws;
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use crate::settings::Settings;

/// The Twitch emote sets an account can use in one channel.
type TwitchSets = Arc<Vec<cyex_twitch::gql::EmoteSet>>;

pub struct Emotes {
    inner: Mutex<Inner>,
    /// F18 As many 7TV connections as the chats need.
    seventv: ws::Pool<EventApi>,
    /// BTTV socket, by channel id.
    bttv: ws::Topics<String>,
    /// Global sets are in (or failed).
    globals_ready: watch::Sender<bool>,
}

#[derive(Default)]
struct Inner {
    /// C11 Provider order, Twitch first.
    priority: Vec<Provider>,
    globals: Vec<Arc<EmoteSet>>,
    globals_loading: bool,
    /// By channel login.
    channels: HashMap<String, Channel>,
    /// C9 Personal and special 7TV sets by id, and which users have which.
    personal_sets: HashMap<String, Arc<EmoteSet>>,
    user_sets: HashMap<String, Vec<String>>,
    personal: Arc<HashMap<String, Arc<Lookup>>>,
    /// Emotes pushed into sets nobody claimed yet; a personal set fills before its
    /// entitlement arrives.
    unclaimed: HashMap<String, Vec<cyex_emotes::Emote>>,
    /// Twitch sets an account can use, by (account, channel id), with when they came.
    twitch_sets: HashMap<(String, String), (Instant, TwitchSets)>,
    /// C23 7TV profile pictures by login; `None` = they have none.
    avatars: HashMap<String, Option<cyex_emotes::seventv::api::Avatar>>,
    /// C13 Bot commands by channel id.
    bot_commands: HashMap<String, Vec<cyex_emotes::bttv::api::BotCommand>>,
    /// A channel's own Twitch emotes (E5), public, by channel id.
    channel_twitch: HashMap<String, (Instant, Arc<cyex_twitch::gql::ChannelEmotes>)>,
    /// Each account's Twitch emotes by name, for its own lines.
    mine: HashMap<String, Arc<Lookup>>,
    mine_loading: HashSet<String>,
    cosmetics: Cosmetics,
}

struct Channel {
    id: String,
    sets: Vec<Arc<EmoteSet>>,
    /// The channel's 7TV user and active set, for live updates.
    seventv_user: Option<String>,
    /// FFZ's pictures for the room's mods and VIPs.
    ffz_badges: cyex_emotes::ffz::api::RoomBadges,
    lookup: Arc<Lookup>,
    ready: watch::Sender<bool>,
}

pub fn init(app: &AppHandle) {
    let (events, rx) = tokio::sync::mpsc::unbounded_channel();
    let seventv = ws::Pool::new(EventApi::new, events, spawn);
    let (events, bttv_rx) = tokio::sync::mpsc::unbounded_channel();
    let (bttv, task) = ws::start(cyex_emotes::bttv::socket::Socket, events);
    tauri::async_runtime::spawn(task);
    let priority = priority(&app.state::<Settings>().read::<ChatSettings>(Partition::Chat));
    app.manage(Emotes {
        inner: Mutex::new(Inner {
            priority,
            ..Inner::default()
        }),
        seventv,
        bttv,
        globals_ready: watch::channel(false).0,
    });
    presence::init(app);
    live::spawn(app.clone(), rx);
    providers::spawn_bttv(app.clone(), bttv_rx);
    looks::spawn(app.clone());
}

fn spawn(task: std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
    tauri::async_runtime::spawn(task);
}

impl Emotes {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn priority(c: &ChatSettings) -> Vec<Provider> {
    let mut out = vec![Provider::Twitch];
    out.extend(c.emote_priority.iter().filter_map(|s| match s {
        EmoteSource::Seventv => Some(Provider::Seventv),
        EmoteSource::Bttv => Some(Provider::Bttv),
        EmoteSource::Ffz => Some(Provider::Ffz),
        _ => None,
    }));
    out
}

impl Inner {
    fn rebuild(&mut self, login: &str) {
        let priority = self.priority.clone();
        let globals = self.globals.clone();
        if let Some(c) = self.channels.get_mut(login) {
            let all: Vec<Arc<EmoteSet>> = c.sets.iter().chain(globals.iter()).cloned().collect();
            c.lookup = Arc::new(Lookup::build(ordered(&all, &priority)));
        }
    }

    fn rebuild_all(&mut self) {
        let logins: Vec<String> = self.channels.keys().cloned().collect();
        for l in logins {
            self.rebuild(&l);
        }
    }

    /// C9 One user's personal emotes, from the sets they have.
    fn rebuild_personal(&mut self, user: &str) {
        let sets: Vec<Arc<EmoteSet>> = self
            .user_sets
            .get(user)
            .into_iter()
            .flatten()
            .filter_map(|id| self.personal_sets.get(id).cloned())
            .collect();
        let mut map = (*self.personal).clone();
        if sets.is_empty() {
            map.remove(user);
        } else {
            map.insert(
                user.to_owned(),
                Arc::new(Lookup::build(sets.iter().map(Arc::as_ref))),
            );
        }
        self.personal = Arc::new(map);
    }
}

/// What a room's lines are tokenized with. Cheap: a few `Arc` clones.
pub fn room(app: &AppHandle, account: Option<&str>, login: &str) -> RoomEmotes {
    let Some(state) = app.try_state::<Emotes>() else {
        return RoomEmotes::default();
    };
    let inner = state.lock();
    RoomEmotes {
        lookup: inner
            .channels
            .get(login)
            .map(|c| c.lookup.clone())
            .unwrap_or_default(),
        personal: inner.personal.clone(),
        mine: account.and_then(|a| inner.mine.get(a).map(|l| (a.to_owned(), l.clone()))),
    }
}

/// 1.26 What whispers are tokenized with: the global sets and personal emotes, no
/// channel. Starts loading the global sets if no chat has yet.
pub fn global(app: &AppHandle) -> RoomEmotes {
    let Some(state) = app.try_state::<Emotes>() else {
        return RoomEmotes::default();
    };
    load::globals(app);
    let inner = state.lock();
    RoomEmotes {
        lookup: Arc::new(Lookup::build(ordered(&inner.globals, &inner.priority))),
        personal: inner.personal.clone(),
        mine: None,
    }
}

/// A channel's chat joined: loads its emotes unless they're loaded or loading.
pub fn join(app: &AppHandle, login: &str, channel_id: &str, account: Option<&str>) {
    let state = app.state::<Emotes>();
    let fresh = {
        let mut inner = state.lock();
        if inner.channels.contains_key(login) {
            false
        } else {
            let (ready, _) = watch::channel(false);
            inner.channels.insert(
                login.to_owned(),
                Channel {
                    id: channel_id.to_owned(),
                    sets: Vec::new(),
                    seventv_user: None,
                    ffz_badges: Default::default(),
                    lookup: Arc::new(Lookup::default()),
                    ready,
                },
            );
            inner.rebuild(login);
            true
        }
    };
    if fresh {
        for t in SevenTopic::channel(channel_id) {
            state.seventv.add(t);
        }
        state.bttv.add(channel_id.to_owned());
        load::channel(app.clone(), login.to_owned(), channel_id.to_owned());
    }
    if let Some(a) = account {
        load::mine(app.clone(), a.to_owned(), channel_id.to_owned(), false);
    }
}

/// The last room of a channel left.
pub fn leave(app: &AppHandle, login: &str) {
    let Some(state) = app.try_state::<Emotes>() else {
        return;
    };
    let Some(c) = state.lock().channels.remove(login) else {
        return;
    };
    for t in SevenTopic::channel(&c.id) {
        state.seventv.remove(&t);
    }
    state.bttv.remove(c.id.clone());
    for set in c.sets.iter().filter(|s| s.provider == Provider::Seventv) {
        state.seventv.remove(&SevenTopic::Set(set.id.clone()));
    }
    if let Some(u) = c.seventv_user {
        state.seventv.remove(&SevenTopic::User(u));
    }
}

/// Waits until a channel's emotes and the global ones are in, at most `limit`. History
/// waits for this so its lines get them too.
pub async fn ready(app: &AppHandle, login: &str, limit: Duration) {
    let (mut channel, mut globals) = {
        let Some(state) = app.try_state::<Emotes>() else {
            return;
        };
        let inner = state.lock();
        match inner.channels.get(login) {
            Some(c) => (c.ready.subscribe(), state.globals_ready.subscribe()),
            None => return,
        }
    };
    let _ = tokio::time::timeout(limit, async {
        let _ = channel.wait_for(|r| *r).await;
        let _ = globals.wait_for(|r| *r).await;
    })
    .await;
}

/// Chat settings changed: the provider order may have too (C11).
pub fn on_settings(app: &AppHandle) {
    let Some(state) = app.try_state::<Emotes>() else {
        return;
    };
    let p = priority(&app.state::<Settings>().read::<ChatSettings>(Partition::Chat));
    let mut inner = state.lock();
    if inner.priority != p {
        inner.priority = p;
        inner.rebuild_all();
    }
}

/// C12 Counts the emotes in a message the account sent.
pub fn record_usage(app: &AppHandle, body: &[cyex_chat::model::Fragment]) {
    use cyex_chat::model::Fragment;
    use cyex_settings::schema::{EmoteKey, Emotes as EmoteSettings};
    let mut used: Vec<EmoteKey> = Vec::new();
    for f in body {
        let Fragment::Emote {
            provider, id, name, ..
        } = f
        else {
            continue;
        };
        let source = match provider {
            Provider::Twitch => EmoteSource::Twitch,
            Provider::Seventv => EmoteSource::Seventv,
            Provider::Bttv => EmoteSource::Bttv,
            Provider::Ffz => EmoteSource::Ffz,
        };
        if !used.iter().any(|k| k.source == source && k.id == *id) {
            used.push(EmoteKey {
                source,
                id: id.clone(),
                name: name.clone(),
            });
        }
    }
    if used.is_empty() {
        return;
    }
    let now = crate::auth::unix_now();
    let _ = app
        .state::<Settings>()
        .update::<EmoteSettings>(app, Partition::Emotes, |e| {
            e.record(&used, now);
            true
        });
}

/// 1.39 FFZ's mod and VIP pictures for a channel.
pub fn room_badges(app: &AppHandle, login: &str) -> cyex_emotes::ffz::api::RoomBadges {
    app.state::<Emotes>()
        .lock()
        .channels
        .get(login)
        .map(|c| c.ffz_badges.clone())
        .unwrap_or_default()
}

/// C23 A user's animated 7TV profile picture, if they have one. Cached for the run.
pub async fn avatar(app: &AppHandle, login: &str) -> Option<cyex_emotes::seventv::api::Avatar> {
    let login = login.to_ascii_lowercase();
    let state = app.state::<Emotes>();
    if let Some(a) = state.lock().avatars.get(&login) {
        return a.clone();
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let found = cyex_emotes::seventv::api::avatars(&http, std::slice::from_ref(&login))
        .await
        .ok()?;
    let a = found.get(&login).cloned();
    state.lock().avatars.insert(login, a.clone());
    a
}

/// The account just got new Twitch emotes.
pub fn refresh_mine(app: &AppHandle, account: &str, channel_id: &str) {
    load::mine(app.clone(), account.to_owned(), channel_id.to_owned(), true);
}

/// C13 A channel's chat bot commands, cached for the run.
pub async fn bot_commands(
    app: &AppHandle,
    login: &str,
) -> Result<Vec<cyex_emotes::bttv::api::BotCommand>, String> {
    let id = channel_id(app, login).ok_or("This chat isn’t open.")?;
    let state = app.state::<Emotes>();
    if let Some(c) = state.lock().bot_commands.get(&id) {
        return Ok(c.clone());
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let cmds = cyex_emotes::bttv::api::bot_commands(&http, &id)
        .await
        .map_err(|e| e.to_string())?;
    state.lock().bot_commands.insert(id, cmds.clone());
    Ok(cmds)
}

/// Channel id of a joined channel.
pub fn channel_id(app: &AppHandle, login: &str) -> Option<String> {
    app.try_state::<Emotes>()?
        .lock()
        .channels
        .get(login)
        .map(|c| c.id.clone())
}
