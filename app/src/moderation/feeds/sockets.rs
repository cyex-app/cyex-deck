//! V107-R17 The mod feeds' EventSub sockets per account. AutoMod for every channel comes
//! first (`subs::plan`); a second socket opens only when the first can't hold every
//! channel's AutoMod. Never more than two per account, and never past Twitch's three per
//! user token (`cyex_eventsub::gate`, which every socket goes through). A channel's subscriptions all
//! stay on one socket. Subscribes and deletes go out paced; a 429 pauses the account.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use cyex_eventsub::gate;
use cyex_eventsub::session::{self, Out, Session};
use cyex_eventsub::subs::{self, Answer, ChannelSubs};
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use super::Feeds;
use crate::auth;

/// Mod feed sockets per account, at most.
pub const MAX_SOCKETS: usize = 2;
/// Between two subscribe or delete calls of one account: at most 600 a minute, under
/// Helix's 800-point bucket.
const SPACING_MS: i64 = 100;
/// After the shown tab changes, before its channel's set is planned: quick switches settle.
const SHOWN_SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

/// The shown tab changed: its channel's full set gets priority on the next plan. Another
/// socket of a token waiting for its slot takes back our second (`give_back`).
pub(super) fn init(app: &AppHandle) {
    let a = app.clone();
    gate::GATE.on_pressure(move |acc| give_back(&a, acc));
    let Some(mut wake) = crate::chat::shown::wake(app) else { return };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while wake.changed().await.is_ok() {
            tokio::time::sleep(SHOWN_SETTLE).await;
            wake.borrow_and_update();
            super::sync(&app);
        }
    });
}

/// One socket and what is made on it.
pub(super) struct Conn {
    key: u64,
    _session: Session,
    session_id: Option<String>,
    /// V107-A2 Per channel id, what is made on this session. Closed channels stay: their
    /// subscriptions live until the socket ends or they're pushed out.
    subs: HashMap<String, ChannelSubs>,
    /// Channels with a task running, and how many subscriptions those still make.
    pending: HashSet<String>,
    in_flight: usize,
    /// A retry of failed subscriptions is scheduled.
    retry_armed: bool,
}

impl Conn {
    fn held(&self) -> usize {
        self.subs.values().map(ChannelSubs::held).sum()
    }

    /// Whether the channel lives on this socket: made, waiting to retry, or in a task.
    fn homes(&self, cid: &str) -> bool {
        self.pending.contains(cid) || self.subs.get(cid).is_some_and(|s| s.held() > 0 || s.retry_at().is_some())
    }

    /// Channels holding subscriptions here.
    fn holding(&self) -> impl Iterator<Item = &String> {
        self.subs.iter().filter(|(_, s)| s.held() > 0).map(|(c, _)| c)
    }

    /// A new session (or none): nothing is made on it yet.
    fn reset(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
        self.subs.clear();
        self.pending.clear();
        self.in_flight = 0;
        self.retry_armed = false;
    }
}

pub(super) struct Account {
    conns: Vec<Conn>,
    /// Channels this account mods and that are open: channel id → login.
    pub(super) wanted: HashMap<String, String>,
    /// V107-A2 When each wanted channel was opened (a counter), and which are shown.
    opened: HashMap<String, u64>,
    next_open: u64,
    front: HashSet<String>,
    /// After a 429: no new subscribes on any socket before this.
    full_until_ms: i64,
    /// Last logged (channels with AutoMod, channels).
    coverage: Option<(usize, usize)>,
}

impl Account {
    pub(super) fn new(app: &AppHandle, account: &str) -> Self {
        Account {
            conns: vec![open(app, account)],
            wanted: HashMap::new(),
            opened: HashMap::new(),
            next_open: 0,
            front: HashSet::new(),
            full_until_ms: 0,
            coverage: None,
        }
    }

    /// New open channels and which are shown.
    pub(super) fn set_wanted(&mut self, channels: HashMap<String, String>, front: &HashSet<String>) {
        self.opened.retain(|c, _| channels.contains_key(c));
        for c in channels.keys() {
            if !self.opened.contains_key(c) {
                self.opened.insert(c.clone(), self.next_open);
                self.next_open += 1;
            }
        }
        self.front = channels.keys().filter(|c| front.contains(*c)).cloned().collect();
        self.wanted = channels;
    }

    /// Whether a socket holds subscriptions for the channel.
    pub(super) fn covers(&self, cid: &str) -> bool {
        self.conns.iter().any(|c| c.subs.get(cid).is_some_and(|s| s.held() > 0))
    }

