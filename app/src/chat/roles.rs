//! 1.57, V102-21 Our role in every open channel: the one source every mod-only control
//! reads, here and in the UI. Rooms keep it from USERSTATE and our own lines; this adds
//! what IRC never says (being modded or unmodded while a chat is open): Twitch's public
//! moderated-channel answer every 30 s, and mod/unmod events and revoked mod
//! subscriptions from the mod feeds. Every change goes to all windows as `mod:roles`.
//! V102-26 A tab that shows only the stream has no room to say our role, so the same
//! answer covers it: the channel's shield and its mod view show from it until a room
//! opens (the mod view's chat). Mod actions still wait for the room.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use cyex_chat::model::ModRole;
use cyex_moderation::may::Caps;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;
use ts_rs::TS;

use super::{Chat, Identity, queue};

const CHECK_EVERY: Duration = Duration::from_secs(30);
/// Rooms that join together are checked together.
const SETTLE: Duration = Duration::from_secs(2);

/// One account's role in one channel, for the UI.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoleEntry {
    pub account: String,
    pub channel: String,
    pub role: ModRole,
    /// V107-R14 What the role may do here, so menus answer without asking.
    pub may: Caps,
}

impl RoleEntry {
    fn new(account: &str, channel: &str, role: ModRole) -> RoleEntry {
        RoleEntry {
            account: account.to_owned(),
            channel: channel.to_owned(),
            role,
            may: Caps::of(role),
        }
    }
}

#[derive(Default)]
struct Roles {
    /// What the windows were told last.
    sent: Mutex<Vec<RoleEntry>>,
    /// The check's last answer per (account, channel). Only a change acts, so a stale
    /// answer never overrules a fresh USERSTATE.
    answers: Mutex<HashMap<(String, String), bool>>,
    /// V102-26 Stream-only tiles signed in to an account: (account, login) → what we know.
    watched: Mutex<Watched>,
    wake: Notify,
}

/// V102-26 A channel shown without its chat, as one account.
#[derive(Clone, Debug, PartialEq)]
struct Watch {
    channel_id: String,
    /// Twitch's last answer. The account's own channel needs none.
    moderator: bool,
}

impl Watch {
    fn role(&self, account: &str) -> Option<ModRole> {
        if account == self.channel_id {
            Some(ModRole::Broadcaster)
        } else {
            self.moderator.then_some(ModRole::Moderator)
        }
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Roles::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let woken = {
                let roles = app.state::<Roles>();
                tokio::select! {
                    () = tokio::time::sleep(CHECK_EVERY) => false,
                    () = roles.wake.notified() => true,
                }
            };
            if woken {
                tokio::time::sleep(SETTLE).await;
            }
            check(&app).await;
        }
    });
}

/// The account's role in a channel, `None` when it has no mod rights there or no open room.
/// An account's own channel is always its broadcaster's, room or not (S1: the stream
/// manager's chat closes when its mod view opens, and an anonymous room says no role).
pub fn role(app: &AppHandle, account: &str, login: &str) -> Option<ModRole> {
    let from_room = app.try_state::<Chat>().and_then(|chat| {
        let key = (Identity::Account(account.to_owned()), login.to_ascii_lowercase());
        chat.lock().rooms.get(&key)?.room.role()
    });
    from_room.or_else(|| owns(&own_channels(app), account, login).then_some(ModRole::Broadcaster))
}

/// Every signed-in account as (user id, login).
fn own_channels(app: &AppHandle) -> Vec<(String, String)> {
    let Some(settings) = app.try_state::<crate::settings::Settings>() else {
        return Vec::new();
    };
    let accounts: cyex_settings::schema::Accounts = settings.read(cyex_settings::Partition::Accounts);
    accounts
        .list
        .into_iter()
        .map(|a| (a.user_id, a.login.to_ascii_lowercase()))
        .collect()
}

/// The channel is the account's own.
fn owns(own: &[(String, String)], account: &str, login: &str) -> bool {
    own.iter()
        .any(|(id, l)| id == account && l.eq_ignore_ascii_case(login))
}

/// S1 The broadcaster role of each account in its own channel, where nothing else in
/// `list` names that pair yet.
fn own_roles(own: &[(String, String)], list: &[RoleEntry]) -> Vec<RoleEntry> {
    own.iter()
        .filter(|(id, login)| {
            !list
                .iter()
                .any(|e| e.account == *id && e.channel == *login)
        })
        .map(|(id, login)| RoleEntry::new(id, login, ModRole::Broadcaster))
        .collect()
}

