//! 1.35, X4 The stream manager (spec 10) wired to Tauri. A fixed tab for the signed-in
//! account's own channel: it edits the title and category, runs ads, raids, markers,
//! polls, predictions and chat modes, hosts Stream Together, shows a live activity feed
//! (`activity`) and stats (`stats`), and keeps OBS status files (`obs`). GQL first, the way
//! Twitch's dashboard works; Helix when the account has no GQL sign-in (D30, V101-28).
//! Viewers, title, category and raids come live from Hermes (`live`, D7, D2).
//! Actions are in `actions`; no write is run in testing.

pub mod actions;
pub mod activity;
pub mod follows;
mod gql_feed;
pub mod live;
pub mod obs;
pub mod stats;
pub mod words;

use std::collections::VecDeque;
use std::sync::Mutex;

use cyex_twitch::{gql, helix};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::auth::{self, Auth};

/// The Helix scope that lets an account without a GQL sign-in run the manager.
const SCOPE: &str = "channel:manage:broadcast";
const MAX_ACTIVITY: usize = 200;
/// ~6 hours at one point per 30 s.
const MAX_SERIES: usize = 720;

pub fn init(app: &AppHandle) {
    app.manage(State::default());
    app.manage(follows::Seen::default());
}

#[derive(Default)]
pub struct State {
    /// Newest activity last; capped at `MAX_ACTIVITY`.
    activity: Mutex<VecDeque<activity::Item>>,
    /// Viewer count over time for the X4 graph: (unix seconds, viewers).
    series: Mutex<VecDeque<(i64, u64)>>,
    /// The newest stats tick.
    stats: Mutex<Option<stats::Stats>>,
    /// D2 The raid counting down from the channel.
    raid: Mutex<Option<cyex_twitch::gql::dashboard::raids::RaidStatus>>,
    /// The running feed and poller, while the tab is open. Ref-counted by `opens`.
    live: Mutex<Option<Live>>,
    opens: Mutex<u32>,
}

/// The feed and stats poller that run while the manager tab is open.
pub struct Live {
    pub account: String,
    pub channel_id: String,
    login: String,
    /// The EventSub session, when the feed runs on Helix; `None` on GQL (Hermes).
    session: Option<cyex_eventsub::session::Session>,
    /// Set once the EventSub socket welcomes; cleared when it drops.
    pub session_id: Option<String>,
    /// Stops the stats poller.
    stop: tokio::sync::watch::Sender<bool>,
}

fn state(app: &AppHandle) -> tauri::State<'_, State> {
    app.state::<State>()
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// The tab opened. Ref-counted: the feed and poller start on the first open. Returns the
/// activity already buffered so a fresh view fills in.
pub fn open(app: &AppHandle) -> Vec<activity::Item> {
    let st = state(app);
    *lock(&st.opens) += 1;
    start(app);
    lock(&st.activity).iter().cloned().collect()
}

/// The tab closed. Stops the feed and poller when the last view goes.
pub fn close(app: &AppHandle) {
    let st = state(app);
    {
        let mut opens = lock(&st.opens);
        *opens = opens.saturating_sub(1);
        if *opens > 0 {
            return;
        }
    }
    stop(app);
}

/// Accounts changed: while the tab is open, follow the account that now runs the manager
/// (or stop when none can).
pub fn accounts_changed(app: &AppHandle) {
    let st = state(app);
    if *lock(&st.opens) == 0 {
        return;
    }
    let want = manager(app).map(|m| m.account_id);
    let have = lock(&st.live).as_ref().map(|l| l.account.clone());
    if want != have {
        stop(app);
        start(app);
    }
}

/// Starts the feed session, the stats poller and the vote watcher, if not running.
fn start(app: &AppHandle) {
    let st = state(app);
    // Held while starting, so the session's welcome can't arrive before `Live` is stored.
    let mut live = lock(&st.live);
    if live.is_some() {
        return;
    }
    let Some(m) = manager(app) else { return };
    let session = activity::start(app, &m.account_id);
    live::start(app, &m.account_id);
    let (stop, stop_rx) = tokio::sync::watch::channel(false);
    stats::spawn(app, stop_rx);
    // The broadcaster's own poll and prediction, kept live by the chat's vote watcher.
    crate::chat::votes::watch(app, &m.login, Some(m.account_id.clone()));
    *live = Some(Live {
        account: m.account_id.clone(),
        channel_id: m.account_id,
        login: m.login,
        session,
        session_id: None,
        stop,
    });
}