    /// Channels holding subscriptions on any socket (all go when the account's feed ends).
    pub(super) fn held_channels(&self) -> Vec<String> {
        self.conns.iter().flat_map(Conn::holding).cloned().collect()
    }

    /// D30 The stream manager let go of `channel.follow` here: the channel's socket takes it,
    /// counted, when it has room and the type isn't made or refused already. Returns the
    /// session to make it on.
    pub(super) fn take_follow(&mut self, cid: &str) -> Option<String> {
        let k = self.home(cid).unwrap_or(0);
        let conn = self.conns.get_mut(k)?;
        let sid = conn.session_id.clone()?;
        let i = subs::MOD_TYPES.iter().position(|(t, _)| *t == "channel.follow")?;
        if conn.pending.contains(cid) || conn.held() + conn.in_flight >= subs::MAX_PER_SESSION {
            return None;
        }
        let s = conn.subs.entry(cid.to_owned()).or_default();
        if s.has("channel.follow") || !s.todo(i64::MAX).contains(&i) {
            return None;
        }
        s.answer(i, Answer::Done, crate::chat::now_ms());
        Some(sid)
    }

    fn home(&self, cid: &str) -> Option<usize> {
        self.conns.iter().position(|c| c.homes(cid))
    }

    /// Wanted channels in priority order: shown first, then the first opened.
    fn by_priority(&self) -> Vec<String> {
        let mut ids: Vec<&String> = self.wanted.keys().collect();
        ids.sort_by_key(|c| (!self.front.contains(*c), self.opened.get(*c).copied().unwrap_or(u64::MAX), (*c).clone()));
        ids.into_iter().cloned().collect()
    }
}

/// One socket's work: deletes first, then subscribes.
struct Job {
    session_id: String,
    drops: Vec<(String, usize, String)>,
    adds: Vec<(String, usize)>,
}

/// Deletes and subscribes for one socket, as (channel, type).
type Work = (Vec<(String, usize)>, Vec<(String, usize)>);

/// What [`plan_all`] decided across the sockets.
#[derive(Debug, Default, PartialEq, Eq)]
struct Spread {
    /// Per socket that is welcomed: (deletes, subscribes) as (channel, type).
    per_conn: Vec<Option<Work>>,
    /// Channels without AutoMod after every socket had its turn.
    without: usize,
    /// Channels no socket can take yet (no AutoMod room, not placed anywhere).
    unplaced: usize,
    /// Every socket was welcomed and planned.
    all_planned: bool,
}

/// The socket state [`plan_all`] reads, so it runs without tauri in tests.
struct View<'a> {
    session: bool,
    subs: &'a HashMap<String, ChannelSubs>,
    pending: &'a HashSet<String>,
    in_flight: usize,
}

impl View<'_> {
    fn homes(&self, cid: &str) -> bool {
        self.pending.contains(cid) || self.subs.get(cid).is_some_and(|s| s.held() > 0 || s.retry_at().is_some())
    }
}

/// Plans every socket: each takes its own channels, the first also the new ones; new ones
/// whose AutoMod doesn't fit go on to the next socket.
fn plan_all(order: &[String], front: &HashSet<String>, conns: &[View], now: i64) -> Spread {
    let homes: Vec<Option<usize>> = order.iter().map(|c| conns.iter().position(|v| v.homes(c))).collect();
    let mut carry: HashSet<&str> = order.iter().zip(&homes).filter(|(_, h)| h.is_none()).map(|(c, _)| c.as_str()).collect();
    let mut out = Spread { all_planned: true, ..Spread::default() };
    for (k, v) in conns.iter().enumerate() {
        if !v.session {
            // New channels wait for this socket rather than spill onto the next.
            out.per_conn.push(None);
            out.all_planned = false;
            carry.clear();
            continue;
        }
        let list: Vec<&str> = order
            .iter()
            .zip(&homes)
            .filter(|(c, h)| **h == Some(k) || (h.is_none() && carry.contains(c.as_str())))
            .map(|(c, _)| c.as_str())
            .collect();
        let fronts = list.iter().take_while(|c| front.contains(**c)).count();
        let p = subs::plan(&list, fronts, v.subs, v.pending, v.in_flight, now, subs::MAX_PER_SESSION);
        let own = |l: &[(&str, usize)]| l.iter().map(|(c, i)| ((*c).to_owned(), *i)).collect::<Vec<_>>();
        out.without += p.without_automod.iter().filter(|c| !carry.contains(**c)).count();
        let next: HashSet<&str> = p.without_automod.iter().filter_map(|c| carry.get(*c).copied()).collect();
        out.per_conn.push(Some((own(&p.drop), own(&p.add))));
        carry = next;
    }
    out.unplaced = carry.len();
    out.without += carry.len();
    out
}

