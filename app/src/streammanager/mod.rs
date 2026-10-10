//! 1.35, X4 The stream manager (spec 10) wired to Tauri. A fixed tab for the signed-in
//! account's own channel, Helix only (D26): it edits the title and category, runs ads,
//! raids, markers, polls, predictions and chat modes, hosts Stream Together, shows a live
//! activity feed (its own EventSub session, `activity`) and stats (`stats`), and keeps OBS
//! status files (`obs`). Actions are in `actions`; no write is run in testing.

pub mod actions;
pub mod activity;
pub mod obs;
pub mod stats;

use std::collections::VecDeque;
use std::sync::Mutex;

use cyex_twitch::helix;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::auth::{self, Auth};

/// Which account runs the manager, scope that gates it.
const SCOPE: &str = "channel:manage:broadcast";
const MAX_ACTIVITY: usize = 200;
/// ~6 hours at one point per 30 s.
const MAX_SERIES: usize = 720;

pub fn init(app: &AppHandle) {
    app.manage(State::default());
}

#[derive(Default)]
pub struct State {
    /// Newest activity last; capped at `MAX_ACTIVITY`.
    activity: Mutex<VecDeque<activity::Item>>,
    /// Viewer count over time for the X4 graph: (unix seconds, viewers).
    series: Mutex<VecDeque<(i64, u64)>>,
    /// The newest stats tick.
    stats: Mutex<Option<stats::Stats>>,
    /// The running feed and poller, while the tab is open. Ref-counted by `opens`.
    live: Mutex<Option<Live>>,
    opens: Mutex<u32>,
}

/// The feed session and stats poller that run while the manager tab is open.
pub struct Live {
    pub account: String,
    pub channel_id: String,
    login: String,
    _session: cyex_eventsub::session::Session,
    /// Set once the socket welcomes; cleared when it drops.
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
    let session = activity::start(app);
    let (stop, stop_rx) = tokio::sync::watch::channel(false);
    stats::spawn(app, m.account_id.clone(), stop_rx);
    // The broadcaster's own poll and prediction, kept live by the chat's vote watcher.
    crate::chat::votes::watch(app, &m.login, Some(m.account_id.clone()));
    *live = Some(Live {
        account: m.account_id.clone(),
        channel_id: m.account_id,
        login: m.login,
        _session: session,
        session_id: None,
        stop,
    });
}

/// Ends the session (dropping it closes the socket) and the poller.
fn stop(app: &AppHandle) {
    let live = lock(&state(app).live).take();
    if let Some(live) = live {
        let _ = live.stop.send(true);
        crate::chat::votes::unwatch(app, &live.login);
    }
}

/// Appends an activity item and tells open views.
pub(crate) fn push_activity(app: &AppHandle, item: activity::Item) {
    let st = state(app);
    {
        let mut buf = lock(&st.activity);
        buf.push_back(item.clone());
        while buf.len() > MAX_ACTIVITY {
            buf.pop_front();
        }
    }
    let _ = tauri::Emitter::emit(app, "stream:activity", item);
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
/// first that does. Needs a Helix sign-in with `channel:manage:broadcast` (D26).
pub fn manager(app: &AppHandle) -> Option<Broadcaster> {
    let accounts = auth::accounts(app);
    let qualifies = |id: &str| auth::helix_scoped(app, id, SCOPE).is_some();
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

/// Runs `f` with the manager account's Helix auth and its channel id (its own user id).
/// Both broadcaster and moderator ids are the account itself: it mods its own channel.
async fn with_helix<T, F, Fut>(app: &AppHandle, f: F) -> Result<T, String>
where
    F: FnOnce(cyex_twitch::Http, HelixCall) -> Fut,
    Fut: std::future::Future<Output = cyex_twitch::Result<T>>,
{
    let id = account_id(app)?;
    let (token, client_id) = auth::helix_token(app, &id).ok_or("Sign in again in Settings → Accounts.")?;
    let http = http(app);
    let call = HelixCall {
        token,
        client_id,
        channel_id: id,
    };
    f(http, call).await.map_err(helix_error)
}

/// Twitch's error as a short line (reuses the moderation wording).
pub(crate) fn helix_error(e: cyex_twitch::Error) -> String {
    match &e {
        cyex_twitch::Error::Api { status, message } => crate::moderation::codes::helix(*status, message),
        _ => "Couldn't reach Twitch.".into(),
    }
}

/// Owns the strings a Helix call borrows, so the auth can outlive the lookup.
struct HelixCall {
    token: String,
    client_id: String,
    channel_id: String,
}

impl HelixCall {
    fn auth(&self) -> helix::Auth<'_> {
        helix::Auth {
            client_id: &self.client_id,
            token: &self.token,
        }
    }

    /// The broadcaster acting as their own moderator.
    fn room(&self) -> helix::Room<'_> {
        helix::Room {
            broadcaster_id: &self.channel_id,
            moderator_id: &self.channel_id,
        }
    }

    /// The same, inside a Stream Together session.
    fn in_session<'a>(&'a self, session_id: &'a str) -> helix::gueststar::InSession<'a> {
        helix::gueststar::InSession {
            broadcaster_id: &self.channel_id,
            moderator_id: &self.channel_id,
            session_id,
        }
    }
}
