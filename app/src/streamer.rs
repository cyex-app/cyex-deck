//! U4 Streamer mode: on, off, or while a streaming app runs (looked for every 10 s). What
//! it hides is in `General::streamer`; windows hear about changes through
//! `streamer:changed`. The tray, a title-bar button or a global hotkey can flip it for now;
//! that lasts until the streaming app starts or stops, or the setting changes.

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use cyex_settings::Partition;
use cyex_settings::schema::{General, StreamerMode, StreamerWhen};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::settings::Settings;

const CHANGED_EVENT: &str = "streamer:changed";
const LOOK_EVERY: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct Streamer(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    /// A streaming app from the list is running.
    app_running: bool,
    /// Flipped by hand; cleared when the app state or the setting changes.
    forced: Option<bool>,
    when: Option<StreamerWhen>,
    active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StreamerState {
    pub active: bool,
    /// A streaming app is running (only looked for while set to Auto).
    pub app_running: bool,
    /// Flipped by hand for now.
    pub forced: bool,
}

fn lock(app: &AppHandle) -> MutexGuard<'_, Inner> {
    app.state::<Streamer>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

fn mode(app: &AppHandle) -> StreamerMode {
    app.state::<Settings>()
        .read::<General>(Partition::General)
        .streamer
}

pub fn init(app: &AppHandle) {
    app.manage(Streamer::default());
    on_settings(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let m = mode(&app);
            if m.when == StreamerWhen::Auto {
                let running = tauri::async_runtime::spawn_blocking(move || crate::sys::any_running(&m.apps))
                    .await
                    .unwrap_or(false);
                let changed = {
                    let mut inner = lock(&app);
                    let changed = inner.app_running != running;
                    if changed {
                        inner.app_running = running;
                        inner.forced = None;
                    }
                    changed
                };
                if changed {
                    update(&app);
                }
            }
            tokio::time::sleep(LOOK_EVERY).await;
        }
    });
}

/// The setting changed: a new "when" drops a manual flip.
pub fn on_settings(app: &AppHandle) {
    let when = mode(app).when;
    {
        let mut inner = lock(app);
        if inner.when != Some(when) {
            inner.when = Some(when);
            inner.forced = None;
            if when != StreamerWhen::Auto {
                inner.app_running = false;
            }
        }
    }
    update(app);
}

/// Turns streamer mode the other way until the streaming app starts or stops.
pub fn toggle(app: &AppHandle) {
    {
        let mut inner = lock(app);
        inner.forced = Some(!inner.active);
    }
    update(app);
}

pub fn state(app: &AppHandle) -> StreamerState {
    let inner = lock(app);
    StreamerState {
        active: inner.active,
        app_running: inner.app_running,
        forced: inner.forced.is_some(),
    }
}

/// Whether streamer mode is on and hides what `pick` names.
pub fn hides(app: &AppHandle, pick: impl Fn(&StreamerMode) -> bool) -> bool {
    app.try_state::<Streamer>().is_some() && lock(app).active && pick(&mode(app))
}

fn update(app: &AppHandle) {
    let state = {
        let mut inner = lock(app);
        let auto = match inner.when.unwrap_or_default() {
            StreamerWhen::Off => false,
            StreamerWhen::On => true,
            StreamerWhen::Auto => inner.app_running,
        };
        inner.active = inner.forced.unwrap_or(auto);
        StreamerState {
            active: inner.active,
            app_running: inner.app_running,
            forced: inner.forced.is_some(),
        }
    };
    let _ = app.emit(CHANGED_EVENT, state);
    crate::tray::set_streamer(app, state.active);
}