/// Every open account's role, by channel.
pub fn snapshot(app: &AppHandle) -> Vec<RoleEntry> {
    let Some(chat) = app.try_state::<Chat>() else {
        return Vec::new();
    };
    let mut rooms = HashSet::new();
    let mut list: Vec<RoleEntry> = chat
        .lock()
        .rooms
        .iter()
        .filter_map(|((identity, login), e)| {
            let Identity::Account(account) = identity else {
                return None;
            };
            rooms.insert((account.clone(), login.clone()));
            Some(RoleEntry::new(account, login, e.room.role()?))
        })
        .collect();
    if let Some(roles) = app.try_state::<Roles>() {
        let watched = roles.watched.lock().unwrap_or_else(|p| p.into_inner());
        list.extend(watched_roles(&watched, &rooms));
    }
    let own = own_roles(&own_channels(app), &list);
    list.extend(own);
    list.sort();
    list
}

/// V102-26 Roles of the stream-only tiles where that account has no room (a room knows
/// better).
fn watched_roles(watched: &Watched, rooms: &HashSet<(String, String)>) -> Vec<RoleEntry> {
    watched
        .iter()
        .filter(|(key, _)| !rooms.contains(*key))
        .filter_map(|((account, login), w)| {
            Some(RoleEntry::new(account, login, w.role(account)?))
        })
        .collect()
}

/// V102-26 The tiles that show a channel's stream without its chat, signed in, as
/// (account, login, channel id). New ones are asked about soon; the windows hear of a
/// role that came or went with them.
pub fn watch(app: &AppHandle, tiles: Vec<(String, String, String)>) {
    let Some(roles) = app.try_state::<Roles>() else {
        return;
    };
    let fresh = {
        let mut watched = roles.watched.lock().unwrap_or_else(|p| p.into_inner());
        let (next, fresh) = rewatch(&watched, tiles);
        *watched = next;
        fresh
    };
    if fresh {
        roles.wake.notify_one();
    }
    emit(app);
}

type Watched = HashMap<(String, String), Watch>;

/// The new watch list, keeping what Twitch said about tiles that stay; `true` when a
/// tile needs asking.
fn rewatch(old: &Watched, tiles: Vec<(String, String, String)>) -> (Watched, bool) {
    let mut fresh = false;
    let next = tiles
        .into_iter()
        .map(|(account, login, channel_id)| {
            let key = (account, login.to_ascii_lowercase());
            let moderator = match old.get(&key) {
                Some(w) if w.channel_id == channel_id => w.moderator,
                _ => {
                    fresh |= key.0 != channel_id;
                    false
                }
            };
            (
                key,
                Watch {
                    channel_id,
                    moderator,
                },
            )
        })
        .collect();
    (next, fresh)
}

/// Tells every window the roles, when they changed since last time. Cheap to call after
/// anything that may have changed them (joins, leaves, USERSTATE).
pub fn emit(app: &AppHandle) {
    let Some(roles) = app.try_state::<Roles>() else {
        return;
    };
    let now = snapshot(app);
    {
        let mut sent = roles.sent.lock().unwrap_or_else(|p| p.into_inner());
        if *sent == now {
            return;
        }
        sent.clone_from(&now);
    }
    let _ = app.emit("mod:roles", now);
}

/// After a role changed in one room: the mod feeds (1.46) and the windows.
pub fn changed(app: &AppHandle) {
    crate::moderation::feeds::sync(app);
    emit(app);
}

/// Asks Twitch about new rooms soon instead of at the next round.
pub fn check_soon(app: &AppHandle) {
    if let Some(roles) = app.try_state::<Roles>() {
        roles.wake.notify_one();
    }
}

/// Twitch says the account moderates a channel now, or not anymore.
pub fn set_moderator(app: &AppHandle, account: &str, login: &str, on: bool) {
    let Some(chat) = app.try_state::<Chat>() else {
        return;
    };
    let key = (Identity::Account(account.to_owned()), login.to_ascii_lowercase());
    let moved = {
        let mut inner = chat.lock();
        let Some(entry) = inner.rooms.get_mut(&key) else {
            return;
        };
        let before = entry.room.role();
        let ev = entry.room.set_moderator(on);
        let after = entry.room.role();
        queue(entry, ev, &chat.wake);
        before != after
    };
    if moved {
        changed(app);
    }
}