/// Ends the feed (dropping the session closes its socket) and the poller.
fn stop(app: &AppHandle) {
    let live = lock(&state(app).live).take();
    if let Some(live) = live {
        let _ = live.stop.send(true);
        crate::chat::votes::unwatch(app, &live.login);
        self::live::stop(app, &live.channel_id);
        if live.session.is_some() {
            drop(live.session);
            // Its `channel.follow` is gone with it; the mod feed takes follows back.
            crate::moderation::feeds::follow_released(app, &live.channel_id);
        } else {
            activity::stop_gql(app, &live.account);
        }
    }
}

/// The EventSub feed, when the manager runs on Helix: (account, channel id, session id).
pub(crate) fn eventsub_session(app: &AppHandle) -> Option<(String, String, String)> {
    let st = state(app);
    let live = lock(&st.live);
    let l = live.as_ref()?;
    Some((l.account.clone(), l.channel_id.clone(), l.session_id.clone()?))
}

/// The mod feed's session for a channel closed: the manager's EventSub feed takes over
/// its follows if it watches that channel (D30).
pub(crate) fn follow_released(app: &AppHandle, channel_id: &str) {
    if let Some((account, cid, session_id)) = eventsub_session(app)
        && cid == channel_id
    {
        follows::take_over(app, &account, &cid, &session_id);
    }
}

/// A dashboard topic message from Hermes (GQL feed, D30), or a live one (D7, D2).
pub fn on_hermes(app: &AppHandle, e: &cyex_twitch::hermes::Event) {
    use cyex_twitch::hermes::Kind;
    match e.topic.kind {
        Kind::Playback | Kind::Raid | Kind::BroadcastSettings => live::on_event(app, e),
        _ => gql_feed::on_event(app, e),
    }
}

/// Appends an activity item and tells open views. One the feed has already is skipped.
pub(crate) fn push_activity(app: &AppHandle, item: activity::Item) {
    let st = state(app);
    {
        let mut buf = lock(&st.activity);
        if buf.iter().any(|i| i.id == item.id) {
            return;
        }
        buf.push_back(item.clone());
        while buf.len() > MAX_ACTIVITY {
            buf.pop_front();
        }
    }
    let _ = tauri::Emitter::emit(app, "stream:activity", item);
}

/// D1 Older items from the feed's history: put in by time, and views get the whole feed.
pub(crate) fn merge_activity(app: &AppHandle, items: Vec<activity::Item>) {
    let all: Vec<activity::Item> = {
        let st = state(app);
        let mut buf = lock(&st.activity);
        let mut all: Vec<activity::Item> = buf.drain(..).collect();
        for it in items {
            if !all.iter().any(|i| i.id == it.id) {
                all.push(it);
            }
        }
        all.sort_by_key(|i| i.ts);
        let skip = all.len().saturating_sub(MAX_ACTIVITY);
        buf.extend(all.drain(skip..));
        buf.iter().cloned().collect()
    };
    let _ = tauri::Emitter::emit(app, "stream:activity-all", all);
}

/// D4 Where an alert is now in Twitch's alert box.
pub(crate) fn set_status(app: &AppHandle, alert: &str, status: &str) {
    {
        let st = state(app);
        let mut buf = lock(&st.activity);
        let Some(item) = buf.iter_mut().find(|i| i.alert.as_deref() == Some(alert)) else {
            return;
        };
        item.status = Some(status.to_owned());
    }
    let _ = tauri::Emitter::emit(app, "stream:alert", (alert, status));
}

/// Records a viewer-count sample for the X4 graph.
pub(crate) fn push_viewers(app: &AppHandle, at: i64, viewers: u64) {
    let st = state(app);
    let mut series = lock(&st.series);
    series.push_back((at, viewers));
    while series.len() > MAX_SERIES {
        series.pop_front();
    }
}

/// The viewer graph so far: (unix seconds, viewers).
pub fn series(app: &AppHandle) -> Vec<(i64, u64)> {
    lock(&state(app).series).iter().copied().collect()
}

