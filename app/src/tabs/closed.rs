//! U10 What the last edits closed, newest last, until the app quits.

use std::sync::Mutex;

use cyex_settings::schema::Closed;
use tauri::{AppHandle, Emitter, Manager};

/// Edits that can be undone this way.
const KEEP: usize = 20;
const EVENT: &str = "tabs:reopenable";

#[derive(Default)]
pub struct ClosedTabs(Mutex<Vec<Vec<Closed>>>);

impl ClosedTabs {
    fn with<T>(&self, f: impl FnOnce(&mut Vec<Vec<Closed>>) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(|p| p.into_inner()))
    }
}

pub fn push(app: &AppHandle, closed: Vec<Closed>) {
    if closed.is_empty() {
        return;
    }
    app.state::<ClosedTabs>().with(|stack| {
        stack.push(closed);
        if stack.len() > KEEP {
            stack.remove(0);
        }
    });
    let _ = app.emit(EVENT, true);
}

pub fn pop(app: &AppHandle) -> Option<Vec<Closed>> {
    let (last, left) = app.state::<ClosedTabs>().with(|stack| (stack.pop(), !stack.is_empty()));
    if last.is_some() && !left {
        let _ = app.emit(EVENT, false);
    }
    last
}

pub fn any(app: &AppHandle) -> bool {
    app.state::<ClosedTabs>().with(|stack| !stack.is_empty())
}