/// Like `set_moderator`, for news that names the channel by id (EventSub).
pub fn set_moderator_in(app: &AppHandle, account: &str, channel_id: &str, on: bool) {
    let Some(chat) = app.try_state::<Chat>() else {
        return;
    };
    let login = chat.lock().rooms.iter().find_map(|((identity, login), e)| {
        (*identity == Identity::Account(account.to_owned())
            && e.room.state.channel_id.as_deref() == Some(channel_id))
        .then(|| login.clone())
    });
    if let Some(login) = login {
        set_moderator(app, account, &login, on);
    }
}

/// V110-T06 Mocks builds: the rooms of the channels `CYEX_GQL_MODS_MOCK` names are ours to moderate.
#[cfg(feature = "mocks")]
fn mock_mods(app: &AppHandle) {
    let named = super::mods_mock::channels(app);
    if named.is_empty() {
        return;
    }
    let Some(chat) = app.try_state::<Chat>() else {
        return;
    };
    let rooms: Vec<(String, String)> = chat
        .lock()
        .rooms
        .keys()
        .filter_map(|(identity, login)| match identity {
            Identity::Account(a) if named.contains(login) => Some((a.clone(), login.clone())),
            _ => None,
        })
        .collect();
    for (account, login) in rooms {
        set_moderator(app, &account, &login, true);
    }
}

/// One round of Twitch's moderated-channel answer for every room an account reads.
async fn check(app: &AppHandle) {
    #[cfg(feature = "mocks")]
    mock_mods(app);
    // Account → (login, channel id, Twitch has told us about ourselves yet).
    let mut asks: HashMap<String, Vec<(String, String, bool)>> = HashMap::new();
    {
        let Some(chat) = app.try_state::<Chat>() else {
            return;
        };
        for ((identity, login), e) in &chat.lock().rooms {
            let Identity::Account(account) = identity else {
                continue;
            };
            let Some(cid) = e.room.state.channel_id.clone() else {
                continue;
            };
            if e.room.owner() {
                continue;
            }
            asks.entry(account.clone())
                .or_default()
                .push((login.clone(), cid, e.room.state.own.is_some()));
        }
    }
    let http = app.state::<crate::auth::Auth>().http.clone();
    let mut seen = Vec::new();
    for (account, rooms) in asks {
        for chunk in rooms.chunks(cyex_twitch::gql::viewer::MODERATES_MAX) {
            let ids: Vec<String> = chunk.iter().map(|r| r.1.clone()).collect();
            let Ok(answers) = cyex_twitch::gql::viewer::moderates(&http, &account, &ids).await else {
                continue;
            };
            for ((login, _, known), on) in chunk.iter().zip(answers) {
                let key = (account.clone(), login.clone());
                seen.push(key.clone());
                let before = {
                    let roles = app.state::<Roles>();
                    let mut answers = roles.answers.lock().unwrap_or_else(|p| p.into_inner());
                    answers.insert(key, on)
                };
                if acts(before, on, *known) {
                    set_moderator(app, &account, login, on);
                }
            }
        }
    }
    #[cfg(feature = "mocks")]
    mock_mods(app);
    let roles = app.state::<Roles>();
    roles
        .answers
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .retain(|k, _| seen.contains(k));
    check_watched(app, &http).await;
}

/// V102-26 The same answer for stream-only tiles whose account has no room there.
async fn check_watched(app: &AppHandle, http: &cyex_twitch::Http) {
    let Some(chat) = app.try_state::<Chat>() else {
        return;
    };
    let rooms: HashSet<(String, String)> = chat
        .lock()
        .rooms
        .keys()
        .filter_map(|(identity, login)| match identity {
            Identity::Account(a) => Some((a.clone(), login.clone())),
            Identity::Anon => None,
        })
        .collect();
    let mut asks: HashMap<String, Vec<(String, String)>> = HashMap::new();
    {
        let roles = app.state::<Roles>();
        let watched = roles.watched.lock().unwrap_or_else(|p| p.into_inner());
        for ((account, login), w) in watched.iter() {
            if *account != w.channel_id && !rooms.contains(&(account.clone(), login.clone())) {
                asks.entry(account.clone())
                    .or_default()
                    .push((login.clone(), w.channel_id.clone()));
            }
        }
    }
    let mut changed = false;
    for (account, list) in asks {
        for chunk in list.chunks(cyex_twitch::gql::viewer::MODERATES_MAX) {
            let ids: Vec<String> = chunk.iter().map(|c| c.1.clone()).collect();
            let Ok(answers) = cyex_twitch::gql::viewer::moderates(http, &account, &ids).await else {
                continue;
            };
            let roles = app.state::<Roles>();
            let mut watched = roles.watched.lock().unwrap_or_else(|p| p.into_inner());
            for ((login, cid), on) in chunk.iter().zip(answers) {
                if let Some(w) = watched.get_mut(&(account.clone(), login.clone()))
                    && w.channel_id == *cid
                    && w.moderator != on
                {
                    w.moderator = on;
                    changed = true;
                }
            }
        }
    }
    if changed {
        emit(app);
    }
}