/// V107-A2 Starts what the sockets still need: new channels, failed types whose wait is
/// over, room for the shown tab. Opens or closes the second socket. Called with the feeds
/// lock held; returns channels whose follows went with a closed socket.
pub(super) fn queue(app: &AppHandle, account: &str, entry: &mut Account) -> Vec<String> {
    let Some(token) = auth::helix_token(app, account) else { return Vec::new() };
    let now = crate::chat::now_ms();
    if now < entry.full_until_ms {
        return Vec::new();
    }
    let order = entry.by_priority();
    let views: Vec<View> = entry
        .conns
        .iter()
        .map(|c| View { session: c.session_id.is_some(), subs: &c.subs, pending: &c.pending, in_flight: c.in_flight })
        .collect();
    let spread = plan_all(&order, &entry.front, &views, now);
    drop(views);
    let mut jobs = Vec::new();
    for (conn, planned) in entry.conns.iter_mut().zip(&spread.per_conn) {
        let (Some((drops, adds)), Some(sid)) = (planned, conn.session_id.clone()) else { continue };
        if drops.is_empty() && adds.is_empty() {
            continue;
        }
        let drops: Vec<(String, usize, String)> =
            drops.iter().filter_map(|(c, i)| Some((c.clone(), *i, conn.subs.get(c)?.id(*i)?.to_owned()))).collect();
        conn.pending.extend(drops.iter().map(|(c, _, _)| c.clone()).chain(adds.iter().map(|(c, _)| c.clone())));
        conn.in_flight += adds.len();
        jobs.push(Job { session_id: sid, drops, adds: adds.clone() });
    }
    let m = order.len();
    if spread.all_planned {
        let cov = (m - spread.without.min(m), m);
        if cov.0 < m && entry.coverage != Some(cov) {
            eprintln!("mod feeds: {account}: AutoMod for {} of {m} channels", cov.0);
        }
        entry.coverage = Some(cov);
    }
    let released = second_socket(app, account, entry, &order, &spread);
    for job in jobs {
        run(app, account, token.clone(), job);
    }
    released
}

/// Opens the second socket when new channels have no room for AutoMod, closes it once no
/// open channel lives there (or whispers or the stream manager need the slot).
fn second_socket(app: &AppHandle, account: &str, entry: &mut Account, order: &[String], spread: &Spread) -> Vec<String> {
    let allowed = gate::PER_TOKEN.saturating_sub(others(app, account, entry.conns.len())) >= MAX_SOCKETS;
    let used = entry.conns.get(1).is_some_and(|c| c.in_flight > 0 || !c.pending.is_empty() || order.iter().any(|ch| c.homes(ch)));
    match second(entry.conns.len(), spread, used, allowed) {
        Second::Open => {
            eprintln!("mod feeds: {account}: opening a second connection for AutoMod");
            entry.conns.push(open(app, account));
        }
        Second::Close => {
            eprintln!("mod feeds: {account}: second connection closed");
            if let Some(c) = entry.conns.pop() {
                return c.holding().cloned().collect();
            }
        }
        Second::Keep => {}
    }
    Vec::new()
}

#[derive(Debug, PartialEq, Eq)]
enum Second {
    Open,
    Close,
    Keep,
}

/// Whether to open or close the second socket. `used`: an open channel lives there or a
/// task runs on it. `allowed`: Twitch's three per token leave room for two of ours.
fn second(sockets: usize, spread: &Spread, used: bool, allowed: bool) -> Second {
    if !spread.per_conn.first().is_some_and(Option::is_some) {
        return Second::Keep;
    }
    let need = spread.unplaced > 0 || used;
    match sockets {
        1 if need && allowed => Second::Open,
        MAX_SOCKETS if spread.all_planned && (!need || !allowed) => Second::Close,
        _ => Second::Keep,
    }
}

/// Other EventSub sockets on this account's token: whispers and the stream manager.
/// The gate's count (sockets open or waiting, less ours) or what the features say, the larger.
fn others(app: &AppHandle, account: &str, ours: usize) -> usize {
    let (open, waiting) = gate::GATE.count(account);
    let gated = (open + waiting).saturating_sub(ours);
    let whispers = crate::whispers::eventsub_account(app).is_some_and(|a| a == account);
    let manager = crate::streammanager::own_channel(app).is_some_and(|(a, _, _)| a == account && auth::gql_token(app, &a).is_none());
    gated.max(usize::from(whispers) + usize::from(manager))
}

