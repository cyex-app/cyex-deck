//! Live dots (1.29) and renamed channels (1.15). Every tab channel is looked up by id
//! once a minute, and right away when a tab brings a new one.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::{Resolved, Tabs};
use cyex_twitch::gql::{self, ChannelInfo};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::auth::Auth;
use crate::settings::Settings;

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

/// Polls now if a tab has a channel the last poll didn't ask about.
pub fn nudge(app: &AppHandle) {
    let Some(live) = app.try_state::<Live>() else { return };
    let tabs: Tabs = app.state::<Settings>().read(Partition::Tabs);
    let asked = live.asked.lock().unwrap_or_else(|p| p.into_inner());
    if tabs.channel_ids().iter().any(|id| !asked.contains(id)) {
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
    let ids = app.state::<Settings>().read::<Tabs>(Partition::Tabs).channel_ids();
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

    let resolved: Vec<Resolved> = channels
        .iter()
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

    let changed = {
        let mut last = live.channels.lock().unwrap_or_else(|p| p.into_inner());
        // S8 Channels whose login or display name changed since the last look.
        let renamed: Vec<(String, String, String)> = channels
            .iter()
            .filter_map(|c| {
                let old = last.iter().find(|o| o.id == c.id)?;
                (old.login != c.login || old.name != c.name).then(|| (c.login.clone(), old.login.clone(), c.name.clone()))
            })
            .collect();
        crate::plugins::events::renamed(app, renamed);
        let changed = *last != channels;
        *last = channels.clone();
        changed
    };
    if changed {
        let _ = app.emit(EVENT, channels);
    }
}
