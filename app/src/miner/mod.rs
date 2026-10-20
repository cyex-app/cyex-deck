//! The points miner (08: 1.30-1.34, 1.44, C22, W1-W4), running whenever the app runs
//! (D24). `channels.rs` keeps the list and who's live, `watch.rs` sends watch time for
//! the two slots, `realtime.rs` reads Hermes, `claims.rs` takes chests, moments and
//! raids, `bets.rs` bets, `drops.rs` claims drops, `goals.rs` feeds community goals,
//! `tabs.rs` covers watching in the app (W1) and chests in open tabs (C22), `stats.rs`
//! keeps the log per channel (1.32) and `log.rs` the miner log (W4).

mod bets;
mod channels;
mod claims;
pub mod drops;
mod goals;
pub mod peek;
pub mod log;
mod realtime;
pub mod stats;
mod tabs;
mod watch;

pub use realtime::{on_channel_event, on_user_event};
pub use tabs::chat_rooms;

use crate::auth::gql_auth; // V107-R18: miner submodules reach it via super::
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use cyex_miner::slots::Streak;
use cyex_settings::Partition;
use cyex_settings::schema::Miner as MinerSettings;
use cyex_twitch::gql::miner::MinerStream;
use cyex_twitch::gql::AccessToken;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;
use ts_rs::TS;

use crate::settings::Settings;

const STATUS_EVENT: &str = "miner:status";

pub struct Miner {
    state: Mutex<State>,
    /// Wakes the channel loop: settings changed, a stream went up.
    wake: Arc<Notify>,
    /// When this run started (ms), for "this session" in the stats.
    pub session_start: i64,
    log: log::Log,
    last: Mutex<Option<MinerStatus>>,
}

#[derive(Default)]
struct State {
    /// The account mining, once it has a GQL sign-in.
    account: Option<String>,
    problem: Option<String>,
    /// In priority order.
    channels: Vec<Chan>,
    /// 1.33 Getting watch time now.
    slots: Vec<String>,
    /// W1 Live channels the mining account watches in the app.
    watching: Vec<String>,
    /// Channels whose Hermes topics are followed: the slots and `watching`.
    followed: HashSet<String>,
    /// Raid ids joined and prediction ids bet on, so each happens once.
    raids: HashSet<String>,
    bets: HashSet<String>,
    drop_progress: Vec<drops::DropProgress>,
    /// spade's address, read from twitch.tv once per run.
    spade: Option<String>,
}

struct Chan {
    id: String,
    login: String,
    name: String,
    avatar: Option<String>,
    stream: Option<MinerStream>,
    /// When it counted as live (ms); 0 offline.
    live_since: i64,
    streak: Streak,
    balance: Option<u64>,
    multiplier: f64,
    /// Drop campaigns running here.
    drops: bool,
    /// When the points context and the drops were last read (ms).
    context_at: i64,
    drops_at: i64,
    /// Playback token and when it was issued (ms).
    token: Option<(AccessToken, i64)>,
    /// Last minute spade took (ms).
    watched_at: i64,
    /// Seconds watched not yet written to the stats.
    unlogged_secs: u64,
}

impl Chan {
    fn new(id: &str, login: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            login: login.to_owned(),
            name: name.to_owned(),
            avatar: None,
            stream: None,
            live_since: 0,
            streak: Streak::default(),
            balance: None,
            multiplier: 0.0,
            drops: false,
            context_at: 0,
            drops_at: 0,
            token: None,
            watched_at: 0,
            unlogged_secs: 0,
        }
    }
}

/// What the miner view shows (1.31, 1.33).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MinerStatus {
    /// Switched on with an account that can mine.
    pub running: bool,
    pub account: Option<String>,
    /// Why it isn't mining, in words.
    pub problem: Option<String>,
    pub channels: Vec<MinedChannel>,
    /// Drops still being earned.
    pub drops: Vec<drops::DropProgress>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MinedChannel {
    pub id: String,
    pub login: String,
    pub name: String,
    pub avatar: Option<String>,
    pub live: bool,
    /// 1.33 Getting watch time from the miner now.
    pub mining: bool,
    /// W1 Watched in the app, which earns on its own.
    pub watching: bool,
    /// 1.44
    pub first: bool,
    #[ts(type = "number | null")]
    pub balance: Option<u64>,
    /// A watch streak is still to be had this stream.
    pub streak: bool,
    pub drops: bool,
    pub game: Option<String>,
}

pub fn init(app: &AppHandle) {
    app.manage(Miner {
        state: Mutex::default(),
        wake: Arc::new(Notify::new()),
        session_start: now_ms(),
        log: log::Log::default(),
        last: Mutex::default(),
    });
    channels::spawn(app);
    watch::spawn(app);
    drops::spawn(app);
    tabs::spawn(app);
}

/// Settings, accounts or tabs changed.
pub fn on_settings(app: &AppHandle) {
    if let Some(m) = app.try_state::<Miner>() {
        m.wake.notify_one();
    }
    tabs::on_changed(app);
}

fn settings(app: &AppHandle) -> MinerSettings {
    app.state::<Settings>().read(Partition::Miner)
}

fn lock(app: &AppHandle) -> MutexGuard<'_, State> {
    app.state::<Miner>()
        .inner()
        .state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// The mining account and its GQL token, when it's running.
fn signed_in(app: &AppHandle) -> Option<(String, String, String)> {
    let account = lock(app).account.clone()?;
    let (token, client_id) = crate::auth::gql_token(app, &account)?;
    Some((account, token, client_id))
}

async fn spade_url(app: &AppHandle) -> String {
    if let Some(url) = lock(app).spade.clone() {
        return url;
    }
    let url = cyex_twitch::spade::find_url(&http(app)).await;
    lock(app).spade = Some(url.clone());
    url
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<crate::auth::Auth>().http.clone()
}

/// The status now, for a window that just opened.
pub fn status(app: &AppHandle) -> MinerStatus {
    let s = settings(app);
    let first: HashSet<&str> = s.first.iter().map(|c| c.id.as_str()).collect();
    let st = lock(app);
    MinerStatus {
        running: st.account.is_some(),
        account: st.account.clone(),
        problem: st.problem.clone(),
        channels: st
            .channels
            .iter()
            .map(|c| MinedChannel {
                id: c.id.clone(),
                login: c.login.clone(),
                name: c.name.clone(),
                avatar: c.avatar.clone(),
                live: c.stream.is_some(),
                mining: st.slots.contains(&c.id),
                watching: st.watching.contains(&c.id),
                first: first.contains(c.id.as_str()),
                balance: c.balance,
                streak: c.stream.is_some() && c.streak.due(now_ms()),
                drops: c.drops,
                game: c
                    .stream
                    .as_ref()
                    .and_then(|s| s.game.as_ref().map(|g| g.1.clone())),
            })
            .collect(),
        drops: st.drop_progress.clone(),
    }
}

/// Tells windows when anything they show changed.
fn emit_status(app: &AppHandle) {
    let status = status(app);
    let miner = app.state::<Miner>();
    let mut last = miner.last.lock().unwrap_or_else(|p| p.into_inner());
    if last.as_ref() != Some(&status) {
        let _ = app.emit(STATUS_EVENT, &status);
        *last = Some(status);
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// A channel's display name by id, for log lines.
fn name_of(app: &AppHandle, channel_id: &str) -> Option<(String, String)> {
    let st = lock(app);
    st.channels
        .iter()
        .find(|c| c.id == channel_id)
        .map(|c| (c.login.clone(), c.name.clone()))
}
