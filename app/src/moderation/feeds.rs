//! EventSub mod feeds (1.48, 1.56, M6). One websocket session per account that moderates
//! an open channel; on each session's welcome we subscribe the mod types for those
//! channels (Helix, cost 0). Events become chat notices and mod-view items. V103-15 A
//! GQL-only account (EventSub is Helix, D25) follows its mod actions and AutoMod decisions
//! on Hermes instead, signed in with its GQL token. The stream manager's own channel is
//! followed while the manager is open, so its "Mod actions" fill without its chat.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cyex_eventsub::events::{self, Event, Moderate};
use cyex_eventsub::session::{self, Out, Session};
use cyex_eventsub::subs;
use cyex_chat::model::NoticeKind;
use cyex_moderation::actions::{self, Decision, ModLine};
use cyex_twitch::hermes::Kind;
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::view::{self, FeedItem, HeldItem, SuspiciousItem};
use super::{channel_id, http};
use crate::auth;

struct Account {
    _session: Session,
    session_id: Option<String>,
    /// Channel ids we've subscribed on the current session.
    subscribed: HashSet<String>,
    /// Channels this account mods and that are open: channel id → login.
    wanted: HashMap<String, String>,
}

#[derive(Default)]
pub struct Feeds(Mutex<HashMap<String, Account>>);

/// V103-15 Hermes mod topics followed for GQL-only mods: (account, channel id) → login.
#[derive(Default)]
pub struct HermesFeeds(Mutex<HashMap<(String, String), String>>);

pub fn init(app: &AppHandle) {
    app.manage(Feeds::default());
    app.manage(HermesFeeds::default());
}

/// Recomputes which (account, channel) pairs need feeds, from the open chat rooms and the
/// stream manager, and starts or stops sessions and topics to match. Call when rooms,
/// roles, accounts or the stream manager change.
pub fn sync(app: &AppHandle) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    // Account id → (channel id → login) for channels the account mods and has open.
    let mut wanted: HashMap<String, HashMap<String, String>> = HashMap::new();
    // V103-15 The same for GQL-only accounts, on Hermes.
    let mut topics: HashMap<(String, String), String> = HashMap::new();
    let mut pairs: Vec<(String, String, String)> = crate::chat::moderated_open(app)
        .into_iter()
        .filter_map(|(acc, login)| Some((acc, channel_id(app, &login)?, login)))
        .collect();
    pairs.extend(crate::streammanager::own_channel(app));
    for (acc, cid, login) in pairs {
        if auth::helix_token(app, &acc).is_some() {
            wanted.entry(acc).or_default().insert(cid, login);
        } else if auth::gql_token(app, &acc).is_some() {
            topics.insert((acc, cid), login);
        }
    }
    sync_hermes(app, topics);
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    // Channels whose session closes here: the stream manager may need their follows back.
    let mut released = Vec::new();
    map.retain(|acc, entry| {
        let keep = wanted.contains_key(acc);
        if !keep {
            released.extend(entry.subscribed.iter().cloned());
        }
        keep
    });
    for (acc, channels) in wanted {
        let entry = map.entry(acc.clone()).or_insert_with(|| start(app, &acc));
        entry.wanted = channels;
        // If already welcomed, subscribe any new channels right away.
        if entry.session_id.is_some() {
            subscribe_new(app, &acc, entry);
        }
    }
    drop(map);
    for cid in released {
        crate::streammanager::follow_released(app, &cid);
    }
}

/// The stream manager let go of `channel.follow` for a channel (D30): a mod feed that has
/// the channel takes it over.
pub fn follow_released(app: &AppHandle, channel_id: &str) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let holders: Vec<(String, String)> = {
        let map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
        map.iter()
            .filter(|(_, e)| e.wanted.contains_key(channel_id) || e.subscribed.contains(channel_id))
            .filter_map(|(acc, e)| Some((acc.clone(), e.session_id.clone()?)))
            .collect()
    };
    for (acc, session_id) in holders {
        crate::streammanager::follows::take_over(app, &acc, channel_id, &session_id);
    }
}

/// A follow line in the mod view, when a mod feed covers the channel.
pub fn follow_line(app: &AppHandle, channel_id: &str, text: String, login: String) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let covered = {
        let map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
        map.values()
            .any(|e| e.wanted.contains_key(channel_id) || e.subscribed.contains(channel_id))
    };
    if covered {
        activity(app, channel_id, "follow", text, Some(login), crate::chat::now_ms());
    }
}

fn start(app: &AppHandle, account: &str) -> Account {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (session, task) = session::start(tx);
    tauri::async_runtime::spawn(task);
    let app2 = app.clone();
    let acc = account.to_owned();
    tauri::async_runtime::spawn(async move {
        while let Some(out) = rx.recv().await {
            handle(&app2, &acc, out);
        }
    });
    Account {
        _session: session,
        session_id: None,
        subscribed: HashSet::new(),
        wanted: HashMap::new(),
    }
}

