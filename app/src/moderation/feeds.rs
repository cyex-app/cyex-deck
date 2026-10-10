//! EventSub mod feeds (1.48, 1.56, M6). One websocket session per account that moderates
//! an open channel; on each session's welcome we subscribe the mod types for those
//! channels (Helix, cost 0). Events become chat notices and mod-view items. GQL-only
//! accounts get no feeds, since EventSub is Helix (D25).

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cyex_eventsub::events::{self, Event, Moderate};
use cyex_eventsub::session::{self, Out, Session};
use cyex_eventsub::subs;
use cyex_chat::model::NoticeKind;
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

pub fn init(app: &AppHandle) {
    app.manage(Feeds::default());
}

/// Recomputes which (account, channel) pairs need feeds, from the open chat rooms, and
/// starts or stops sessions to match. Call when rooms, roles or accounts change.
pub fn sync(app: &AppHandle) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    // Account id → (channel id → login) for channels the account mods and has open.
    let mut wanted: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (acc, login) in crate::chat::moderated_open(app) {
        if auth::helix_token(app, &acc).is_none() {
            continue; // EventSub is Helix; a GQL-only mod gets no feeds.
        }
        if let Some(cid) = channel_id(app, &login) {
            wanted.entry(acc).or_default().insert(cid, login);
        }
    }
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    map.retain(|acc, _| wanted.contains_key(acc));
    for (acc, channels) in wanted {
        let entry = map.entry(acc.clone()).or_insert_with(|| start(app, &acc));
        entry.wanted = channels;
        // If already welcomed, subscribe any new channels right away.
        if entry.session_id.is_some() {
            subscribe_new(app, &acc, entry);
        }
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
        Out::Revoked { kind, .. } => eprintln!("eventsub: {account} lost {kind}"),
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

fn route(app: &AppHandle, channel_id: &str, login: Option<&str>, ev: Event) {
    let now = crate::chat::now_ms();
    let id = || cyex_chat::build::local_id();
    match ev {
        Event::Moderate(m) => {
            let secs = timeout_secs(&m, now);
            let text = cyex_moderation::log::describe(&m, secs);
            view::add_action(
                app,
                channel_id,
                FeedItem {
                    id: id(),
                    kind: "mod_action".into(),
                    text: text.clone(),
                    ts: now,
                    user: m.target.as_ref().map(|t| t.login.clone()),
                },
            );
            if let Some(login) = login {
                crate::chat::notice_line(app, login, NoticeKind::Moderation, &text);
            }
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
        Event::HeldDone { message_id, .. } => view::resolve_held(app, channel_id, &message_id),
        Event::Follow { user, .. } => activity(app, channel_id, "follow", format!("{} followed.", display(&user)), Some(user.login), now),
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
        Event::Shield { on, .. } => {
            activity(app, channel_id, "shield", format!("Shield mode {}.", if on { "on" } else { "off" }), None, now);
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
