//! 1.35 Live numbers while the tab is open: stream (viewers, uptime), title and category,
//! follower total, subscriber total and sub points. GQL first, Helix otherwise (D30). D7
//! Viewers, title and category come live from Hermes (`live`); the full read runs every
//! 2 minutes for followers and subs, which have no live updates. Each tick feeds the X4
//! viewer graph and the OBS status files. A failed read keeps the last value.

use std::time::Duration;

use cyex_twitch::gql::dashboard::stats::{Numbers, numbers_now};
use cyex_twitch::helix;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;
use ts_rs::TS;

use super::{Call, State, lock, push_viewers};
use crate::auth;

const EVERY: Duration = Duration::from_secs(120);
const EVENT: &str = "stream:stats";

/// What the stats widget and the OBS files show.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "SmStats")]
pub struct Stats {
    pub live: bool,
    pub title: String,
    pub game: String,
    #[ts(type = "number")]
    pub viewers: u64,
    /// RFC 3339; empty while offline. The UI counts the uptime from it.
    pub started_at: String,
    #[ts(type = "number | null")]
    pub followers: Option<u64>,
    #[ts(type = "number | null")]
    pub subs: Option<u64>,
    #[ts(type = "number | null")]
    pub sub_points: Option<u64>,
}

/// The newest numbers, for a view that just opened.
pub fn latest(app: &AppHandle) -> Option<Stats> {
    lock(&app.state::<State>().stats).clone()
}

pub fn spawn(app: &AppHandle, mut stop: watch::Receiver<bool>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            if *stop.borrow() {
                return;
            }
            tick(&app).await;
            tokio::select! {
                _ = tokio::time::sleep(EVERY) => {}
                _ = stop.changed() => return,
            }
        }
    });
}

/// D7 A live change: applied to the latest numbers, then the OBS files and the views.
pub(crate) fn pushed(app: &AppHandle, change: impl FnOnce(&mut Stats)) {
    let s = {
        let st = app.state::<State>();
        let mut cur = lock(&st.stats);
        let mut s = cur.clone().unwrap_or_default();
        change(&mut s);
        if cur.as_ref() == Some(&s) {
            return;
        }
        *cur = Some(s.clone());
        s
    };
    super::obs::write(app, &s);
    let _ = app.emit(EVENT, s);
}

/// Reads again now, e.g. right after the title changed.
pub fn refresh(app: &AppHandle) {
    if lock(&app.state::<State>().live).is_none() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move { tick(&app).await });
}

async fn tick(app: &AppHandle) {
    let Ok(c) = Call::new(app) else { return };
    let mut s = latest(app).unwrap_or_default();
    let gql = if c.has_gql() {
        Some(numbers_now(&c.http, c.gql(), &c.channel_id).await)
    } else {
        None
    };
    match gql {
        Some(Ok(n)) => apply(&mut s, n),
        Some(Err(e)) if !(e.is_schema_error() && c.has_helix()) => {}
        // No GQL sign-in, or GQL no longer knows the query.
        _ => helix_tick(&c, &mut s).await,
    }
    if s.live {
        push_viewers(app, auth::unix_now(), s.viewers);
    }
    *lock(&app.state::<State>().stats) = Some(s.clone());
    super::obs::write(app, &s);
    let _ = app.emit(EVENT, s);
}

/// GQL's numbers into the stats. A missing sub read keeps the last one.
fn apply(s: &mut Stats, n: Numbers) {
    s.live = n.live;
    s.viewers = n.viewers;
    s.started_at = n.started_at;
    s.title = n.title;
    s.game = n.game;
    if n.followers.is_some() {
        s.followers = n.followers;
    }
    if let Some((subs, points)) = n.subs {
        s.subs = Some(subs);
        s.sub_points = Some(points);
    }
}

async fn helix_tick(c: &Call, s: &mut Stats) {
    let (http, auth, id) = (&c.http, c.helix(), c.channel_id.as_str());
    let (stream, info, followers, subs) = tokio::join!(
        helix::stats::stream(http, auth, id),
        helix::channel::info(http, auth, id),
        helix::stats::followers_total(http, auth, id),
        helix::stats::subscriptions(http, auth, id),
    );
    match stream {
        Ok(Some(st)) => {
            s.live = true;
            s.viewers = st.viewers;
            s.started_at = st.started_at;
            s.title = st.title;
            s.game = st.game_name;
        }
        Ok(None) => {
            s.live = false;
            s.viewers = 0;
            s.started_at.clear();
        }
        Err(_) => {}
    }
    // The channel's own title and category win offline, and right after an edit.
    if let Ok(i) = info {
        s.title = i.title;
        s.game = i.game_name;
    }
    if let Ok(n) = followers {
        s.followers = Some(n);
    }
    if let Ok(sub) = subs {
        s.subs = Some(sub.total);
        s.sub_points = Some(sub.points);
    }
}