/// Whether an answer from the check changes the room: when it differs from the last one
/// (modded or unmodded since), or, the first time, when Twitch hasn't said anything else
/// yet and the answer is "mod".
fn acts(before: Option<bool>, now: bool, known: bool) -> bool {
    match before {
        Some(b) => b != now,
        None => now && !known,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use cyex_chat::model::ModRole;

    use super::{RoleEntry, Watch, acts, own_roles, owns, rewatch, watched_roles};

    #[test]
    fn only_changes_in_the_answer_act() {
        // First answer: only fills in a "mod" before USERSTATE came.
        assert!(acts(None, true, false));
        assert!(!acts(None, true, true));
        assert!(!acts(None, false, false));
        assert!(
            !acts(None, false, true),
            "a stale \"no\" doesn't overrule USERSTATE"
        );
        // Later: a flip is a mod or unmod while the chat is open.
        assert!(acts(Some(false), true, true));
        assert!(acts(Some(true), false, true));
        assert!(!acts(Some(true), true, true));
        assert!(!acts(Some(false), false, true));
    }

    fn tile(account: &str, login: &str, id: &str) -> (String, String, String) {
        (account.into(), login.into(), id.into())
    }

    /// V102-26 Stream-only tiles: asked once, answers kept while the tile stays.
    #[test]
    fn stream_only_tiles_keep_their_answer() {
        let (mut w, fresh) = rewatch(&HashMap::new(), vec![tile("me", "XQC", "71092938")]);
        assert!(fresh, "a new tile is asked about");
        let key = ("me".to_owned(), "xqc".to_owned());
        assert!(!w[&key].moderator, "no role until Twitch says so");
        w.get_mut(&key).unwrap().moderator = true;

        let (again, fresh) = rewatch(&w, vec![tile("me", "xqc", "71092938")]);
        assert!(!fresh);
        assert!(again[&key].moderator, "kept");
        // Another channel under that name (renamed): asked again.
        let (other, fresh) = rewatch(&w, vec![tile("me", "xqc", "1")]);
        assert!(fresh && !other[&key].moderator);
        // Your own channel needs no asking.
        let (own, fresh) = rewatch(&HashMap::new(), vec![tile("42", "me", "42")]);
        assert!(!fresh);
        let mine = &own[&("42".to_owned(), "me".to_owned())];
        assert_eq!(mine.role("42"), Some(ModRole::Broadcaster));
    }

    #[test]
    fn a_room_knows_better_than_the_watch() {
        let w = |moderator| Watch {
            channel_id: "c".into(),
            moderator,
        };
        let watched = HashMap::from([
            (("me".to_owned(), "a".to_owned()), w(true)),
            (("me".to_owned(), "b".to_owned()), w(true)),
            (("me".to_owned(), "n".to_owned()), w(false)),
        ]);
        let rooms = HashSet::from([("me".to_owned(), "b".to_owned())]);
        let roles = watched_roles(&watched, &rooms);
        assert_eq!(roles.len(), 1, "b has a room, n isn't a mod");
        assert_eq!(
            (roles[0].channel.as_str(), roles[0].role),
            ("a", ModRole::Moderator)
        );
    }

    /// S1 The own channel is the broadcaster's even without a room there (an anonymous
    /// chat, or the stream manager's chat closed while its mod view opens).
    #[test]
    fn own_channel_is_always_broadcaster() {
        let own = vec![("42".to_owned(), "me".to_owned()), ("7".to_owned(), "alt".to_owned())];
        assert!(owns(&own, "42", "Me"));
        assert!(!owns(&own, "42", "alt"), "another account's channel");
        assert!(!owns(&own, "1", "me"), "another account in my channel");

        // A room that already says so stays the one entry; the other account gets its own.
        let have = vec![RoleEntry::new("42", "me", ModRole::Broadcaster)];
        let added = own_roles(&own, &have);
        assert_eq!(added.len(), 1);
        assert_eq!(
            (added[0].account.as_str(), added[0].channel.as_str(), added[0].role),
            ("7", "alt", ModRole::Broadcaster)
        );
        assert!(added[0].may.broadcaster, "the mod view's gate reads this");
        // No rooms at all: every account still moderates its own channel.
        assert_eq!(own_roles(&own, &[]).len(), 2);
    }
}