fn handle(app: &AppHandle, account: &str, out: Out) {
    match out {
        Out::Welcome(id) => {
            if let Some(feeds) = app.try_state::<Feeds>() {
                let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(entry) = map.get_mut(account) {
                    entry.session_id = Some(id);
                    entry.subscribed.clear();
                }
            }
            resubscribe(app, account);
        }
        Out::Lost => {
            if let Some(feeds) = app.try_state::<Feeds>() {
                let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(entry) = map.get_mut(account) {
                    entry.session_id = None;
                    entry.subscribed.clear();
                }
            }
        }
        Out::Notification { kind, condition, event } => {
            let Some(cid) = subs::channel_of(&condition).map(str::to_owned) else { return };
            let login = login_for(app, account, &cid);
            if let Some(ev) = events::parse(&kind, &event) {
                route(app, &cid, login.as_deref(), ev);
            }
        }
        Out::Revoked { kind, status, condition } => {
            eprintln!("eventsub: {account} lost {kind} ({status})");
            // V102-21 Twitch ends a mod's subscriptions when they're unmodded.
            if status == "moderator_removed"
                && let Some(cid) = subs::channel_of(&condition)
            {
                crate::chat::roles::set_moderator_in(app, account, cid, false);
            }
        }
    }
}

/// The login for a channel id this account has open.
fn login_for(app: &AppHandle, account: &str, channel_id: &str) -> Option<String> {
    let feeds = app.try_state::<Feeds>()?;
    let map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    map.get(account)?.wanted.get(channel_id).cloned()
}

fn resubscribe(app: &AppHandle, account: &str) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let (session_id, channels) = {
        let map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
        match map.get(account) {
            Some(e) => match &e.session_id {
                Some(id) => (id.clone(), e.wanted.keys().cloned().collect::<Vec<_>>()),
                None => return,
            },
            None => return,
        }
    };
    subscribe(app, account, &session_id, channels);
}

fn subscribe_new(app: &AppHandle, account: &str, entry: &Account) {
    let Some(session_id) = entry.session_id.clone() else { return };
    let fresh: Vec<String> = entry
        .wanted
        .keys()
        .filter(|c| !entry.subscribed.contains(*c))
        .cloned()
        .collect();
    if !fresh.is_empty() {
        subscribe(app, account, &session_id, fresh);
    }
}

/// Subscribes the mod types for each channel on the account's session (Helix).
fn subscribe(app: &AppHandle, account: &str, session_id: &str, channels: Vec<String>) {
    let Some((token, client_id)) = auth::helix_token(app, account) else { return };
    let app = app.clone();
    let account = account.to_owned();
    let session_id = session_id.to_owned();
    tauri::async_runtime::spawn(async move {
        let http = http(&app);
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        for cid in channels {
            let mut ok = true;
            for (kind, version) in subs::MOD_TYPES {
                let cond = serde_json::json!({ "broadcaster_user_id": cid, "moderator_user_id": account });
                if let Err(e) = helix::eventsub::subscribe(&http, auth, &session_id, kind, version, &cond).await {
                    // 409 duplicate is fine; anything else once.
                    if !e.api_message().is_some_and(|m| m.contains("already")) {
                        ok = false;
                    }
                }
            }
            if ok && let Some(feeds) = app.try_state::<Feeds>() {
                let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(entry) = map.get_mut(&account) {
                    entry.subscribed.insert(cid);
                }
            }
        }
    });
}

/// V103-15 Starts and stops Hermes mod topics to match `want`.
fn sync_hermes(app: &AppHandle, want: HashMap<(String, String), String>) {
    let Some(state) = app.try_state::<HermesFeeds>() else { return };
    let mut have = state.0.lock().unwrap_or_else(|p| p.into_inner());
    for (acc, cid) in have.keys().filter(|k| !want.contains_key(*k)) {
        for kind in [Kind::ModActions, Kind::AutomodQueue] {
            crate::hermes::remove_user_topic(app, acc, kind, &format!("{acc}.{cid}"));
        }
    }
    for (acc, cid) in want.keys().filter(|k| !have.contains_key(*k)) {
        for kind in [Kind::ModActions, Kind::AutomodQueue] {
            crate::hermes::add_user_topic(app, acc, kind, &format!("{acc}.{cid}"));
        }
    }
    *have = want;
}

/// V103-15 A message on a GQL-only mod's Hermes topic: a mod action (PubSub's shapes) or
/// an AutoMod decision.
pub fn on_hermes(app: &AppHandle, e: &cyex_twitch::hermes::Event) {
    let Some((acc, cid)) = e.topic.channel_id.split_once('.') else { return };
    let login = app.try_state::<HermesFeeds>().and_then(|s| {
        let map = s.0.lock().unwrap_or_else(|p| p.into_inner());
        map.get(&(acc.to_owned(), cid.to_owned())).cloned()
    });
    let now = crate::chat::now_ms();
    match e.topic.kind {
        Kind::ModActions => {
            if let Some((m, secs)) = actions::pubsub(&e.payload) {
                moderate(app, cid, login.as_deref(), &m, secs, now);
            }
        }
        Kind::AutomodQueue => {
            if let Some(d) = actions::automod_queue(&e.payload) {
                view::add_action(app, cid, ModLine::decision(&d, cyex_chat::build::local_id(), now));
            }
        }
        _ => {}
    }
}

