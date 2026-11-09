//! The points miner (08: 1.30-1.34, 1.44, C22, W1-W4), running whenever the app runs
//! (D24). `channels.rs` keeps the list and who's live, `watch.rs` sends watch time for
//! the two slots, `realtime.rs` reads Hermes, `claims.rs` takes chests, moments and
//! raids, `bets.rs` bets, `drops.rs` claims drops, `goals.rs` feeds community goals,
//! `tabs.rs` covers watching in the app (W1) and chests in open tabs (C22), `stats.rs`
//! keeps the log per channel (1.32) and `log.rs` the miner log (W4). V110-U68: `campaigns.rs`
//! loads the badge campaigns, `badges.rs` finds who can earn them (and helper channels),
//! `idle.rs` slows everything down while the app is hidden with nothing to earn, and
//! `source.rs` asks Twitch (or, in mocks builds, `drops_mock.rs`) about drops.

mod badges;
mod bets;
mod campaigns;
mod channels;
mod claims;
pub mod drops;
#[cfg(feature = "mocks")]
mod drops_mock;
mod goals;
pub mod idle;
pub mod peek;
pub mod log;
mod realtime;
mod source;
pub mod stats;
mod tabs;
mod watch;

pub use realtime::{on_channel_event, on_user_event};
pub use tabs::chat_rooms;

use crate::auth::gql_auth; // V107-R18: miner submodules reach it via super::
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};

use cyex_miner::badges::{Badge, BadgeView, EarnedBadge, Helper};
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
    /// V110-U68 Wakes the watch, drops and campaign loops (`notify_waiters`): a window
    /// showed, a stream went up, settings changed.
    idle_wake: Arc<Notify>,
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
    /// spade's address, read from twitch.tv once it reads (a failure isn't kept).
    spade: Option<String>,
    /// V110-F55 When the last read of it failed: the fallback address serves until it is due again.
    spade_failed: Option<std::time::Instant>,
    /// V110-U68 When it was last read, so refused minutes read it again at most once a minute.
    spade_read_at: Option<std::time::Instant>,
    /// V110-U68 Badges that can be earned by watching (`campaigns.rs`, `drops.rs`).
    badges: Vec<Badge>,
    /// Channels found for a badge, kept in memory only; they also sit in `channels`.
    helpers: Vec<Helper>,
    /// Helpers added per campaign this session, and when each was last looked for (secs).
    helper_adds: HashMap<String, u32>,
    helper_tried: HashMap<String, i64>,
    /// Helpers whose stream couldn't be reached; not tried again this session.
    helper_skip: HashSet<String>,
    /// Badges earned while the app ran, newest last.
    earned: Vec<EarnedBadge>,
    /// The badge being earned now: (channel id, campaign id).
    focus: Option<(String, String)>,
    /// Told once per focus that a stream playing here takes the other slot.
    focus_warned: bool,
    /// Campaigns whose drops are all claimed: a channel with only these has no drops left.
    done_campaigns: HashSet<String>,
    /// Offline channels whose stream topic is followed while hidden and idle.
    idle_topics: HashSet<String>,
    /// Followed channels that were followed as helpers (fewer topics, `watch::topics`).
    helper_topics: HashSet<String>,
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
    /// Drop campaigns running here that still have something to earn.
    drops: bool,
    /// V110-U68 Ids of the drop campaigns running here.
    campaigns: Vec<String>,
    /// V110-U68 Found for this badge campaign, not on the list.
    helper_for: Option<String>,
    /// V110-U68 The watch streak count when this stream began, and when it was last read
    /// (ms): a higher count means this stream's streak is in.
    streak_start: Option<u32>,
    streak_checked_at: i64,
    /// Minutes in a row that couldn't reach the stream.
    fails: u32,
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
            campaigns: Vec::new(),
            helper_for: None,
            streak_start: None,
            streak_checked_at: 0,
            fails: 0,
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
    /// V110-U68 Badges that can be earned by watching, one per campaign.
    pub badges: Vec<BadgeView>,
    /// V110-U68 The campaign whose badge is being earned now.
    pub focus: Option<String>,
    /// V110-U68 Badges earned while the app ran.
    pub earned: Vec<EarnedBadge>,
    /// V110-U68 When a waiting badge is looked for again (unix seconds): nothing live
    /// can earn it now.
    #[ts(type = "number | null")]
    pub retry_at: Option<i64>,
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
    /// V110-U68 Watched only for this badge campaign; not on the list.
    #[ts(optional = nullable)]
    pub helper_for: Option<String>,
}

