//! Sends each room's queued events to its views, at most every 50 ms (ARCHITECTURE §2).
//! Sleeps while chat is quiet. Each view gets only what its filters let through (F3).
//! With a chat delay (C19) other people's messages wait until their time; deletions and
//! timeouts don't wait and reach the held messages too, so those never show unstruck.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use cyex_chat::model::{ChatMessage, Flag, RoomEvent};
use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use super::{Chat, Entry, RoomKey, now_ms, passes};

const BATCH: Duration = Duration::from_millis(50);

pub fn spawn(app: AppHandle, wake: Arc<Notify>) {
    tauri::async_runtime::spawn(async move {
        let mut next: Option<i64> = None;
        loop {
            match next {
                Some(at) => {
                    let wait = Duration::from_millis((at - now_ms()).max(0) as u64);
                    tokio::select! {
                        () = wake.notified() => {}
                        () = tokio::time::sleep(wait) => {}
                    }
                }
                None => wake.notified().await,
            }
            tokio::time::sleep(BATCH).await;
            next = flush(&app);
        }
    });
}

/// Returns when the next held message is due.
fn flush(app: &AppHandle) -> Option<i64> {
    let chat = app.state::<Chat>();
    let delay = chat.delay_ms.load(Ordering::Relaxed);
    let now = now_ms();
    let mut next: Option<i64> = None;
    let mut inner = chat.lock();
    // S8 One room per channel tells plugins what happened.
    let reporters: Vec<RoomKey> = if crate::plugins::events::wanted(app) {
        let mut logins: Vec<&String> = inner.rooms.keys().map(|k| &k.1).collect();
        logins.sort();
        logins.dedup();
        logins.into_iter().filter_map(|l| super::peek::pick(&inner, l)).collect()
    } else {
        Vec::new()
    };
    for (key, entry) in inner.rooms.iter_mut() {
        let login = &key.1;
        let mut batch = std::mem::take(&mut entry.outbox);
        if delay > 0 || !entry.delayed.is_empty() {
            batch = hold(entry, batch, now, delay);
            if let Some((at, _)) = entry.delayed.front() {
                next = Some(next.map_or(*at, |n| n.min(*at)));
            }
        }
        if batch.is_empty() {
            continue;
        }
        if reporters.contains(key) {
            crate::plugins::events::room(app, login, &batch);
        }
        let filtered = entry.views.iter().any(|s| !s.filters.is_empty());
        let live = filtered && crate::tabs::live::is_live(app, login);
        // A closed window drops its channel; close() removes the view soon after.
        for sink in &entry.views {
            if sink.filters.is_empty() {
                let _ = sink.view.send(batch.clone());
                continue;
            }
            let ok = |m: &ChatMessage| passes(&sink.filters, m, login, live);
            let mine: Vec<RoomEvent> = batch
                .iter()
                .filter_map(|e| match e {
                    RoomEvent::Message { message } if !ok(message) => None,
                    RoomEvent::History { messages } => Some(RoomEvent::History {
                        messages: messages.iter().filter(|m| ok(m)).cloned().collect(),
                    }),
                    RoomEvent::Gap { messages } => Some(RoomEvent::Gap {
                        messages: messages.iter().filter(|m| ok(m)).cloned().collect(),
                    }),
                    other => Some(other.clone()),
                })
                .collect();
            if !mine.is_empty() {
                let _ = sink.view.send(mine);
            }
        }
    }
    next
}

/// C19 Puts new chat lines on hold and returns what may go out now.
fn hold(entry: &mut Entry, batch: Vec<RoomEvent>, now: i64, delay: i64) -> Vec<RoomEvent> {
    let mut out = Vec::new();
    for e in batch {
        match &e {
            RoomEvent::Message { message } if delay > 0 && !message.has(Flag::Own) && message.sender.is_some() => {
                entry.delayed.push_back((now + delay, e));
            }
            RoomEvent::Delete { id } => {
                strike(entry, |m| m.id == *id);
                out.push(e);
            }
            RoomEvent::ClearUser { user_id } => {
                strike(entry, |m| m.notice.is_none() && m.sender_id() == Some(user_id.as_str()));
                out.push(e);
            }
            _ => out.push(e),
        }
    }
    while entry.delayed.front().is_some_and(|(at, _)| *at <= now) {
        if let Some((_, e)) = entry.delayed.pop_front() {
            out.push(e);
        }
    }
    out
}

/// Marks held messages deleted.
fn strike(entry: &mut Entry, hit: impl Fn(&ChatMessage) -> bool) {
    for (_, e) in entry.delayed.iter_mut() {
        if let RoomEvent::Message { message } = e
            && hit(message)
            && !message.has(Flag::Deleted)
        {
            let mut m = (**message).clone();
            m.set(Flag::Deleted);
            *message = Arc::new(m);
        }
    }
}
