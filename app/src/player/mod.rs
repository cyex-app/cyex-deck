//! Playback (1.1, 1.2) wired to Tauri: one session per player, its playlists served to
//! hls.js over `cyexhls://` (`protocol.rs`). Sessions nobody asked about for a while are
//! dropped, so a closed window can't leave one behind.

pub mod blank;
pub mod clips;
mod device;
#[cfg(feature = "mocks")]
pub mod mock;
pub mod positions;
mod protocol;
pub mod proxy;

pub use protocol::{SCHEME, handle};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use cyex_player::fetch::{SignIn, Twitch};
use cyex_player::live::{AdState, Live, Options};
use cyex_player::vod::Vod;
use cyex_settings::Partition;
use cyex_settings::schema::{Player, random_id};
use cyex_twitch::Http;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::auth::{Auth, gql_token};
use crate::settings::Settings;

const AD_EVENT: &str = "player:ads";
/// hls.js polls a live playlist every 2 s; a paused VOD asks for nothing.
const LIVE_IDLE: Duration = Duration::from_secs(60);
const VOD_IDLE: Duration = Duration::from_secs(4 * 3600);

pub struct Players {
    http: Http,
    device_id: String,
    /// P10 The ad-block proxy's client, when one is set.
    proxy: Mutex<Option<Http>>,
    sessions: Mutex<HashMap<String, Session>>,
    pub positions: positions::Positions,
}

#[derive(Clone)]
pub enum Kind {
    Live(Arc<Live<Twitch>>),
    Vod(Arc<Vod<Twitch>>),
}

struct Session {
    kind: Kind,
    /// The window playing it: ad breaks are told there.
    window: String,
    used: Instant,
    ad: AdState,
    /// W1 A live stream's channel and the account watching it.
    live: Option<(String, Option<String>)>,
}

/// What a player loads: `url` is the master playlist for hls.js, `audio` the one with
/// only the sound (P1).
#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlayerSession {
    pub id: String,
    pub url: String,
    pub audio: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdChanged<'a> {
    id: &'a str,
    state: AdState,
}

pub fn init(app: &AppHandle) {
    let dir = crate::paths::config(app).ok();
    app.manage(Players {
        http: app.state::<Auth>().http.clone(),
        device_id: device::id(app),
        proxy: Mutex::new(proxy::load().and_then(|u| proxy::client(&u))),
        sessions: Mutex::default(),
        positions: positions::Positions::load(dir.map(|d| d.join("vod-positions.json"))),
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let now = Instant::now();
            lock(&app).retain(|_, s| {
                let idle = match s.kind {
                    Kind::Live(_) => LIVE_IDLE,
                    Kind::Vod(_) => VOD_IDLE,
                };
                now.duration_since(s.used) < idle
            });
        }
    });
}

fn players(app: &AppHandle) -> tauri::State<'_, Players> {
    app.state::<Players>()
}

fn lock(app: &AppHandle) -> MutexGuard<'_, HashMap<String, Session>> {
    app.state::<Players>()
        .inner()
        .sessions
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// D19 Plays with `account`'s GQL sign-in when it has one, read fresh on each use.
fn fetch(app: &AppHandle, account: Option<&str>) -> Twitch {
    let p = players(app);
    let sign_in: Option<SignIn> = account.map(|id| {
        let (app, id) = (app.clone(), id.to_owned());
        Arc::new(move || gql_token(&app, &id)) as SignIn
    });
    Twitch {
        http: p.http.clone(),
        proxy: p.proxy.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        device_id: p.device_id.clone(),
        sign_in,
        #[cfg(feature = "mocks")]
        mock: mock::base(app),
    }
}

/// A live stream for a player in `window`, with the ad settings of the moment.
pub fn open_live(app: &AppHandle, window: &str, login: &str, account: Option<&str>) -> PlayerSession {
    let s: Player = app.state::<Settings>().read(Partition::Player);
    let options = Options {
        low_latency: s.low_latency,
        block_ads: s.block_ads,
    };
    let login = login.to_ascii_lowercase();
    let live = Live::new(fetch(app, account), &login, options);
    let who = Some((login, account.map(str::to_owned)));
    open(app, window, Kind::Live(Arc::new(live)), "live", who)
}

pub fn open_vod(app: &AppHandle, window: &str, id: &str, account: Option<&str>) -> PlayerSession {
    let vod = Vod::new(fetch(app, account), id);
    open(app, window, Kind::Vod(Arc::new(vod)), "vod", None)
}

/// W1 Live streams playing now (hls.js asked for their playlist in the last 15 s), as
/// (login, account), once each.
pub fn playing_live(app: &AppHandle) -> Vec<(String, Option<String>)> {
    let now = Instant::now();
    let mut out: Vec<(String, Option<String>)> = lock(app)
        .values()
        .filter(|s| now.duration_since(s.used) < Duration::from_secs(15))
        .filter_map(|s| s.live.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// V103-07 A live player of `login` plays now, in any window, as any account.
pub fn plays(app: &AppHandle, login: &str) -> bool {
    let login = login.to_ascii_lowercase();
    playing_live(app).iter().any(|(l, _)| *l == login)
}

fn open(app: &AppHandle, window: &str, kind: Kind, path: &str, live: Option<(String, Option<String>)>) -> PlayerSession {
    let id = random_id();
    lock(app).insert(
        id.clone(),
        Session {
            kind,
            window: window.to_owned(),
            used: Instant::now(),
            ad: AdState::None,
            live,
        },
    );
    let base = format!("{}{path}/{id}/", protocol::base());
    PlayerSession {
        url: format!("{base}master.m3u8"),
        audio: format!("{base}audio.m3u8"),
        id,
    }
}

pub fn close(app: &AppHandle, id: &str) {
    lock(app).remove(id);
}

/// P10 Sets or clears the proxy. New players use it.
pub fn set_proxy(app: &AppHandle, url: Option<&str>) -> Result<(), String> {
    let client = proxy::save(url)?;
    *players(app).proxy.lock().unwrap_or_else(|e| e.into_inner()) = client;
    Ok(())
}

/// The session for a request, marked as used.
pub(crate) fn session(app: &AppHandle, id: &str) -> Option<Kind> {
    let mut sessions = lock(app);
    let s = sessions.get_mut(id)?;
    s.used = Instant::now();
    Some(s.kind.clone())
}

/// Tells the player's window when a break starts or ends.
pub(crate) fn set_ad(app: &AppHandle, id: &str, state: AdState) {
    let window = {
        let mut sessions = lock(app);
        let Some(s) = sessions.get_mut(id).filter(|s| s.ad != state) else { return };
        s.ad = state;
        s.window.clone()
    };
    let _ = app.emit_to(window.as_str(), AD_EVENT, AdChanged { id, state });
}
