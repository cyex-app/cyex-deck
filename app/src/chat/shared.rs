//! V108-N12 Shared chat as a viewer: while a joined channel is in a Shared Chat session its
//! rooms get short system lines (started, joined, left, ended) and the UI the current
//! participants (`chat:shared`, `chat_shared_state`). Comes from EventSub
//! (`channel.shared_chat.*`, no scope), one socket per account with a Helix sign-in that has
//! the channel open; a GQL-only account gets nothing. A join also reads the running session
//! once. Per-channel subscriptions follow the channel registry (`Known` / `Gone`, V107-R16).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use cyex_eventsub::events::{SharedChat, Who, parse_shared};
use cyex_eventsub::session::{self, Out, Session};
use cyex_eventsub::subs;
use cyex_twitch::helix;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::channels::Event;
use super::{Chat, Identity};
use crate::auth;

const EVENT: &str = "chat:shared";

/// Someone in a shared chat session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SharedParticipant {
    pub id: String,
    pub login: String,
    /// Display name; the login when Twitch sent none.
    pub name: String,
}

impl From<&Who> for SharedParticipant {
    fn from(w: &Who) -> Self {
        Self {
            id: w.id.clone(),
            login: w.login.clone(),
            name: if w.name.is_empty() { w.login.clone() } else { w.name.clone() },
        }
    }
}

/// `chat:shared`: everyone in the chat's session now, the chat's own channel included;
/// empty when the session ended.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SharedChatUpdate {
    /// Channel login.
    pub channel: String,
    pub participants: Vec<SharedParticipant>,
}

/// The session a channel is in.
struct Current {
    session: String,
    who: Vec<SharedParticipant>,
}

/// A channel that is followed.
struct Chan {
    id: String,
    account: String,
    /// Subscription ids on the account's current socket session.
    subs: Vec<String>,
    current: Option<Current>,
}

struct Sock {
    _session: Session,
    /// `None` while it connects or after the connection dropped.
    session_id: Option<String>,
}

#[derive(Default)]
struct Inner {
    /// By channel login.
    chans: HashMap<String, Chan>,
    /// By account id; one socket holds the channels the account has open.
    socks: HashMap<String, Sock>,
}

#[derive(Default)]
struct Shared {
    inner: Mutex<Inner>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
}

pub fn init(app: &AppHandle) {
    app.manage(Shared::default());
    app.state::<Chat>().channels.hook(on_channel);
}

/// The participants now, for a view that opens mid-session. Empty when there is none.
pub fn state(app: &AppHandle, channel: &str) -> Vec<SharedParticipant> {
    let Some(shared) = app.try_state::<Shared>() else { return Vec::new() };
    let inner = shared.lock();
    inner
        .chans
        .get(&channel.to_ascii_lowercase())
        .and_then(|c| c.current.as_ref())
        .map(|c| c.who.clone())
        .unwrap_or_default()
}

/// A channel's first room knows its id: follow it. Its last room closing ends that (no event).
fn on_channel(app: &AppHandle, e: &Event) {
    match e {
        Event::Known { login, id, generation } => {
            let (app, login, id, generation) = (app.clone(), (*login).to_owned(), (*id).to_owned(), *generation);
            tauri::async_runtime::spawn(async move { attach(&app, login, id, generation).await });
        }
        Event::Gone { login, .. } => detach(app, login),
        _ => {}
    }
}

/// An account with a Helix sign-in that has `login` open.
fn account_for(app: &AppHandle, login: &str) -> Option<String> {
    let chat = app.state::<Chat>();
    let inner = chat.lock();
    inner
        .rooms
        .keys()
        .filter(|(_, l)| l == login)
        .filter_map(|(i, _)| match i {
            Identity::Account(a) => Some(a),
            Identity::Anon => None,
        })
        .find(|a| auth::helix_token(app, a).is_some())
        .cloned()
}

async fn attach(app: &AppHandle, login: String, id: String, generation: u64) {
    let Some(account) = account_for(app, &login) else { return };
    let session_id = {
        let shared = app.state::<Shared>();
        let mut inner = shared.lock();
        if !app.state::<Chat>().channels.current(&login, generation) {
            return;
        }
        let chan = Chan { id: id.clone(), account: account.clone(), subs: Vec::new(), current: None };
        inner.chans.insert(login.clone(), chan);
        let sock = inner.socks.entry(account.clone()).or_insert_with(|| start_sock(app, &account));
        sock.session_id.clone()
    };
    if let Some(session_id) = session_id {
        subscribe(app, &account, &login, &id, &session_id).await;
    }
    read_running(app, &account, &login, &id).await;
}

