//! Third-party emotes, badges and paints (1.37-1.39) wired to chat. Sets load when a
//! channel's chat joins and go when its last room leaves; live changes come over the
//! providers' sockets (`live`). Chat tokenizes with [`room`]; the UI looks up who wears
//! what through `looks`.

mod card;
mod card_flags;
pub mod edit;
mod live;
mod load;
mod looks;
pub mod menu;
pub mod presence;
mod providers;
mod retry;
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
use cyex_emotes::{Capped, Cosmetics, EmoteSet, Lookup, RoomEmotes};
use cyex_settings::Partition;
use cyex_settings::schema::{Chat as ChatSettings, EmoteSource};
use cyex_twitch::ws;
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use crate::settings::Settings;
use providers::Which;

/// The Twitch emote sets an account can use in one channel.
type TwitchSets = Arc<Vec<cyex_twitch::gql::EmoteSet>>;
/// V107-R2 A personal lookup with the sets it was built from.
type SharedLookup = (Vec<Arc<EmoteSet>>, Arc<Lookup>);

/// V107-R1 Users with personal emotes kept at most (as `Cosmetics`' 7TV users).
const MAX_PERSONAL_USERS: usize = 20_000;
/// V107-R1 7TV profile pictures kept; past this the cache starts over.
const MAX_AVATARS: usize = 2_000;

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
    /// The global sets are being fetched or asked for again (`retry::globals`).
    globals_loading: bool,
    /// B4 Providers whose global sets (and badges) answered, for good or with none.
    globals_done: Which,
    /// By channel login.
    channels: HashMap<String, Channel>,
    /// C9 Personal and special 7TV sets by id, and which users have which.
    personal_sets: HashMap<String, Arc<EmoteSet>>,
    /// V107-R1 S7 Capped: last set change or read first to stay (B9).
    user_sets: Capped<String, Vec<String>, MAX_PERSONAL_USERS>,
    personal: Arc<HashMap<String, Arc<Lookup>>>,
    /// V107-R2 One lookup per distinct list of personal sets, shared by every user with that
    /// list (many share event sets), keyed by the sets' addresses. The entry holds the sets,
    /// so an address can't come back for another set while it's here.
    personal_lookups: HashMap<Vec<usize>, SharedLookup>,
    /// V107-R2 Size at which the unused `personal_lookups` go next.
    lookups_prune_at: usize,
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
    /// B6 Which join this is: the channel registry's generation (V107-R16).
    generation: u64,
    /// B6 The retry task for this join, stopped when it leaves.
    retry: Option<tauri::async_runtime::JoinHandle<()>>,
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
        let lookup = (!sets.is_empty()).then(|| self.shared_lookup(sets));
        // V107-M3 In place; copies only while a room still holds the old map.
        let map = Arc::make_mut(&mut self.personal);
        if let Some(lookup) = lookup {
            map.insert(user.to_owned(), lookup);
        } else {
            map.remove(user);
            if self.user_sets.get(user).is_some_and(Vec::is_empty) {
                self.user_sets.remove(user);
            }
        }
    }

    /// V107-R2 The lookup for these sets, built once for every user who has them. Past twice
    /// the entries in use, the ones no user holds any more go.
    fn shared_lookup(&mut self, sets: Vec<Arc<EmoteSet>>) -> Arc<Lookup> {
        let key: Vec<usize> = sets.iter().map(|s| Arc::as_ptr(s) as usize).collect();
        if let Some((_, l)) = self.personal_lookups.get(&key) {
            return l.clone();
        }
        if self.personal_lookups.len() >= self.lookups_prune_at {
            self.prune_lookups();
        }
        let l = Arc::new(Lookup::build(sets.iter().map(Arc::as_ref)));
        self.personal_lookups.insert(key, (sets, l.clone()));
        l
    }

    fn prune_lookups(&mut self) {
        self.personal_lookups.retain(|_, (_, l)| Arc::strong_count(l) > 1);
        self.lookups_prune_at = (self.personal_lookups.len() * 2).max(64);
    }

    /// V107-R1 Past the cap, drops the users whose sets changed or were read least
    /// recently, and the sets only they had. Push-only like the cosmetics (V107-M3): a
    /// dropped user's emotes come back with their next entitlement. B9 Called once after
    /// a batch, so the batch's own users (just stamped) are never the ones dropped.
    fn trim_personal(&mut self) {
        let gone = self.user_sets.trim();
        if gone.is_empty() {
            return;
        }
        let mut dropped_sets: HashSet<String> = HashSet::new();
        let map = Arc::make_mut(&mut self.personal);
        for (u, sets) in gone {
            dropped_sets.extend(sets);
            map.remove(&u);
        }
        let held: HashSet<&String> = self.user_sets.values().flatten().collect();
        dropped_sets.retain(|id| !held.contains(id));
        for id in dropped_sets {
            self.personal_sets.remove(&id);
        }
        self.prune_lookups();
    }

    /// V107-R1 A channel's chat left: its entry, and every cache keyed by a channel id no
    /// open chat has (this one's, and any a load finished into after its chat closed).
    fn forget_channel(&mut self, login: &str) -> Option<Channel> {
        let c = self.channels.remove(login)?;
        if let Some(r) = &c.retry {
            r.abort();
        }
        let open: HashSet<&str> = self.channels.values().map(|c| c.id.as_str()).collect();
        self.bot_commands.retain(|id, _| open.contains(id.as_str()));
        self.channel_twitch.retain(|id, _| open.contains(id.as_str()));
        self.twitch_sets.retain(|(_, id), _| open.contains(id.as_str()));
        Some(c)
    }

    /// B6 The channel, if it's still the join `generation`.
    fn live(&mut self, login: &str, generation: u64) -> Option<&mut Channel> {
        self.channels.get_mut(login).filter(|c| c.generation == generation)
    }

    /// V107-M3 Drops personal sets no user has any more.
    fn prune_personal_sets(&mut self, ids: &[String]) {
        for id in ids {
            if !self.user_sets.values().any(|s| s.contains(id)) {
                self.personal_sets.remove(id);
            }
        }
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

/// V107-R16 Joins and leaves of channels, from the chat's channel registry.
pub fn on_channel(app: &AppHandle, e: &crate::chat::channels::Event) {
    use crate::chat::channels::Event;
    match e {
        Event::Known { login, id, generation } => join(app, login, id, *generation),
        Event::Gone { login, generation, .. } => leave(app, login, *generation),
        _ => {}
    }
}

/// A channel's chat joined (join `generation`): loads its emotes unless they're loaded or loading.
fn join(app: &AppHandle, login: &str, channel_id: &str, generation: u64) {
    let state = app.state::<Emotes>();
    let stale = state.lock().channels.get(login).map(|c| c.generation).filter(|g| *g != generation);
    if let Some(old) = stale {
        leave(app, login, old);
    }
    let fresh = {
        let mut inner = state.lock();
        if inner.channels.contains_key(login) {
            None
        } else {
            let (ready, _) = watch::channel(false);
            inner.channels.insert(
                login.to_owned(),
                Channel {
                    id: channel_id.to_owned(),
                    generation,
                    retry: None,
                    sets: Vec::new(),
                    seventv_user: None,
                    ffz_badges: Default::default(),
                    lookup: Arc::new(Lookup::default()),
                    ready,
                },
            );
            inner.rebuild(login);
            Some(generation)
        }
    };
    if let Some(generation) = fresh {
        for t in SevenTopic::channel(channel_id) {
            state.seventv.add(t);
        }
        state.bttv.add(channel_id.to_owned());
        load::channel(app.clone(), login.to_owned(), channel_id.to_owned(), generation);
    }
}

/// A room read by `account` joined the channel: the account's own Twitch emotes there.
pub fn join_account(app: &AppHandle, account: &str, channel_id: &str) {
    load::mine(app.clone(), account.to_owned(), channel_id.to_owned(), false);
}

/// The last room of a channel left (join `generation` or older).
fn leave(app: &AppHandle, login: &str, generation: u64) {
    let Some(state) = app.try_state::<Emotes>() else {
        return;
    };
    let c = {
        let mut inner = state.lock();
        if inner.channels.get(login).is_none_or(|c| c.generation > generation) {
            return;
        }
        inner.forget_channel(login)
    };
    let Some(c) = c else {
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
    let mut inner = state.lock();
    if inner.avatars.len() >= MAX_AVATARS {
        inner.avatars.clear();
    }
    inner.avatars.insert(login, a.clone());
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
    let mut inner = state.lock();
    // V107-R1 Kept only while the chat is open; `forget_channel` drops it.
    if inner.channels.values().any(|c| c.id == id) {
        inner.bot_commands.insert(id, cmds.clone());
    }
    Ok(cmds)
}

/// Channel id of a joined channel (V107-R16 the chat's channel registry keeps it).
pub fn channel_id(app: &AppHandle, login: &str) -> Option<String> {
    app.try_state::<crate::chat::Chat>()?.channels.id(login)
}

/// V107-R2 Test builds only: the emote stores' sizes for the memory probe (`perf_probe`).
#[cfg(feature = "mocks")]
pub fn probe(app: &AppHandle) -> serde_json::Value {
    let e = app.state::<Emotes>();
    let i = e.lock();
    let set_emotes: usize = i.channels.values().flat_map(|c| c.sets.iter()).map(|s| s.emotes.len()).sum();
    serde_json::json!({
        "channels": i.channels.len(), "setEmotes": set_emotes, "globals": i.globals.len(),
        "personalSets": i.personal_sets.len(), "userSets": i.user_sets.len(), "personal": i.personal.len(),
        "personalLookups": i.personal_lookups.len(), "unclaimed": i.unclaimed.values().map(Vec::len).sum::<usize>(),
        "twitchSets": i.twitch_sets.len(), "avatars": i.avatars.len(), "botCommands": i.bot_commands.len(),
        "channelTwitch": i.channel_twitch.len(), "mine": i.mine.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyex_emotes::Scope;

    #[test]
    fn personal_state_empties_with_its_last_user() {
        let mut inner = Inner::default();
        let set = EmoteSet::new("s", Provider::Seventv, Scope::Personal, Vec::new());
        inner.personal_sets.insert("s".into(), Arc::new(set));
        inner.user_sets.insert("u".into(), vec!["s".into()]);
        inner.rebuild_personal("u");
        assert!(inner.personal.contains_key("u"));
        inner.user_sets.get_mut("u").unwrap().clear();
        inner.rebuild_personal("u");
        assert!(inner.personal.is_empty() && inner.user_sets.is_empty());
        inner.prune_personal_sets(&["s".to_owned()]);
        assert!(inner.personal_sets.is_empty());
    }

    fn channel(id: &str) -> Channel {
        Channel {
            id: id.into(),
            generation: 0,
            retry: None,
            sets: Vec::new(),
            seventv_user: None,
            ffz_badges: Default::default(),
            lookup: Arc::new(Lookup::default()),
            ready: watch::channel(false).0,
        }
    }

    #[test]
    fn v107_r1_a_thousand_joins_and_leaves_leave_nothing_behind() {
        let mut inner = Inner::default();
        inner.channels.insert("stay".into(), channel("0"));
        inner.bot_commands.insert("0".into(), Vec::new());
        for i in 1..=1000 {
            let (login, id) = (format!("c{i}"), i.to_string());
            inner.channels.insert(login.clone(), channel(&id));
            inner.bot_commands.insert(id.clone(), Vec::new());
            inner.twitch_sets.insert(("me".into(), id.clone()), (Instant::now(), Arc::default()));
            // A load that finished after its chat closed.
            inner.bot_commands.insert(format!("late{i}"), Vec::new());
            assert!(inner.forget_channel(&login).is_some());
        }
        assert_eq!(inner.channels.len(), 1);
        assert_eq!(inner.bot_commands.len(), 1);
        assert!(inner.twitch_sets.is_empty());
        assert!(inner.forget_channel("c1").is_none());
    }

    #[test]
    fn v107_r1_personal_users_stop_at_the_cap() {
        let mut inner = Inner::default();
        let n = MAX_PERSONAL_USERS + MAX_PERSONAL_USERS / 2;
        for i in 0..n {
            let (u, s) = (format!("u{i}"), format!("s{i}"));
            let set = EmoteSet::new(s.clone(), Provider::Seventv, Scope::Personal, Vec::new());
            inner.personal_sets.insert(s.clone(), Arc::new(set));
            inner.user_sets.insert(u.clone(), vec![s, "shared".into()]);
            inner.rebuild_personal(&u);
            inner.trim_personal();
            if i == 0 {
                let set = EmoteSet::new("shared", Provider::Seventv, Scope::Personal, Vec::new());
                inner.personal_sets.insert("shared".into(), Arc::new(set));
            }
        }
        let users = inner.user_sets.len();
        assert!((MAX_PERSONAL_USERS..=MAX_PERSONAL_USERS * 11 / 10).contains(&users));
        assert!(inner.personal.len() <= users);
        // Every set left belongs to a user left; the shared one stays.
        assert_eq!(inner.personal_sets.len(), users + 1);
        assert!(inner.user_sets.contains_key(&format!("u{}", n - 1)));
        assert!(!inner.user_sets.contains_key("u0"));
    }

    /// B9 A batch past the cap keeps all its own users and their set; older users go.
    #[test]
    fn b9_a_batch_past_the_cap_keeps_its_users() {
        let mut inner = Inner::default();
        for i in 0..MAX_PERSONAL_USERS {
            let (u, s) = (format!("old{i}"), format!("s{i}"));
            let set = EmoteSet::new(s.clone(), Provider::Seventv, Scope::Personal, Vec::new());
            inner.personal_sets.insert(s.clone(), Arc::new(set));
            inner.user_sets.insert(u.clone(), vec![s]);
            inner.rebuild_personal(&u);
        }
        // An old user being drawn counts as recent.
        assert!(inner.user_sets.touch("old0"));
        let batch: Vec<String> = (0..MAX_PERSONAL_USERS / 5).map(|i| format!("new{i}")).collect();
        let set = EmoteSet::new("batch", Provider::Seventv, Scope::Personal, Vec::new());
        inner.personal_sets.insert("batch".into(), Arc::new(set));
        // As `live::personal`: every user stamped first, rebuilt, then one trim.
        for u in &batch {
            inner.user_sets.get_or_default(u.clone()).push("batch".into());
        }
        for u in &batch {
            inner.rebuild_personal(u);
        }
        inner.trim_personal();
        assert_eq!(inner.user_sets.len(), MAX_PERSONAL_USERS);
        assert!(batch.iter().all(|u| inner.user_sets.contains_key(u) && inner.personal.contains_key(u)));
        assert!(inner.personal_sets.contains_key("batch"), "the batch's set isn't orphaned");
        assert!(inner.user_sets.contains_key("old0"), "drawn, so kept");
        assert!(!inner.user_sets.contains_key("old1"));
        assert!(!inner.personal_sets.contains_key("s1"), "a dropped user's own set goes too");
    }

    /// B6 A load or retry for an earlier join of the same channel finds it gone, and
    /// leaving stops the retry task.
    #[test]
    fn b6_leave_and_rejoin_starts_a_new_generation() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let mut inner = Inner::default();
            let mut first = channel("1");
            first.generation = 1;
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            first.retry = Some(tauri::async_runtime::JoinHandle::Tokio(tokio::spawn(async move {
                let _tx = tx;
                std::future::pending::<()>().await;
            })));
            inner.channels.insert("a".into(), first);
            assert!(inner.live("a", 1).is_some());
            inner.forget_channel("a");
            assert!(rx.await.is_err(), "the retry task was stopped");
            let mut again = channel("1");
            again.generation = 2;
            inner.channels.insert("a".into(), again);
            assert!(inner.live("a", 1).is_none(), "same id, older join");
            assert!(inner.live("a", 2).is_some());
        });
    }

    fn personal_set(inner: &mut Inner, id: &str, names: &[&str]) {
        let emotes = names.iter().map(|n| cyex_emotes::Emote::new(Provider::Seventv, *n, *n)).collect();
        let set = EmoteSet::new(id, Provider::Seventv, Scope::Personal, emotes);
        inner.personal_sets.insert(id.into(), Arc::new(set));
    }

    #[test]
    fn v107_r2_users_with_the_same_sets_share_one_lookup() {
        let mut inner = Inner::default();
        personal_set(&mut inner, "event", &["a", "b"]);
        personal_set(&mut inner, "own", &["c"]);
        for i in 0..1000 {
            let u = format!("u{i}");
            let sets = if i % 2 == 0 { vec!["event".into()] } else { vec!["event".into(), "own".into()] };
            inner.user_sets.insert(u.clone(), sets);
            inner.rebuild_personal(&u);
        }
        assert_eq!(inner.personal.len(), 1000);
        assert_eq!(inner.personal_lookups.len(), 2);
        assert!(Arc::ptr_eq(&inner.personal["u0"], &inner.personal["u2"]));
        assert!(inner.personal["u1"].get("c").is_some() && inner.personal["u0"].get("c").is_none());
        // A changed set gets a fresh lookup; the stale one goes once nobody holds it.
        personal_set(&mut inner, "event", &["a", "b", "z"]);
        for i in (0..1000).step_by(2) {
            inner.rebuild_personal(&format!("u{i}"));
        }
        assert!(inner.personal["u0"].get("z").is_some());
        assert!(inner.personal["u1"].get("z").is_none());
        inner.prune_lookups();
        assert_eq!(inner.personal_lookups.len(), 2);
    }

    #[test]
    fn v107_r2_lookups_stay_bounded_under_churn() {
        let mut inner = Inner::default();
        for i in 0..5000 {
            let (u, s) = (format!("u{}", i % 50), format!("s{i}"));
            personal_set(&mut inner, &s, &["x"]);
            inner.user_sets.insert(u.clone(), vec![s.clone()]);
            inner.rebuild_personal(&u);
            inner.personal_sets.remove(&s);
        }
        assert!(inner.personal_lookups.len() <= 128, "{}", inner.personal_lookups.len());
    }
}
