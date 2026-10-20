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
use cyex_eventsub::subs::{self, Answer, ChannelSubs};
use cyex_chat::model::NoticeKind;
use cyex_moderation::actions::{self, Decision, ModLine};
use cyex_moderation::own::{Echo, OwnEchoes};
use cyex_moderation::role_change::RoleAction;
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
    /// V107-A2 Per channel id, what is made on the current session. Channels stay after
    /// they close: their subscriptions live until the socket ends.
    subs: HashMap<String, ChannelSubs>,
    /// Channels with a subscribe task running, and how many subscriptions those still send.
    pending: HashSet<String>,
    in_flight: usize,
    /// Channels this account mods and that are open: channel id → login.
    wanted: HashMap<String, String>,
    /// V107-A2 When each wanted channel was opened (a counter), and which are shown: over
    /// the cap, shown ones win, then the first opened.
    opened: HashMap<String, u64>,
    next_open: u64,
    front: HashSet<String>,
    /// After a 429: no new subscribes before this.
    full_until_ms: i64,
    /// The cap was logged (once per account).
    warned_full: bool,
    /// A retry of failed subscriptions is scheduled.
    retry_armed: bool,
}

impl Account {
    /// Whether this session holds subscriptions for the channel.
    fn covers(&self, channel_id: &str) -> bool {
        self.subs.get(channel_id).is_some_and(|s| s.held() > 0)
    }

    /// A new session (or none): nothing is made on it yet.
    fn reset(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
        self.subs.clear();
        self.pending.clear();
        self.in_flight = 0;
        self.full_until_ms = 0;
        self.retry_armed = false;
    }

    /// Wanted channels in priority order: shown first, then the first opened.
    fn by_priority(&self) -> Vec<String> {
        let mut ids: Vec<&String> = self.wanted.keys().collect();
        ids.sort_by_key(|c| (!self.front.contains(*c), self.opened.get(*c).copied().unwrap_or(u64::MAX), (*c).clone()));
        ids.into_iter().cloned().collect()
    }
}

#[derive(Default)]
pub struct Feeds(Mutex<HashMap<String, Account>>);

/// V103-15 Hermes mod topics followed for GQL-only mods: (account, channel id) → login.
#[derive(Default)]
pub struct HermesFeeds(Mutex<HashMap<(String, String), String>>);

pub fn init(app: &AppHandle) {
    app.manage(Feeds::default());
    app.manage(HermesFeeds::default());
    app.manage(Own::default());
}

/// V107-P7 Our own mod actions, waiting for their echo (`cyex_moderation::own`).
#[derive(Default)]
pub struct Own(Mutex<OwnEchoes>);

/// S1 Marks `e` as ours (`mark: false` once it failed): its echo adds no line.
pub fn own(app: &AppHandle, e: Echo, mark: bool) {
    let Some(own) = app.try_state::<Own>() else { return };
    let mut own = own.0.lock().unwrap_or_else(|p| p.into_inner());
    if mark {
        own.mark(e, crate::chat::now_ms());
    } else {
        own.forget(&e);
    }
}

/// V107-P7 We're deleting `msg_id` (`gone: false` once it failed).
pub fn own_delete(app: &AppHandle, msg_id: &str, gone: bool) {
    own(app, Echo::Delete(msg_id.to_owned()), gone);
}

/// V107-M7 We lift `login`'s timeout only to put the new one on (`done: false` once that
/// failed): its echo adds no line, the new timeout's does.
pub fn own_lift(app: &AppHandle, channel_id: &str, login: &str, done: bool) {
    own(app, Echo::lift(channel_id, login), done);
}

/// V107-M8 A swap's steps (`mark: false` once it failed): their echoes add no line, the
/// swap's own line says both.
pub fn own_role(app: &AppHandle, channel_id: &str, action: RoleAction, login: &str, mark: bool) {
    own(app, Echo::role(channel_id, login, action), mark);
}

