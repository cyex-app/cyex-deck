//! S5 Each plugin's log for its console: `print`, `cyex.log`, errors, start and stop.
//! The last 500 lines are kept; a plugin printing more than 50 lines a second is cut short.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use cyex_plugins::Level;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::PluginState;

pub const LOG_EVENT: &str = "plugins:log";
const KEEP: usize = 500;
const PER_SECOND: u32 = 50;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginLog {
    #[ts(type = "number")]
    pub ts: i64,
    pub level: Level,
    pub text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Logged<'a> {
    id: &'a str,
    line: &'a PluginLog,
}

#[derive(Default)]
pub struct Logs(Mutex<HashMap<String, Book>>);

#[derive(Default)]
struct Book {
    lines: VecDeque<PluginLog>,
    second: i64,
    count: u32,
    skipped: u32,
}

pub fn push(app: &AppHandle, id: &str, level: Level, text: &str) {
    let Some(state) = app.try_state::<PluginState>() else {
        return;
    };
    let ts = crate::chat::now_ms();
    let mut out = Vec::new();
    {
        let mut books = state.logs.0.lock().unwrap_or_else(|p| p.into_inner());
        let book = books.entry(id.to_owned()).or_default();
        let second = ts / 1000;
        if book.second != second {
            if book.skipped > 0 {
                out.push(PluginLog {
                    ts,
                    level: Level::Warning,
                    text: format!("{} more lines left out", book.skipped),
                });
            }
            book.second = second;
            book.count = 0;
            book.skipped = 0;
        }
        book.count += 1;
        if book.count > PER_SECOND {
            book.skipped += 1;
        } else {
            out.push(PluginLog {
                ts,
                level,
                text: text.to_owned(),
            });
        }
        for line in &out {
            if book.lines.len() >= KEEP {
                book.lines.pop_front();
            }
            book.lines.push_back(line.clone());
        }
    }
    for line in &out {
        let _ = app.emit(LOG_EVENT, Logged { id, line });
    }
}

pub fn lines(app: &AppHandle, id: &str) -> Vec<PluginLog> {
    let state = app.state::<PluginState>();
    let books = state.logs.0.lock().unwrap_or_else(|p| p.into_inner());
    books
        .get(id)
        .map(|b| b.lines.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn clear(app: &AppHandle, id: &str) {
    let state = app.state::<PluginState>();
    let mut books = state.logs.0.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(b) = books.get_mut(id) {
        b.lines.clear();
    }
}
