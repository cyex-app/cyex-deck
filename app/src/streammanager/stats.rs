//! 1.35 Live numbers, polled every 30 s while the tab is open: stream (viewers, uptime),
//! title and category, follower total, subscriber total and sub points. Each tick feeds the
//! X4 viewer graph and the OBS status files. A failed read keeps the last value.

use std::time::Duration;

use cyex_twitch::helix;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;
use ts_rs::TS;

use super::{State, lock, push_viewers};
use crate::auth;

const EVERY: Duration = Duration::from_secs(30);
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

pub fn spawn(app: &AppHandle, account: String, mut stop: watch::Receiver<bool>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            if *stop.borrow() {
                return;
            }
            tick(&app, &account).await;
            tokio::select! {
                _ = tokio::time::sleep(EVERY) => {}
                _ = stop.changed() => return,
            }
        }
    });
}

/// Reads again now, e.g. right after the title changed.
pub fn refresh(app: &AppHandle) {
    let Some(account) = lock(&app.state::<State>().live)
        .as_ref()
        .map(|l| l.account.clone())
    else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move { tick(&app, &account).await });
}

async fn tick(app: &AppHandle, account: &str) {
    let Some((token, client_id)) = auth::helix_token(app, account) else {
        return;
    };
    let http = app.state::<auth::Auth>().http.clone();
    let auth = helix::Auth {
        client_id: &client_id,
        token: &token,
    };
    let (stream, info, followers, subs) = tokio::join!(
        helix::stats::stream(&http, auth, account),
        helix::channel::info(&http, auth, account),
        helix::stats::followers_total(&http, auth, account),
        helix::stats::subscriptions(&http, auth, account),
    );
    let mut s = latest(app).unwrap_or_default();
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
    if s.live {
        push_viewers(app, auth::unix_now(), s.viewers);
    }
    *lock(&app.state::<State>().stats) = Some(s.clone());
    super::obs::write(app, &s);
    let _ = app.emit(EVENT, s);
}