/// The account that runs the manager, for the UI.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmBroadcaster")]
pub struct Broadcaster {
    pub account_id: String,
    pub login: String,
    pub display: String,
    pub avatar: String,
}

/// The account that can run the stream manager: the default if it qualifies, else the
/// first that does. Needs a GQL sign-in, or Helix with `channel:manage:broadcast` (D30).
pub fn manager(app: &AppHandle) -> Option<Broadcaster> {
    let accounts = auth::accounts(app);
    let qualifies =
        |id: &str| auth::gql_token(app, id).is_some() || auth::helix_scoped(app, id, SCOPE).is_some();
    let account = accounts
        .default_account()
        .filter(|a| qualifies(&a.user_id))
        .or_else(|| accounts.list.iter().find(|a| qualifies(&a.user_id)))?;
    Some(Broadcaster {
        account_id: account.user_id.clone(),
        login: account.login.clone(),
        display: if account.display_name.is_empty() {
            account.login.clone()
        } else {
            account.display_name.clone()
        },
        avatar: account.avatar.clone(),
    })
}

/// The manager account id, or an error for a command to return.
pub fn account_id(app: &AppHandle) -> Result<String, String> {
    manager(app).map(|m| m.account_id).ok_or_else(|| {
        "The stream manager needs a Twitch sign-in for your channel (Settings → Accounts).".into()
    })
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<Auth>().http.clone()
}

/// The manager account's sign-ins and its channel (its own user id). Both broadcaster
/// and moderator ids are the account itself: it mods its own channel.
pub(crate) struct Call {
    pub http: cyex_twitch::Http,
    pub channel_id: String,
    gql: Option<(String, String)>,
    helix: Option<(String, String)>,
}

impl Call {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let id = account_id(app)?;
        let gql = auth::gql_token(app, &id);
        let helix = auth::helix_token(app, &id);
        if gql.is_none() && helix.is_none() {
            return Err("Sign in again in Settings → Accounts.".into());
        }
        Ok(Self {
            http: http(app),
            channel_id: id,
            gql,
            helix,
        })
    }

    pub fn has_gql(&self) -> bool {
        self.gql.is_some()
    }

    pub fn has_helix(&self) -> bool {
        self.helix.is_some()
    }

    /// The GQL sign-in (token and its client).
    pub fn gql(&self) -> gql::Auth<'_> {
        match &self.gql {
            Some((token, client_id)) => gql::Auth {
                client_id,
                token: Some(token),
            },
            None => gql::Auth::PUBLIC,
        }
    }

    pub fn helix(&self) -> helix::Auth<'_> {
        let (token, client_id) = self.helix.as_ref().map_or(("", ""), |(t, c)| (t.as_str(), c.as_str()));
        helix::Auth { client_id, token }
    }

    /// The broadcaster acting as their own moderator.
    pub fn room(&self) -> helix::Room<'_> {
        helix::Room {
            broadcaster_id: &self.channel_id,
            moderator_id: &self.channel_id,
        }
    }

    /// The same, inside a Stream Together session.
    pub fn in_session<'a>(&'a self, session_id: &'a str) -> helix::gueststar::InSession<'a> {
        helix::gueststar::InSession {
            broadcaster_id: &self.channel_id,
            moderator_id: &self.channel_id,
            session_id,
        }
    }
}

/// Runs an action GQL first, like Twitch's own dashboard, and over Helix when the account
/// has no GQL sign-in or GQL no longer knows the query (nothing ran then) (D30).
pub(crate) async fn either<T>(
    app: &AppHandle,
    on_gql: impl AsyncFnOnce(&Call) -> cyex_twitch::Result<T>,
    on_helix: impl AsyncFnOnce(&Call) -> cyex_twitch::Result<T>,
) -> Result<T, String> {
    let c = Call::new(app)?;
    if c.gql.is_some() {
        match on_gql(&c).await {
            Err(e) if e.is_schema_error() && c.helix.is_some() => {}
            other => return other.map_err(|e| words::gql(&e)),
        }
    }
    on_helix(&c).await.map_err(helix_error)
}

/// Twitch's error as a short line (reuses the moderation wording).
pub(crate) fn helix_error(e: cyex_twitch::Error) -> String {
    match &e {
        cyex_twitch::Error::Api { status, message } => crate::moderation::codes::helix(*status, message),
        _ => "Couldn't reach Twitch.".into(),
    }
}
