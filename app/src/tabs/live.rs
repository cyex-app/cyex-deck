//! Live dots (1.29) and renamed channels (1.15). Every tab channel is looked up by id
//! once a minute, and right away when a tab brings a new one. Plugins hear of renames
//! (S8) and streams starting or ending (S12).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_plugins::LiveChange;
use cyex_settings::Partition;
use cyex_settings::schema::{Resolved, Tabs};
use cyex_twitch::gql::{self, ChannelInfo};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::auth::Auth;
use crate::settings::Settings;
use crate::tabs::viewers;

const POLL: Duration = Duration::from_secs(60);
const EVENT: &str = "tabs:live";

#[derive(Default)]
struct Live {
    channels: Mutex<Vec<ChannelInfo>>,
    /// Ids asked for in the last poll.
    asked: Mutex<HashSet<String>>,
    wake: Arc<Notify>,
}

pub fn spawn(app: &AppHandle) {
    app.manage(Live::default());
    app.manage(viewers::Viewers::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let wake = app.state::<Live>().wake.clone();
        loop {
            poll(&app).await;
            tokio::select! {
                () = tokio::time::sleep(POLL) => {}
                () = wake.notified() => {}
            }
        }
    });
}

/// V106-17 Every id to look up: the tab channels plus the default account's own channel (same
/// request, up to 100 ids), and the tab ids alone.
fn wanted(app: &AppHandle) -> (Vec<String>, HashSet<String>) {
    let mut all = app.state::<Settings>().read::<Tabs>(Partition::Tabs).channel_ids();
    let tab_ids: HashSet<String> = all.iter().cloned().collect();
    if let Some(own) = crate::auth::accounts(app).default_id
        && !tab_ids.contains(&own)
    {
        all.push(own);
    }
    (all, tab_ids)
}

/// Polls now if a tab has a channel the last poll didn't ask about.
pub fn nudge(app: &AppHandle) {
    let Some(live) = app.try_state::<Live>() else { return };
    let (ids, _) = wanted(app);
    let asked = live.asked.lock().unwrap_or_else(|p| p.into_inner());
    if ids.iter().any(|id| !asked.contains(id)) {
        live.wake.notify_one();
    }
}

/// The last poll's results, for a window that just opened.
pub fn snapshot(app: &AppHandle) -> Vec<ChannelInfo> {
    app.try_state::<Live>()
        .map(|l| l.channels.lock().unwrap_or_else(|p| p.into_inner()).clone())
        .unwrap_or_default()
}

/// F3 Whether a tab's channel was live at the last poll.
pub fn is_live(app: &AppHandle, login: &str) -> bool {
    app.try_state::<Live>().is_some_and(|l| {
        l.channels
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .any(|c| c.login == login && c.stream.is_some())
    })
}

async fn poll(app: &AppHandle) {
    let (ids, tab_ids) = wanted(app);
    let live = app.state::<Live>();
    *live.asked.lock().unwrap_or_else(|p| p.into_inner()) = ids.iter().cloned().collect();
    let channels = if ids.is_empty() {
        Vec::new()
    } else {
        let http = app.state::<Auth>().http.clone();
        match gql::channels_by_ids(&http, &ids).await {
            Ok(list) => list,
            Err(e) => {
                eprintln!("tabs: live check failed: {e}");
                return;
            }
        }
    };

    // V106-18 One point per tab channel per check; offline and left-out channels drop.
    if let Some(v) = app.try_state::<viewers::Viewers>() {
        viewers::record(
            &mut v.0.lock().unwrap_or_else(|p| p.into_inner()),
            &channels,
            &tab_ids,
            crate::auth::unix_now(),
        );
    }

    let resolved: Vec<Resolved> = channels
        .iter()
        .filter(|c| tab_ids.contains(&c.id))
        .map(|c| Resolved {
            id: c.id.clone(),
            login: c.login.clone(),
            name: c.name.clone(),
        })
        .collect();
    if let Err(e) = app
        .state::<Settings>()
        .update::<Tabs>(app, Partition::Tabs, |t| t.apply_resolved(&resolved))
    {
        eprintln!("tabs: couldn't save new channel names: {e}");
    }

    let (changed, feed_flips) = {
        let mut last = live.channels.lock().unwrap_or_else(|p| p.into_inner());
        // S8 Channels whose login or display name changed since the last look.
        let renamed: Vec<(String, String, String)> = channels
            .iter()
            .filter(|c| tab_ids.contains(&c.id))
            .filter_map(|c| {
                let old = last.iter().find(|o| o.id == c.id)?;
                (old.login != c.login || old.name != c.name).then(|| (c.login.clone(), old.login.clone(), c.name.clone()))
            })
            .collect();
        crate::plugins::events::renamed(app, renamed);
        // S12 Channels whose stream started or ended. Nothing on the first look.
        // V106-07 The same flips go to the Live notifications feed.
        let mut flips: Vec<LiveChange> = Vec::new();
        let mut feed_flips: Vec<crate::feeds::live::Flip> = Vec::new();
        for c in channels.iter().filter(|c| tab_ids.contains(&c.id)) {
            let Some(old) = last.iter().find(|o| o.id == c.id) else { continue };
            if old.stream.is_some() == c.stream.is_some() {
                continue;
            }
            let title = c.stream.as_ref().map(|s| s.title.clone());
            let game = c.stream.as_ref().and_then(|s| s.game.clone());
            feed_flips.push(crate::feeds::live::Flip {
                id: c.id.clone(),
                login: c.login.clone(),
                name: c.name.clone(),
                avatar: c.avatar.clone(),
                live: c.stream.is_some(),
                title: title.clone(),
                game: game.clone(),
                started: c.stream.as_ref().map(|s| s.started_at.clone()),
            });
            flips.push(LiveChange { channel: c.login.clone(), name: c.name.clone(), live: c.stream.is_some(), title, game });
        }
        crate::plugins::events::live(app, flips);
        let changed = *last != channels;
        *last = channels.clone();
        (changed, feed_flips)
    };
    for f in feed_flips {
        crate::feeds::live::flip(app, f);
    }
    if changed {
        let _ = app.emit(EVENT, channels);
    }
}