fn detach(app: &AppHandle, login: &str) {
    let Some(shared) = app.try_state::<Shared>() else { return };
    let (account, subs) = {
        let mut inner = shared.lock();
        let Some(chan) = inner.chans.remove(login) else { return };
        // The account's last channel ends its socket, and with it every subscription.
        if !inner.chans.values().any(|c| c.account == chan.account) {
            inner.socks.remove(&chan.account);
            return;
        }
        (chan.account, chan.subs)
    };
    if subs.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some((token, client_id)) = auth::helix_token(&app, &account) else { return };
        let http = app.state::<auth::Auth>().http.clone();
        let a = auth::helix_auth(&token, &client_id);
        for id in subs {
            let _ = helix::eventsub::unsubscribe(&http, a, &id).await;
        }
    });
}

fn start_sock(app: &AppHandle, account: &str) -> Sock {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (session, task) = session::start(account, tx);
    tauri::async_runtime::spawn(task);
    let (app, account) = (app.clone(), account.to_owned());
    tauri::async_runtime::spawn(async move {
        while let Some(out) = rx.recv().await {
            on_out(&app, &account, out);
        }
    });
    Sock { _session: session, session_id: None }
}

fn on_out(app: &AppHandle, account: &str, out: Out) {
    match out {
        Out::Welcome(session_id) => {
            let targets: Vec<(String, String)> = {
                let shared = app.state::<Shared>();
                let mut inner = shared.lock();
                let Some(sock) = inner.socks.get_mut(account) else { return };
                sock.session_id = Some(session_id.clone());
                inner
                    .chans
                    .iter_mut()
                    .filter(|(_, c)| c.account == account)
                    .map(|(l, c)| {
                        c.subs.clear();
                        (l.clone(), c.id.clone())
                    })
                    .collect()
            };
            for (login, id) in targets {
                let (app, account, session_id) = (app.clone(), account.to_owned(), session_id.clone());
                tauri::async_runtime::spawn(async move { subscribe(&app, &account, &login, &id, &session_id).await });
            }
        }
        Out::Lost => {
            let shared = app.state::<Shared>();
            let mut inner = shared.lock();
            if let Some(sock) = inner.socks.get_mut(account) {
                sock.session_id = None;
            }
            inner.chans.values_mut().filter(|c| c.account == account).for_each(|c| c.subs.clear());
        }
        Out::Notification { kind, condition, event } => {
            let Some(cid) = subs::channel_of(&condition) else { return };
            let login = {
                let shared = app.state::<Shared>();
                let inner = shared.lock();
                inner.chans.iter().find(|(_, c)| c.account == account && c.id == cid).map(|(l, _)| l.clone())
            };
            if let (Some(login), Some(ev)) = (login, parse_shared(&kind, &event)) {
                deliver(app, &login, ev, false);
            }
        }
        Out::Revoked { kind, .. } => eprintln!("shared chat: lost {kind}"),
    }
}

/// Makes the three subscriptions for one channel on the account's socket session.
async fn subscribe(app: &AppHandle, account: &str, login: &str, id: &str, session_id: &str) {
    let Some((token, client_id)) = auth::helix_token(app, account) else { return };
    let http = app.state::<auth::Auth>().http.clone();
    let a = auth::helix_auth(&token, &client_id);
    let mut made = Vec::new();
    for (kind, version, cond) in subs::shared_chat(id) {
        match helix::eventsub::subscribe(&http, a, session_id, kind, version, &cond).await {
            Ok(sub) => made.push(sub),
            Err(e) if e.is_already() => {}
            Err(e) => eprintln!("shared chat: couldn't follow {kind} in {login}: {e}"),
        }
    }
    // The channel was left meanwhile: take them back.
    let stale = {
        let shared = app.state::<Shared>();
        let mut inner = shared.lock();
        match inner.chans.get_mut(login).filter(|c| c.id == id) {
            Some(c) => {
                c.subs.extend(made);
                Vec::new()
            }
            None => made,
        }
    };
    for sub in stale {
        let _ = helix::eventsub::unsubscribe(&http, a, &sub).await;
    }
}

