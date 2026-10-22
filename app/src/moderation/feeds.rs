//! EventSub mod feeds (1.48, 1.56, M6). One websocket session per account that moderates
//! an open channel (two past ~150 channels, V107-R17 `sockets`); on each session's welcome
//! we subscribe the mod types for those channels (Helix, cost 0). Events become chat notices and mod-view items. V103-15 A
//! GQL-only account (EventSub is Helix, D25) follows its mod actions and AutoMod decisions
//! on Hermes instead, signed in with its GQL token. The stream manager's own channel is
//! followed while the manager is open, so its "Mod actions" fill without its chat.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cyex_eventsub::events::{self, Event, Moderate};
use cyex_eventsub::session::Out;
use cyex_eventsub::subs;
use cyex_chat::model::{Gone, GoneHow, NoticeKind};
use cyex_moderation::actions::{self, Decision, ModLine};
use cyex_moderation::own::{Echo, OwnEchoes};
use cyex_moderation::roles::RoleAction;
use cyex_twitch::hermes::Kind;
use tauri::{AppHandle, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::view::{self, FeedItem, HeldItem, SuspiciousItem};
use super::channel_id;
use crate::auth;

mod sockets;

use sockets::Account;

#[derive(Default)]
pub struct Feeds(Mutex<HashMap<String, Account>>);

/// V103-15 Hermes mod topics followed for GQL-only mods: (account, channel id) → login.
#[derive(Default)]
pub struct HermesFeeds(Mutex<HashMap<(String, String), String>>);

pub fn init(app: &AppHandle) {
    app.manage(Feeds::default());
    app.manage(HermesFeeds::default());
    app.manage(Own::default());
    sockets::init(app);
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

/// V107-R44 We time `login` out for `secs` (`None` once that failed): the echo's line
/// says the length we sent.
pub fn own_timeout(app: &AppHandle, channel_id: &str, login: &str, secs: Option<u32>) {
    let Some(own) = app.try_state::<Own>() else { return };
    own.0.lock().unwrap_or_else(|p| p.into_inner()).timeout(channel_id, login, secs, crate::chat::now_ms());
}

/// V107-R44 Our delete of `msg_id` went through: when no echo has come within
/// [`DELETE_ECHO_MS`], `line` says it instead (an account without a mod feed here).
pub fn own_delete_done(app: &AppHandle, channel: &str, msg_id: &str, line: String) {
    let (app, channel, e) = (app.clone(), channel.to_owned(), Echo::Delete(msg_id.to_owned()));
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(DELETE_ECHO_MS)).await;
        let Some(own) = app.try_state::<Own>() else { return };
        let waiting = own.0.lock().unwrap_or_else(|p| p.into_inner()).take(&e, crate::chat::now_ms());
        if waiting {
            crate::chat::notice_line(&app, &channel, NoticeKind::Moderation, &line);
        }
    });
}

/// V107-R44 How long our delete waits for its echo before saying it locally.
const DELETE_ECHO_MS: u64 = 4_000;

/// V107-M8 A swap's steps (`mark: false` once it failed): their echoes add no line, the
/// swap's own line says both.
pub fn own_role(app: &AppHandle, channel_id: &str, action: RoleAction, login: &str, mark: bool) {
    own(app, Echo::role(channel_id, login, action), mark);
}

/// V107-P7 Whether this echo is one of ours: a delete, our lift before a new timeout, a swap step.
/// V107-R44 A delete's echo answers our mark and still adds its line.
fn ours(app: &AppHandle, channel_id: &str, m: &Moderate, now: i64) -> bool {
    let Some(e) = Echo::of(m, channel_id) else { return false };
    let Some(own) = app.try_state::<Own>() else { return false };
    own.0.lock().unwrap_or_else(|p| p.into_inner()).take(&e, now)
}

/// V107-R44 Whether this role echo repeats the change just shown.
fn repeat(app: &AppHandle, channel_id: &str, m: &Moderate, now: i64) -> bool {
    let Some(e) = Echo::of(m, channel_id) else { return false };
    let Some(own) = app.try_state::<Own>() else { return false };
    own.0.lock().unwrap_or_else(|p| p.into_inner()).repeat(&e, now)
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
            released.extend(entry.held_channels());
        }
        keep
    });
    for (acc, channels) in wanted {
        let entry = map.entry(acc.clone()).or_insert_with(|| Account::new(app, &acc));
        entry.set_wanted(channels, &front);
        // If already welcomed, subscribe what is new or due again right away.
        released.extend(sockets::queue(app, &acc, entry));
    }
    drop(map);
    for cid in released {
        crate::streammanager::follow_released(app, &cid);
    }
    crate::feeds::automod::keep_only(app, &open);
    view::keep_only(app, open.iter().map(|(_, cid)| cid.as_str()));
}