/// A mod action from either feed: roles follow, the mod view and the chat get a line.
fn moderate(app: &AppHandle, channel_id: &str, login: Option<&str>, m: &Moderate, secs: Option<u64>, now: i64) {
    // V102-21 One of our accounts modded or unmodded here: its controls follow now.
    if let Some(t) = m.target.as_ref().filter(|t| !t.id.is_empty() && matches!(m.action.as_str(), "mod" | "unmod")) {
        crate::chat::roles::set_moderator_in(app, &t.id, channel_id, m.action == "mod");
    }
    let text = cyex_moderation::log::describe(m, secs);
    view::add_action(app, channel_id, ModLine::live(m, secs, cyex_chat::build::local_id(), now));
    if let Some(login) = login {
        crate::chat::notice_line(app, login, NoticeKind::Moderation, &text);
    }
}

fn route(app: &AppHandle, channel_id: &str, login: Option<&str>, ev: Event) {
    let now = crate::chat::now_ms();
    match ev {
        Event::Moderate(m) => {
            let secs = timeout_secs(&m, now);
            moderate(app, channel_id, login, &m, secs, now);
        }
        Event::Held(h) => {
            view::add_held(
                app,
                channel_id,
                HeldItem {
                    message_id: h.message_id.clone(),
                    user_id: h.user.id.clone(),
                    login: h.user.login.clone(),
                    display: display(&h.user),
                    text: h.message.text.clone(),
                    reason: held_reason(&h),
                    ts: now,
                },
            );
            if let Some(login) = login {
                crate::chat::notice_line(
                    app,
                    login,
                    NoticeKind::Moderation,
                    &format!("AutoMod held a message from {}. Review it in the mod view.", display(&h.user)),
                );
            }
        }
        Event::HeldDone { message_id, status, moderator } => {
            // V103-15 The decision goes to the mod actions, with whose message it was.
            let held = view::held_one(app, channel_id, &message_id);
            let sender = held.as_ref().map(|h| events::Who { id: h.user_id.clone(), login: h.login.clone(), name: h.display.clone() });
            let text = held.map(|h| h.text).unwrap_or_default();
            if let Some(d) = Decision::from_status(&status, moderator.as_ref(), sender.as_ref().and_then(Decision::sender), text) {
                view::add_action(app, channel_id, ModLine::decision(&d, cyex_chat::build::local_id(), now));
            }
            view::resolve_held(app, channel_id, &message_id)
        }
        // Shared with the stream manager's feed (D30).
        Event::Follow { user, .. } => crate::streammanager::follows::seen(app, channel_id, &user),
        Event::Suspicious { user, status, message, .. } => {
            view::add_suspicious(
                app,
                channel_id,
                SuspiciousItem {
                    user_id: user.id.clone(),
                    login: user.login.clone(),
                    display: display(&user),
                    status: status.clone(),
                    text: message.text.clone(),
                    ts: now,
                },
            );
            // Restricted users' messages only reach mods through this event.
            if status == "restricted"
                && let Some(login) = login
            {
                crate::chat::notice_line(
                    app,
                    login,
                    NoticeKind::Moderation,
                    &format!("Restricted: {} said “{}”", display(&user), message.text),
                );
            }
        }
        Event::Shield { on, moderator } => {
            // V103-15 A mod action now, no longer activity.
            view::add_action(app, channel_id, ModLine::shield(on, moderator.as_ref(), cyex_chat::build::local_id(), now));
            if let Some(login) = login {
                crate::chat::notice_line(app, login, NoticeKind::Moderation, &format!("Shield mode {}.", if on { "on" } else { "off" }));
            }
        }
        Event::UnbanRequest { user, .. } => {
            activity(app, channel_id, "unban_request", format!("{} asked to be unbanned.", display(&user)), Some(user.login), now);
        }
        Event::UnbanResolved { .. } | Event::SuspiciousChanged { .. } | Event::WarningRead { .. } => {}
    }
}

fn activity(app: &AppHandle, channel_id: &str, kind: &str, text: String, user: Option<String>, ts: i64) {
    view::add_activity(
        app,
        channel_id,
        FeedItem {
            id: cyex_chat::build::local_id(),
            kind: kind.into(),
            text,
            ts,
            user,
        },
    );
}

fn display(u: &events::Who) -> String {
    if u.name.is_empty() { u.login.clone() } else { u.name.clone() }
}

fn held_reason(h: &events::Held) -> String {
    match h.reason.as_str() {
        "blocked_term" => "blocked term".into(),
        _ => h.category.clone().unwrap_or_else(|| "AutoMod".into()),
    }
}

/// A timeout's length from `expires_at`, if it's a timeout.
fn timeout_secs(m: &Moderate, now_ms: i64) -> Option<u64> {
    let exp = m.expires_at.as_deref()?;
    let end = OffsetDateTime::parse(exp, &Rfc3339).ok()?;
    let secs = end.unix_timestamp() - now_ms / 1000;
    (secs > 0).then_some(secs as u64)
}