/// V107-R17 Another socket of this token waits for a slot: the second mod socket closes
/// first (its channels' AutoMod moves back to the first by priority), so there are never 4.
fn give_back(app: &AppHandle, account: &str) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    let Some(entry) = map.get_mut(account).filter(|e| e.conns.len() == MAX_SOCKETS) else { return };
    eprintln!("mod feeds: {account}: second connection closed, another feature needs the slot");
    let mut released: Vec<String> = entry.conns.pop().map(|c| c.holding().cloned().collect()).unwrap_or_default();
    released.extend(queue(app, account, entry));
    drop(map);
    for cid in released {
        crate::streammanager::follow_released(app, &cid);
    }
}

fn open(app: &AppHandle, account: &str) -> Conn {
    static KEYS: AtomicU64 = AtomicU64::new(0);
    let key = KEYS.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (session, task) = session::start(account, tx);
    tauri::async_runtime::spawn(task);
    let app2 = app.clone();
    let acc = account.to_owned();
    tauri::async_runtime::spawn(async move {
        while let Some(out) = rx.recv().await {
            match out {
                Out::Welcome(id) => welcome(&app2, &acc, key, Some(id)),
                Out::Lost => welcome(&app2, &acc, key, None),
                Out::Revoked { ref kind, ref condition, .. } => {
                    revoked(&app2, &acc, key, kind, condition);
                    super::handle(&app2, &acc, out);
                }
                out => super::handle(&app2, &acc, out),
            }
        }
    });
    Conn {
        key,
        _session: session,
        session_id: None,
        subs: HashMap::new(),
        pending: HashSet::new(),
        in_flight: 0,
        retry_armed: false,
    }
}

/// A socket's new session (or its loss): nothing is made on it now.
fn welcome(app: &AppHandle, account: &str, key: u64, session_id: Option<String>) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    let Some(entry) = map.get_mut(account) else { return };
    let Some(conn) = entry.conns.iter_mut().find(|c| c.key == key) else { return };
    let fresh = session_id.clone();
    let lost: Vec<String> = conn.holding().cloned().collect();
    conn.reset(session_id);
    let released = if fresh.is_some() { queue(app, account, entry) } else { Vec::new() };
    let wait = fresh.filter(|_| entry.full_until_ms > crate::chat::now_ms());
    drop(map);
    // Its follows went with the old session.
    for cid in lost.into_iter().chain(released) {
        crate::streammanager::follow_released(app, &cid);
    }
    // Paused by a 429: subscribe once the pause is over (Twitch closes an unused socket).
    if let Some(sid) = wait {
        retry_later(app, account, &sid);
    }
}

/// Twitch ended one subscription (e.g. unmodded): its room is free again.
fn revoked(app: &AppHandle, account: &str, key: u64, kind: &str, condition: &serde_json::Value) {
    let (Some(cid), Some(i)) = (subs::channel_of(condition), subs::MOD_TYPES.iter().position(|(t, _)| *t == kind)) else {
        return;
    };
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    let conn = map.get_mut(account).and_then(|e| e.conns.iter_mut().find(|c| c.key == key));
    if let Some(s) = conn.and_then(|c| c.subs.get_mut(cid)) {
        s.dropped(i);
        // Not again on this session.
        s.answer(i, Answer::Final, 0);
    }
}

/// What a task does with the account and its socket, if that session is still the one.
fn with_session<R>(app: &AppHandle, account: &str, session_id: &str, f: impl FnOnce(&mut Account, usize) -> R) -> Option<R> {
    let feeds = app.try_state::<Feeds>()?;
    let mut map = feeds.0.lock().unwrap_or_else(|p| p.into_inner());
    let entry = map.get_mut(account)?;
    let k = entry.conns.iter().position(|c| c.session_id.as_deref() == Some(session_id))?;
    Some(f(entry, k))
}

/// One item of a channel is done: the last one frees the channel for the next plan.
fn finish(conn: &mut Conn, left: &mut HashMap<String, usize>, cid: &str) {
    if let Some(n) = left.get_mut(cid) {
        *n = n.saturating_sub(1);
        if *n == 0 {
            conn.pending.remove(cid);
        }
    }
}

/// Waits for this account's next call slot ([`SPACING_MS`]).
async fn pace(account: &str) {
    static NEXT: Mutex<Option<HashMap<String, i64>>> = Mutex::new(None);
    let wait = {
        let mut next = NEXT.lock().unwrap_or_else(|p| p.into_inner());
        let next = next.get_or_insert_with(HashMap::new);
        let now = crate::chat::now_ms();
        let at = next.get(account).copied().unwrap_or(0).max(now);
        next.insert(account.to_owned(), at + SPACING_MS);
        at - now
    };
    if wait > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(wait as u64)).await;
    }
}