/// V107-P7 Whether this echo is one of ours: a delete, our lift before a new timeout, a swap step.
fn ours(app: &AppHandle, channel_id: &str, m: &Moderate, now: i64) -> bool {
    let Some(e) = Echo::of(m, channel_id) else { return false };
    let Some(own) = app.try_state::<Own>() else { return false };
    own.0.lock().unwrap_or_else(|p| p.into_inner()).take(&e, now)
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
    let open: HashSet<(String, String)> = pairs.iter().map(|(a, c, _)| (a.clone(), c.clone())).collect();
    // V107-A2 Shown channels win the session’s slots.
    let front: HashSet<String> = pairs
        .iter()
        .filter(|(_, _, login)| crate::chat::shown::lane(app, login) == cyex_twitch::pace::Lane::Front)
        .map(|(_, cid, _)| cid.clone())
        .collect();
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
            released.extend(entry.subs.iter().filter(|(_, s)| s.held() > 0).map(|(c, _)| c.clone()));
        }
        keep
    });
    for (acc, channels) in wanted {
        let entry = map.entry(acc.clone()).or_insert_with(|| start(app, &acc));
        entry.opened.retain(|c, _| channels.contains_key(c));
        for c in channels.keys() {
            if !entry.opened.contains_key(c) {
                entry.opened.insert(c.clone(), entry.next_open);
                entry.next_open += 1;
            }
        }
        entry.front = channels.keys().filter(|c| front.contains(*c)).cloned().collect();
        entry.wanted = channels;
        // If already welcomed, subscribe what is new or due again right away.
        queue(app, &acc, entry);
    }
    drop(map);
    for cid in released {
        crate::streammanager::follow_released(app, &cid);
    }
    crate::feeds::automod::keep_only(app, &open);
    view::keep_only(app, open.iter().map(|(_, cid)| cid.as_str()));
}

/// The stream manager let go of `channel.follow` for a channel (D30): a mod feed that has
/// the channel takes it over.
pub fn follow_released(app: &AppHandle, channel_id: &str) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let holders: Vec<(String, String)> = {
        let map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
        map.iter()
            .filter(|(_, e)| e.wanted.contains_key(channel_id) || e.covers(channel_id))
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
            .any(|e| e.wanted.contains_key(channel_id) || e.covers(channel_id))
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
        subs: HashMap::new(),
        pending: HashSet::new(),
        in_flight: 0,
        wanted: HashMap::new(),
        opened: HashMap::new(),
        next_open: 0,
        front: HashSet::new(),
        full_until_ms: 0,
        warned_full: false,
        retry_armed: false,
    }
}