/// One read of the session a channel is already in when it is joined. Helix gives ids only,
/// so the names come from one users lookup.
async fn read_running(app: &AppHandle, account: &str, login: &str, id: &str) {
    let Some((token, client_id)) = auth::helix_token(app, account) else { return };
    let http = app.state::<auth::Auth>().http.clone();
    let a = auth::helix_auth(&token, &client_id);
    let session = match helix::shared_chat::session(&http, a, id).await {
        Ok(Some(s)) => s,
        Ok(None) => return,
        Err(e) => {
            eprintln!("shared chat: couldn't read the session of {login}: {e}");
            return;
        }
    };
    let ids: Vec<&str> = session.participants.iter().map(|p| p.broadcaster_id.as_str()).collect();
    let Ok(users) = helix::users_by_id(&http, a, &ids).await else { return };
    let participants = users
        .into_iter()
        .map(|u| Who { id: u.id, login: u.login, name: u.display_name })
        .collect();
    let ev = SharedChat::Now { session_id: session.session_id, begin: false, participants };
    deliver(app, login, ev, true);
}

/// Applies an event to the channel's state; its lines go into every room, the new
/// participants to the UI. `from_read`: the one-time read gives way to any event that came first.
fn deliver(app: &AppHandle, login: &str, ev: SharedChat, from_read: bool) {
    let change = {
        let shared = app.state::<Shared>();
        let mut inner = shared.lock();
        let Some(chan) = inner.chans.get_mut(login) else { return };
        if from_read && chan.current.is_some() {
            return;
        }
        apply(&mut chan.current, ev, &chan.id)
    };
    let Some(change) = change else { return };
    for line in &change.lines {
        super::system_line(app, login, line);
    }
    let _ = app.emit(EVENT, SharedChatUpdate { channel: login.to_owned(), participants: change.publish });
}

struct Change {
    lines: Vec<String>,
    /// Everyone in the session now; empty once it ended.
    publish: Vec<SharedParticipant>,
}

/// The state after `ev`, and what to tell. `me` is the chat's own channel id, left out of
/// every sentence. `None`: nothing changed.
fn apply(cur: &mut Option<Current>, ev: SharedChat, me: &str) -> Option<Change> {
    match ev {
        SharedChat::End { session_id } => {
            if cur.as_ref()?.session != session_id {
                return None;
            }
            *cur = None;
            Some(Change { lines: vec!["Shared chat ended.".into()], publish: Vec::new() })
        }
        SharedChat::Now { session_id, begin, participants } => {
            let now: Vec<SharedParticipant> = participants.iter().map(SharedParticipant::from).collect();
            let lines = match cur.as_ref().filter(|c| c.session == session_id) {
                Some(prev) => changes(&prev.who, &now, me),
                None => vec![intro(&now, me, begin || cur.is_some())],
            };
            *cur = Some(Current { session: session_id, who: now.clone() });
            Some(Change { lines, publish: now })
        }
    }
}

/// "Shared chat started with X and Y." / "In a shared chat with X and Y."
fn intro(now: &[SharedParticipant], me: &str, started: bool) -> String {
    let others = display_names(now.iter().filter(|p| p.id != me));
    match (started, others.is_empty()) {
        (true, true) => "Shared chat started.".into(),
        (true, false) => format!("Shared chat started with {}.", list(&others)),
        (false, true) => "In a shared chat.".into(),
        (false, false) => format!("In a shared chat with {}.", list(&others)),
    }
}

/// "Y joined the shared chat." and "Y left the shared chat.", for the difference.
fn changes(prev: &[SharedParticipant], now: &[SharedParticipant], me: &str) -> Vec<String> {
    // Names in `a` that `b` lacks, not counting the chat's own channel.
    fn only<'a>(a: &'a [SharedParticipant], b: &[SharedParticipant], me: &str) -> Vec<&'a str> {
        display_names(a.iter().filter(|p| p.id != me && !b.iter().any(|q| q.id == p.id)))
    }
    let (joined, left) = (only(now, prev, me), only(prev, now, me));
    let mut lines = Vec::new();
    if !joined.is_empty() {
        lines.push(format!("{} joined the shared chat.", list(&joined)));
    }
    if !left.is_empty() {
        lines.push(format!("{} left the shared chat.", list(&left)));
    }
    lines
}