pub fn init(app: &AppHandle) {
    app.manage(Miner {
        state: Mutex::default(),
        wake: Arc::new(Notify::new()),
        idle_wake: Arc::new(Notify::new()),
        session_start: now_ms(),
        log: log::Log::default(),
        last: Mutex::default(),
    });
    channels::spawn(app);
    watch::spawn(app);
    drops::spawn(app);
    campaigns::spawn(app);
    tabs::spawn(app);
}

/// Settings, accounts or tabs changed.
pub fn on_settings(app: &AppHandle) {
    if let Some(m) = app.try_state::<Miner>() {
        m.wake.notify_one();
        m.idle_wake.notify_waiters();
    }
    tabs::on_changed(app);
}

/// A Hermes connection came back after a drop: the channel list reads again now, and the
/// account's tabs look for a bonus chest that came meanwhile.
pub fn resync(app: &AppHandle, account: &str) {
    channels::nudge(app);
    tabs::resync(app, account);
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
    let fallback = || cyex_twitch::spade::URL.to_owned();
    if !retry_due(lock(app).spade_failed.map(|t| t.elapsed())) {
        return fallback();
    }
    match cyex_twitch::spade::find_url(&http(app)).await {
        Some(url) => {
            let mut st = lock(app);
            st.spade = Some(url.clone());
            st.spade_read_at = Some(std::time::Instant::now());
            url
        }
        None => {
            lock(app).spade_failed = Some(std::time::Instant::now());
            fallback()
        }
    }
}

/// V110-U68 spade turned a minute down: its address is read again for the next minute (at
/// most once a minute), as the cyex-miner reference does.
fn forget_spade(app: &AppHandle) {
    let mut st = lock(app);
    if reread_due(st.spade_read_at.map(|t| t.elapsed())) {
        st.spade = None;
        st.spade_failed = None;
        st.spade_read_at = None;
    }
}

/// V110-U68 The least time between two reads of spade's address after refused minutes.
const SPADE_REREAD: std::time::Duration = std::time::Duration::from_secs(60);

fn reread_due(since_read: Option<std::time::Duration>) -> bool {
    since_read.is_none_or(|d| d >= SPADE_REREAD)
}

/// V110-F58 Most ids the raids and bets sets remember.
const ONCE_CAP: usize = 500;

/// True the first time `id` is seen. The set is cleared when it passes [`ONCE_CAP`]: an id
/// that old belongs to a raid or prediction that is long over.
fn first_time(set: &mut HashSet<String>, id: &str) -> bool {
    if set.len() >= ONCE_CAP {
        set.clear();
    }
    set.insert(id.to_owned())
}

/// V110-F55 A failed read of spade's address is tried again after this long.
const SPADE_RETRY: std::time::Duration = std::time::Duration::from_secs(5 * 60);

fn retry_due(since_failure: Option<std::time::Duration>) -> bool {
    since_failure.is_none_or(|d| d >= SPADE_RETRY)
}

fn http(app: &AppHandle) -> cyex_twitch::Http {
    app.state::<crate::auth::Auth>().http.clone()
}

/// The status now, for a window that just opened.
pub fn status(app: &AppHandle) -> MinerStatus {
    let s = settings(app);
    let first: HashSet<&str> = s.first.iter().map(|c| c.id.as_str()).collect();
    let now_secs = now_ms() / 1000;
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
                helper_for: c.helper_for.clone(),
            })
            .collect(),
        drops: st.drop_progress.clone(),
        badges: cyex_miner::badges::views(&st.badges, &st.helpers, &s, now_secs),
        focus: st.focus.as_ref().map(|f| f.1.clone()),
        earned: st.earned.clone(),
        retry_at: badges::retry_at(&st, &s, now_secs),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn v110_f58_ids_count_once_and_the_set_stays_small() {
        let mut set = HashSet::new();
        assert!(first_time(&mut set, "a"));
        assert!(!first_time(&mut set, "a"));
        for i in 0..2 * ONCE_CAP {
            first_time(&mut set, &i.to_string());
        }
        assert!(set.len() <= ONCE_CAP);
    }

    #[test]
    fn v110_u68_refused_minutes_read_spade_again_at_most_once_a_minute() {
        assert!(reread_due(None));
        assert!(!reread_due(Some(Duration::from_secs(5))));
        assert!(reread_due(Some(SPADE_REREAD)));
    }

    #[test]
    fn v110_f55_a_failed_spade_read_is_tried_again_later() {
        assert!(retry_due(None));
        assert!(!retry_due(Some(Duration::from_secs(30))));
        assert!(retry_due(Some(SPADE_RETRY)));
    }
}