fn handle(app: &AppHandle, account: &str, out: Out) {
    match out {
        Out::Welcome(id) => {
            if let Some(feeds) = app.try_state::<Feeds>() {
                let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(entry) = map.get_mut(account) {
                    entry.reset(Some(id));
                    queue(app, account, entry);
                }
            }
        }
        Out::Lost => {
            if let Some(feeds) = app.try_state::<Feeds>() {
                let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(entry) = map.get_mut(account) {
                    entry.reset(None);
                }
            }
        }
        Out::Notification { kind, condition, event } => {
            let Some(cid) = subs::channel_of(&condition).map(str::to_owned) else { return };
            let login = login_for(app, account, &cid);
            if let Some(ev) = events::parse(&kind, &event) {
                route(app, account, &cid, login.as_deref(), ev);
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

/// V107-A2 Starts subscribing what the session still needs: new channels, and failed types
/// whose wait is over. Within Twitch's 300 per socket (`subs::next_channels`); one task per
/// channel at a time. Called with the feeds lock held.
fn queue(app: &AppHandle, account: &str, entry: &mut Account) {
    let Some(session_id) = entry.session_id.clone() else { return };
    let Some(token) = auth::helix_token(app, account) else { return };
    let now = crate::chat::now_ms();
    if now < entry.full_until_ms {
        return;
    }
    let order = entry.by_priority();
    let order: Vec<&str> = order.iter().map(String::as_str).collect();
    let (next, left_out) = subs::next_channels(&order, &entry.subs, &entry.pending, entry.in_flight, now);
    if left_out > 0 && !entry.warned_full {
        entry.warned_full = true;
        eprintln!("mod feeds: {account}: Twitch's {} per connection reached, {left_out} channels without a feed", subs::MAX_PER_SESSION);
    }
    if next.is_empty() {
        return;
    }
    let jobs: Vec<(String, Vec<usize>)> = next.into_iter().map(|(c, t)| (c.to_owned(), t)).collect();
    for (cid, todo) in &jobs {
        entry.pending.insert(cid.clone());
        entry.in_flight += todo.len();
    }
    subscribe(app, account, token, session_id, jobs);
}

/// What a subscribe task does with the account's state, if its session is still the one.
fn with_entry<R>(app: &AppHandle, account: &str, session_id: &str, f: impl FnOnce(&mut Account) -> R) -> Option<R> {
    let feeds = app.try_state::<Feeds>()?;
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    map.get_mut(account).filter(|e| e.session_id.as_deref() == Some(session_id)).map(f)
}

/// Subscribes the given mod types per channel on the account's session (Helix), one answer
/// at a time into the account's state.
fn subscribe(app: &AppHandle, account: &str, (token, client_id): (String, String), session_id: String, jobs: Vec<(String, Vec<usize>)>) {
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move {
        let http = http(&app);
        let auth = helix::Auth {
            client_id: &client_id,
            token: &token,
        };
        let mut full = false;
        for (cid, todo) in &jobs {
            let mut sent = 0;
            for &i in todo {
                if full {
                    break;
                }
                let (kind, version) = subs::MOD_TYPES[i];
                let cond = serde_json::json!({ "broadcaster_user_id": cid, "moderator_user_id": account });
                let answer = match helix::eventsub::subscribe(&http, auth, &session_id, kind, version, &cond).await {
                    Ok(_) => Answer::Done,
                    Err(e) => {
                        let a = subs::classify(e.status(), e.api_message());
                        // Status and Twitch's message only (the error never carries the token).
                        if a != Answer::Done && first_failure(cid, kind) {
                            eprintln!("mod feeds: {cid}: couldn't subscribe {kind} ({a:?}): {e}");
                        }
                        a
                    }
                };
                full = answer == Answer::Full;
                let now = crate::chat::now_ms();
                sent += 1;
                let live = with_entry(&app, &account, &session_id, |e| {
                    e.in_flight = e.in_flight.saturating_sub(1);
                    e.subs.entry(cid.clone()).or_default().answer(i, answer, now);
                    if full {
                        e.full_until_ms = now + subs::RETRY_MS;
                    }
                });
                // The socket was replaced: the new session subscribes for itself.
                if live.is_none() {
                    return;
                }
            }
            with_entry(&app, &account, &session_id, |e| {
                e.pending.remove(cid);
                e.in_flight = e.in_flight.saturating_sub(todo.len() - sent);
            });
        }
        retry_later(&app, &account, &session_id);
    });
}

/// V107-A2 Failed subscriptions go again once their wait is over, without waiting for a sync.
fn retry_later(app: &AppHandle, account: &str, session_id: &str) {
    let Some(at) = with_entry(app, account, session_id, |e| {
        let at = e.subs.values().filter_map(ChannelSubs::retry_at).min().map(|t| t.max(e.full_until_ms));
        let at = at.filter(|_| !e.retry_armed)?;
        e.retry_armed = true;
        Some(at)
    })
    .flatten() else {
        return;
    };
    let (app, account, session_id) = (app.clone(), account.to_owned(), session_id.to_owned());
    tauri::async_runtime::spawn(async move {
        let wait = (at - crate::chat::now_ms()).max(1_000) as u64;
        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        let app2 = app.clone();
        let acc = account.clone();
        with_entry(&app, &account, &session_id, move |e| {
            e.retry_armed = false;
            queue(&app2, &acc, e);
        });
    });
}

/// Whether this is the first failed subscribe of `kind` in channel `cid` (logged once each).
fn first_failure(cid: &str, kind: &str) -> bool {
    static SEEN: Mutex<Option<HashSet<(String, String)>>> = Mutex::new(None);
    let mut seen = SEEN.lock().unwrap_or_else(|p| p.into_inner());
    seen.get_or_insert_with(HashSet::new).insert((cid.to_owned(), kind.to_owned()))
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
            // V106-07 Q2b Held (pending) messages count too, so GQL-only mods see them.
            if let Some(p) = actions::automod_pending(&e.payload) {
                let item = HeldItem {
                    message_id: p.message_id,
                    user_id: p.sender.id.clone(),
                    login: p.sender.login.clone(),
                    display: display(&p.sender),
                    text: p.text,
                    reason: p.reason,
                    ts: now,
                };
                held(app, acc, cid, login.as_deref(), item);
            } else if let Some(r) = actions::automod_resolved(&e.payload) {
                if let Some(d) = actions::automod_queue(&e.payload) {
                    view::add_action(app, cid, ModLine::decision(&d, cyex_chat::build::local_id(), now));
                }
                crate::feeds::automod::done(app, &r.message_id, &r.status, r.by.as_deref());
                view::resolve_held(app, cid, &r.message_id);
            }
        }
        _ => {}
    }
}

/// A mod action from either feed: roles follow, the mod view and the chat get a line.
fn moderate(app: &AppHandle, channel_id: &str, login: Option<&str>, m: &Moderate, secs: Option<u64>, now: i64) {
    let role = RoleAction::parse(&m.action);
    // V102-21 One of our accounts modded or unmodded here: its controls follow now.
    if let (Some(t), Some(r)) = (m.target.as_ref().filter(|t| !t.id.is_empty()), role.filter(|r| r.moderator())) {
        crate::chat::roles::set_moderator_in(app, &t.id, channel_id, r.on());
    }
    // V107-M8 Role buttons follow any mod's change.
    if let (Some(t), Some(l), Some(r)) = (m.target.as_ref(), login, role) {
        super::standing::changed(app, l, &t.login, r.moderator(), r.on());
    }
    // V107-P7 A delete quotes the text as chat had it, before Twitch's own copy.
    let shown = m.message_id.as_deref().filter(|_| m.action.ends_with("delete"));
    let body = shown.and_then(|id| crate::chat::live::deleted_of(app, id).1);
    let m = &Moderate { message_body: body.or_else(|| m.message_body.clone()), ..m.clone() };
    let text = cyex_moderation::log::describe(m, secs);
    view::add_action(app, channel_id, ModLine::live(m, secs, cyex_chat::build::local_id(), now));
    // V107-P7 Our own delete already left its line.
    let Some(login) = login.filter(|_| !ours(app, channel_id, m, now)) else { return };
    // V107-M7 Timeouts of one user in a row share a line with IRC's copy.
    let timed_out = m.target.as_ref().filter(|t| m.action == "timeout" && !t.id.is_empty());
    match timed_out {
        Some(t) => crate::chat::timeout_line(app, login, &t.id, &text),
        None => crate::chat::notice_line(app, login, NoticeKind::Moderation, &text),
    }
}

/// A held message, from EventSub or Hermes (V103-15): mod view, chat notice and the AutoMod feed (V106-07).
fn held(app: &AppHandle, account: &str, channel_id: &str, login: Option<&str>, item: HeldItem) {
    let who = item.display.clone();
    if let Some(login) = login {
        crate::feeds::automod::held(app, account, channel_id, login, &item);
    }
    view::add_held(app, channel_id, item);
    if let Some(login) = login {
        crate::chat::notice_line(
            app,
            login,
            NoticeKind::Moderation,
            &format!("AutoMod held a message from {who}. Review it in the mod view."),
        );
    }
}

fn route(app: &AppHandle, account: &str, channel_id: &str, login: Option<&str>, ev: Event) {
    let now = crate::chat::now_ms();
    match ev {
        Event::Moderate(m) => {
            let secs = timeout_secs(&m, now);
            moderate(app, channel_id, login, &m, secs, now);
        }
        Event::Held(h) => {
            let item = HeldItem {
                message_id: h.message_id.clone(),
                user_id: h.user.id.clone(),
                login: h.user.login.clone(),
                display: display(&h.user),
                text: h.message.text.clone(),
                reason: held_reason(&h),
                ts: now,
            };
            held(app, account, channel_id, login, item);
        }
        Event::HeldDone { message_id, status, moderator } => {
            // V103-15 The decision goes to the mod actions, with whose message it was.
            let held = view::held_one(app, channel_id, &message_id);
            let sender = held.as_ref().map(|h| events::Who { id: h.user_id.clone(), login: h.login.clone(), name: h.display.clone() });
            let text = held.map(|h| h.text).unwrap_or_default();
            if let Some(d) = Decision::from_status(&status, moderator.as_ref(), sender.as_ref().and_then(Decision::sender), text) {
                view::add_action(app, channel_id, ModLine::decision(&d, cyex_chat::build::local_id(), now));
            }
            // V106-07
            let by = moderator.as_ref().map(|m| if m.name.is_empty() { m.login.as_str() } else { m.name.as_str() });
            crate::feeds::automod::done(app, &message_id, &status, by);
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
            crate::chat::powerups::shield_changed(app, channel_id, on);
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