fn display_names<'a>(ps: impl Iterator<Item = &'a SharedParticipant>) -> Vec<&'a str> {
    ps.map(|p| p.name.as_str()).collect()
}

/// "X", "X and Y", "X, Y and Z".
fn list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(id: &str, name: &str) -> Who {
        Who { id: id.into(), login: name.to_lowercase(), name: name.into() }
    }

    fn now(session: &str, begin: bool, ids: &[(&str, &str)]) -> SharedChat {
        SharedChat::Now {
            session_id: session.into(),
            begin,
            participants: ids.iter().map(|(i, n)| who(i, n)).collect(),
        }
    }

    #[test]
    fn names_read_as_a_list() {
        assert_eq!(list(&[]), "");
        assert_eq!(list(&["A"]), "A");
        assert_eq!(list(&["A", "B"]), "A and B");
        assert_eq!(list(&["A", "B", "C"]), "A, B and C");
    }

    #[test]
    fn a_session_from_start_to_end() {
        let mut cur = None;
        let c = apply(&mut cur, now("s", true, &[("1", "Me"), ("2", "Xe"), ("3", "Ye")]), "1").unwrap();
        assert_eq!(c.lines, ["Shared chat started with Xe and Ye."]);
        assert_eq!(c.publish.len(), 3, "the UI drops the channel itself");

        let c = apply(&mut cur, now("s", false, &[("1", "Me"), ("2", "Xe"), ("3", "Ye"), ("4", "Ze")]), "1").unwrap();
        assert_eq!(c.lines, ["Ze joined the shared chat."]);
        let c = apply(&mut cur, now("s", false, &[("1", "Me"), ("2", "Xe")]), "1").unwrap();
        assert_eq!(c.lines, ["Ye and Ze left the shared chat."]);
        // Someone joins and someone leaves at once.
        let c = apply(&mut cur, now("s", false, &[("1", "Me"), ("3", "Ye")]), "1").unwrap();
        assert_eq!(c.lines, ["Ye joined the shared chat.", "Xe left the shared chat."]);
        // Nothing changed (e.g. a rename): no line, the UI still hears it.
        let c = apply(&mut cur, now("s", false, &[("1", "Me"), ("3", "Ye")]), "1").unwrap();
        assert!(c.lines.is_empty() && c.publish.len() == 2);

        let c = apply(&mut cur, SharedChat::End { session_id: "s".into() }, "1").unwrap();
        assert_eq!((c.lines, c.publish), (vec!["Shared chat ended.".to_owned()], Vec::new()));
        assert!(cur.is_none());
        assert!(apply(&mut cur, SharedChat::End { session_id: "s".into() }, "1").is_none(), "twice says nothing");
    }

    #[test]
    fn a_session_found_at_join_or_replaced() {
        let mut cur = None;
        let c = apply(&mut cur, now("s", false, &[("1", "Me"), ("2", "Xe")]), "1").unwrap();
        assert_eq!(c.lines, ["In a shared chat with Xe."]);
        // The same session announced again (event and read raced): quiet.
        let c = apply(&mut cur, now("s", true, &[("1", "Me"), ("2", "Xe")]), "1").unwrap();
        assert!(c.lines.is_empty());
        // Another session without an end in between.
        let c = apply(&mut cur, now("t", false, &[("1", "Me"), ("5", "Qe")]), "1").unwrap();
        assert_eq!(c.lines, ["Shared chat started with Qe."]);
        // An end for an older session changes nothing.
        assert!(apply(&mut cur, SharedChat::End { session_id: "s".into() }, "1").is_none());
        assert_eq!(cur.as_ref().unwrap().session, "t");
        // Alone in it.
        let mut alone = None;
        let c = apply(&mut alone, now("u", true, &[("1", "Me")]), "1").unwrap();
        assert_eq!(c.lines, ["Shared chat started."]);
    }

    #[test]
    fn a_missing_display_name_falls_back_to_the_login() {
        let w = Who { id: "2".into(), login: "xe".into(), name: String::new() };
        assert_eq!(SharedParticipant::from(&w).name, "xe");
    }
}