/// Deletes, then subscribes, one answer at a time into the socket's state. Never sends a
/// subscribe that would pass the socket's cap, even when a delete failed.
fn run(app: &AppHandle, account: &str, (token, client_id): (String, String), job: Job) {
    let app = app.clone();
    let account = account.to_owned();
    tauri::async_runtime::spawn(async move {
        let http = super::super::http(&app);
        let auth = helix::Auth { client_id: &client_id, token: &token };
        let Job { session_id, drops, adds } = job;
        let mut left: HashMap<String, usize> = HashMap::new();
        for c in drops.iter().map(|d| &d.0).chain(adds.iter().map(|a| &a.0)) {
            *left.entry(c.clone()).or_default() += 1;
        }
        for (cid, i, id) in drops {
            pace(&account).await;
            let r = helix::eventsub::unsubscribe(&http, auth, &id).await;
            let gone = match &r {
                Ok(()) => true,
                Err(e) => e.status() == Some(404),
            };
            if let Err(e) = &r
                && !gone
            {
                eprintln!("mod feeds: {cid}: couldn't remove {}: {e}", subs::MOD_TYPES[i].0);
            }
            let live = with_session(&app, &account, &session_id, |e, k| {
                let c = &mut e.conns[k];
                if gone && let Some(s) = c.subs.get_mut(&cid) {
                    s.dropped(i);
                }
                finish(c, &mut left, &cid);
            });
            if live.is_none() {
                return;
            }
            if gone && subs::MOD_TYPES[i].0 == "channel.follow" {
                crate::streammanager::follow_released(&app, &cid);
            }
        }
        let mut full = false;
        for (cid, i) in adds {
            let go = with_session(&app, &account, &session_id, |e, k| {
                let c = &mut e.conns[k];
                let ok = !full && c.held() + c.in_flight <= subs::MAX_PER_SESSION;
                if !ok {
                    c.in_flight = c.in_flight.saturating_sub(1);
                    finish(c, &mut left, &cid);
                }
                ok
            });
            match go {
                None => return,
                Some(false) => continue,
                Some(true) => {}
            }
            pace(&account).await;
            let (kind, version) = subs::MOD_TYPES[i];
            let cond = serde_json::json!({ "broadcaster_user_id": cid, "moderator_user_id": account });
            let (answer, id) = match helix::eventsub::subscribe(&http, auth, &session_id, kind, version, &cond).await {
                Ok(id) => (Answer::Done, Some(id)),
                Err(e) => {
                    let a = subs::classify(e.status(), e.api_message());
                    // Status and Twitch's message only (the error never carries the token).
                    if a != Answer::Done && super::first_failure(&cid, kind) {
                        eprintln!("mod feeds: {cid}: couldn't subscribe {kind} ({a:?}): {e}");
                    }
                    (a, None)
                }
            };
            full = answer == Answer::Full;
            let now = crate::chat::now_ms();
            let live = with_session(&app, &account, &session_id, |e, k| {
                if full {
                    e.full_until_ms = now + subs::RETRY_MS;
                }
                let c = &mut e.conns[k];
                c.in_flight = c.in_flight.saturating_sub(1);
                let s = c.subs.entry(cid.clone()).or_default();
                match id {
                    Some(id) => s.made_as(i, id, now),
                    None => s.answer(i, answer, now),
                }
                finish(c, &mut left, &cid);
            });
            // The socket was replaced: the new session subscribes for itself.
            if live.is_none() {
                return;
            }
        }
        retry_later(&app, &account, &session_id);
    });
}

/// V107-A2 Failed subscriptions go again once their wait is over, without waiting for a sync.
fn retry_later(app: &AppHandle, account: &str, session_id: &str) {
    let now = crate::chat::now_ms();
    let Some(at) = with_session(app, account, session_id, |e, k| {
        let paused = e.full_until_ms;
        let c = &mut e.conns[k];
        let mut at = c.subs.values().filter_map(ChannelSubs::retry_at).min();
        if paused > now {
            at = Some(at.unwrap_or(paused).max(paused));
        }
        let at = at.filter(|_| !c.retry_armed)?;
        c.retry_armed = true;
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
        let released = with_session(&app, &account, &session_id, move |e, k| {
            e.conns[k].retry_armed = false;
            queue(&app2, &acc, e)
        });
        for cid in released.unwrap_or_default() {
            crate::streammanager::follow_released(&app, &cid);
        }
    });
}

#[cfg(test)]
mod tests;
