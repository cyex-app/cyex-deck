//! W4 The miner log (V106-27): points, chests, moments, raids, bets, drops, goals,
//! channels joined, left, live or offline, and errors in plain words. The last 1.000
//! lines, kept in memory while the app runs, never saved.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::Miner;
use crate::feeds::ring::Ring as Lines;

const EVENT: &str = "miner:log";
const CLEARED: &str = "miner:log:cleared";
const MAX: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LogKind {
    Points,
    Claim,
    Raid,
    Bet,
    Drop,
    Channel,
    Error,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LogLine {
    /// Rises with every line; the list's key.
    #[ts(type = "number")]
    pub id: u64,
    /// Unix ms.
    #[ts(type = "number")]
    pub t: i64,
    pub kind: LogKind,
    /// The channel's display name.
    pub channel: Option<String>,
    pub text: String,
}

#[derive(Default)]
pub struct Log(Mutex<Ring>);

/// R18 On the feeds' capped list.
struct Ring {
    lines: Lines<LogLine>,
    next: u64,
    /// The last error per channel; the same one again is not logged.
    last_err: std::collections::HashMap<Option<String>, String>,
}

impl Default for Ring {
    fn default() -> Self {
        Self {
            lines: Lines::new(MAX),
            next: 0,
            last_err: std::collections::HashMap::new(),
        }
    }
}

impl Ring {
    fn clear(&mut self) {
        self.lines.clear();
        self.last_err.clear();
    }

    fn push(&mut self, t: i64, kind: LogKind, channel: Option<&str>, text: String) -> Option<LogLine> {
        // The same error again and again (a minute loop) is one line, per channel.
        let key = channel.map(str::to_owned);
        if kind == LogKind::Error {
            if self.last_err.get(&key) == Some(&text) {
                return None;
            }
            self.last_err.insert(key, text.clone());
        } else {
            self.last_err.remove(&key);
            if channel.is_none() {
                self.last_err.clear();
            }
        }
        self.next += 1;
        let line = LogLine {
            id: self.next,
            t,
            kind,
            channel: channel.map(str::to_owned),
            text,
        };
        self.lines.push(line.clone(), |_| false);
        Some(line)
    }
}

pub fn add(app: &AppHandle, kind: LogKind, channel: Option<&str>, text: impl Into<String>) {
    let miner = app.state::<Miner>();
    let line = miner
        .log
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(super::now_ms(), kind, channel, text.into());
    if let Some(line) = line {
        let _ = app.emit(EVENT, line);
    }
}

/// Empties the log in every window.
pub fn clear(app: &AppHandle) {
    let miner = app.state::<Miner>();
    miner.log.0.lock().unwrap_or_else(|p| p.into_inner()).clear();
    let _ = app.emit(CLEARED, ());
}

/// Oldest first.
pub fn lines(app: &AppHandle) -> Vec<LogLine> {
    let miner = app.state::<Miner>();
    let log = miner.log.0.lock().unwrap_or_else(|p| p.into_inner());
    log.lines.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_caps_at_a_thousand_with_rising_ids() {
        let mut r = Ring::default();
        for i in 0..1200 {
            r.push(i, LogKind::Points, None, format!("{i}"));
        }
        assert_eq!(r.lines.len(), MAX);
        assert_eq!(r.lines.front().unwrap().id, 201);
        assert_eq!(r.lines.back().unwrap().id, 1200);
        assert!(r.lines.iter().zip(r.lines.iter().skip(1)).all(|(a, b)| a.id < b.id));
    }

    #[test]
    fn clear_empties_and_ids_keep_rising() {
        let mut r = Ring::default();
        r.push(1, LogKind::Bet, Some("a"), "x".into());
        r.clear();
        let l = r.push(2, LogKind::Bet, Some("a"), "x".into()).unwrap();
        assert_eq!(l.id, 2);
        assert_eq!(r.lines.len(), 1);
    }

    #[test]
    fn repeated_errors_collapse() {
        let mut r = Ring::default();
        assert!(r.push(1, LogKind::Error, Some("a"), "e".into()).is_some());
        assert!(r.push(2, LogKind::Error, Some("a"), "e".into()).is_none());
        assert!(r.push(3, LogKind::Error, Some("b"), "e".into()).is_some());
        assert!(r.push(4, LogKind::Points, Some("b"), "e".into()).is_some());
    }

    #[test]
    fn alternating_channel_errors_collapse_per_channel() {
        let mut r = Ring::default();
        assert!(r.push(1, LogKind::Error, Some("a"), "e".into()).is_some());
        assert!(r.push(2, LogKind::Error, Some("b"), "e".into()).is_some());
        assert!(r.push(3, LogKind::Error, Some("a"), "e".into()).is_none());
    }
}