/// The stream manager let go of `channel.follow` for a channel (D30): a mod feed that has
/// the channel takes it over, on the channel's socket and counted (V107-R17).
pub fn follow_released(app: &AppHandle, channel_id: &str) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let holders: Vec<(String, String)> = {
        let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
        map.iter_mut()
            .filter(|(_, e)| e.wanted.contains_key(channel_id) || e.covers(channel_id))
            .filter_map(|(acc, e)| Some((acc.clone(), e.take_follow(channel_id)?)))
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

fn handle(app: &AppHandle, account: &str, out: Out) {
    match out {
        // V107-R17 Sessions are `sockets`' business.
        Out::Welcome(_) | Out::Lost => {}
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
                moderate(app, acc, cid, login.as_deref(), &m, secs, now);
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
fn moderate(
    app: &AppHandle,
    account: &str,
    channel_id: &str,
    login: Option<&str>,
    m: &Moderate,
    secs: Option<u64>,
    now: i64,
) {
    let role = RoleAction::parse(&m.action);
    // V102-21 One of our accounts modded or unmodded here: its controls follow now.
    if let (Some(t), Some(r)) = (m.target.as_ref().filter(|t| !t.id.is_empty()), role.filter(|r| r.moderator())) {
        crate::chat::roles::set_moderator_in(app, &t.id, channel_id, r.on());
    }
    // V107-M8 Role buttons follow any mod's change.
    if let (Some(t), Some(l), Some(r)) = (m.target.as_ref(), login, role) {
        super::standing::changed(app, l, &t.login, r.moderator(), r.on());
    }
    // V107-R44 Twitch can say one role change twice (a mod timed out loses mod): one line.
    if role.is_some() && repeat(app, channel_id, m, now) {
        return;
    }
    // V107-P7 A delete quotes the text as chat had it, before Twitch's own copy.
    let shown = m.message_id.as_deref().filter(|_| m.action.ends_with("delete"));
    let body = shown.and_then(|id| crate::chat::live::deleted_of(app, id).1);
    let m = &Moderate { message_body: body.or_else(|| m.message_body.clone()), ..m.clone() };
    // V107-R44 Our own timeout says the length we sent.
    let secs = match (m.action.as_str(), m.target.as_ref()) {
        ("timeout", Some(t)) => sent_secs(app, channel_id, &t.login, now).map(u64::from).or(secs),
        _ => secs,
    };
    let text = cyex_moderation::log::describe(m, secs);
    view::add_action(app, channel_id, ModLine::live(m, secs, cyex_chat::build::local_id(), now));
    // V107-R34 The deleted lines name the mod.
    if let Some(l) = login {
        attribute(app, account, l, m, secs);
    }
    // V107-P7 Our own lift or swap step already has its line. V107-R44 A delete's echo is the
    // one line for our delete too: taking the mark stops the local fallback.
    let mine = ours(app, channel_id, m, now) && m.action != "delete";
    let Some(login) = login.filter(|_| !mine) else { return };
    if let Some(t) = m.target.as_ref().filter(|t| matches!(m.action.as_str(), "untimeout" | "unban") && !t.id.is_empty()) {
        crate::chat::lifted(app, login, &t.id);
    }
    // V107-M7 Timeouts of one user in a row share a line with IRC's copy; V107-N2e a ban too.
    let stacked = m.target.as_ref().filter(|t| matches!(m.action.as_str(), "timeout" | "ban") && !t.id.is_empty());
    match stacked {
        Some(t) => crate::chat::timeout_line(app, login, &t.id, m.action == "ban", &text),
        None => crate::chat::notice_line(app, login, NoticeKind::Moderation, &text),
    }
}

/// V107-R34 A delete, timeout or ban in this channel (not a shared chat's other one): the
/// lines it took say which mod.
fn attribute(app: &AppHandle, account: &str, login: &str, m: &Moderate, secs: Option<u64>) {
    use crate::chat::GoneTarget;
    if m.source_channel.is_some() {
        return;
    }
    let mo = &m.moderator;
    let by = if mo.name.is_empty() { mo.login.clone() } else { mo.name.clone() };
    let by = (!by.is_empty()).then_some(by);
    let user = m.target.as_ref().map(|t| t.id.as_str()).filter(|id| !id.is_empty());
    let (target, how) = match (m.action.as_str(), m.message_id.as_deref(), user) {
        ("delete", Some(id), _) => (GoneTarget::Message(id), GoneHow::Deleted),
        ("timeout", _, Some(u)) => (GoneTarget::User(u), GoneHow::TimedOut),
        ("ban", _, Some(u)) => (GoneTarget::User(u), GoneHow::Banned),
        _ => return,
    };
    let secs = secs.filter(|_| how == GoneHow::TimedOut);
    crate::chat::gone_by(app, account, login, target, Gone::new(how, by).with_secs(secs));
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
            moderate(app, account, channel_id, login, &m, secs, now);
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

fn sent_secs(app: &AppHandle, channel_id: &str, login: &str, now: i64) -> Option<u32> {
    let own = app.try_state::<Own>()?;
    own.0.lock().unwrap_or_else(|p| p.into_inner()).sent_secs(channel_id, login, now)
}

/// A timeout's length from `expires_at`, if it's a timeout. V107-R44 To the nearest
/// second: the event arrives a little after it started, so 7 s must not read 6 s.
fn timeout_secs(m: &Moderate, now_ms: i64) -> Option<u64> {
    let exp = m.expires_at.as_deref()?;
    let end = OffsetDateTime::parse(exp, &Rfc3339).ok()?;
    left_secs((end.unix_timestamp_nanos() / 1_000_000) as i64, now_ms)
}

/// Seconds from `now_ms` to `end_ms`, rounded; none once it's over.
fn left_secs(end_ms: i64, now_ms: i64) -> Option<u64> {
    let secs = (end_ms - now_ms + 500).div_euclid(1000);
    (secs > 0).then_some(secs as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v107_r44_timeout_lengths_round() {
        assert_eq!(left_secs(7_000, 300), Some(7), "7 s seen 0.3 s late");
        assert_eq!(left_secs(7_000, 600), Some(6));
        assert_eq!(left_secs(600_000, 900), Some(599));
        assert_eq!(left_secs(1_000, 1_000), None);
        assert_eq!(left_secs(1_000, 5_000), None);
    }
}
