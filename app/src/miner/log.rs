//! W4 The miner log: chests, moments, raids joined, bets and their results, drops and
//! goals. The last 200 lines, kept while the app runs.

use std::collections::VecDeque;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::Miner;

const EVENT: &str = "miner:log";
const MAX: usize = 200;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LogLine {
    /// Unix ms.
    #[ts(type = "number")]
    pub t: i64,
    /// The channel's display name.
    pub channel: Option<String>,
    pub text: String,
}

#[derive(Default)]
pub struct Log(Mutex<VecDeque<LogLine>>);

pub fn add(app: &AppHandle, channel: Option<&str>, text: impl Into<String>) {
    let line = LogLine {
        t: super::now_ms(),
        channel: channel.map(str::to_owned),
        text: text.into(),
    };
    let miner = app.state::<Miner>();
    let mut log = miner.log.0.lock().unwrap_or_else(|p| p.into_inner());
    log.push_back(line.clone());
    while log.len() > MAX {
        log.pop_front();
    }
    let _ = app.emit(EVENT, line);
}

/// Oldest first.
pub fn lines(app: &AppHandle) -> Vec<LogLine> {
    let miner = app.state::<Miner>();
    let log = miner.log.0.lock().unwrap_or_else(|p| p.into_inner());
    log.iter().cloned().collect()
}
