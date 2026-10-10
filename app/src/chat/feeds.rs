//! F4 Feed tabs: messages that match a rule with "Also show in" are copied into the feed
//! with that name, from every joined chat. A feed keeps its last lines while no tab shows
//! it, and deletions and timeouts in the source chat apply to the copies too.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use cyex_chat::model::{ChatMessage, Flag, RoomEvent};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

/// Lines a feed keeps.
const LINES: usize = 500;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FeedLine {
    /// Channel login it was said in.
    pub channel: String,
    #[ts(as = "ChatMessage")]
    pub message: Arc<ChatMessage>,
}

#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum FeedEvent {
    Line {
        line: FeedLine,
    },
    /// A copy was deleted in its chat.
    Update {
        line: FeedLine,
    },
}

type View = Channel<Vec<FeedEvent>>;
/// Which copies a deletion reaches.
type Hit = Box<dyn Fn(&ChatMessage) -> bool>;

#[derive(Default)]
struct Feed {
    lines: VecDeque<FeedLine>,
    views: Vec<(u32, View)>,
}

#[derive(Default)]
pub struct Feeds {
    feeds: Mutex<HashMap<String, Feed>>,
    next_view: Mutex<u32>,
}

fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub fn init(app: &AppHandle) {
    app.manage(Feeds::default());
}

/// Copies a message into the feeds it was routed to.
pub fn route(app: &AppHandle, channel: &str, message: &Arc<ChatMessage>, routes: &[String]) {
    let Some(feeds) = app.try_state::<Feeds>() else {
        return;
    };
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for name in routes {
        let feed = map.entry(key(name)).or_default();
        if feed.lines.iter().any(|l| l.message.id == message.id) {
            continue;
        }
        let line = FeedLine {
            channel: channel.to_owned(),
            message: message.clone(),
        };
        if feed.lines.len() >= LINES {
            feed.lines.pop_front();
        }
        feed.lines.push_back(line.clone());
        send(feed, FeedEvent::Line { line });
    }
}

/// Deletions and timeouts in a chat reach its copies.
pub fn moderate(app: &AppHandle, channel: &str, events: &[RoomEvent]) {
    let hits: Vec<Hit> = events
        .iter()
        .filter_map(|e| -> Option<Hit> {
            match e {
                RoomEvent::Delete { id } => {
                    let id = id.clone();
                    Some(Box::new(move |m| m.id == id))
                }
                RoomEvent::ClearUser { user_id } => {
                    let user = user_id.clone();
                    Some(Box::new(move |m| {
                        m.notice.is_none() && m.sender_id() == Some(user.as_str())
                    }))
                }
                _ => None,
            }
        })
        .collect();
    if hits.is_empty() {
        return;
    }
    let Some(feeds) = app.try_state::<Feeds>() else {
        return;
    };
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for feed in map.values_mut() {
        let mut changed = Vec::new();
        for line in feed.lines.iter_mut().filter(|l| l.channel == channel) {
            if !line.message.has(Flag::Deleted) && hits.iter().any(|h| h(&line.message)) {
                let mut m = (*line.message).clone();
                m.set(Flag::Deleted);
                line.message = Arc::new(m);
                changed.push(line.clone());
            }
        }
        for line in changed {
            send(feed, FeedEvent::Update { line });
        }
    }
}

fn send(feed: &mut Feed, event: FeedEvent) {
    feed.views.retain(|(_, v)| v.send(vec![event.clone()]).is_ok());
}

/// Starts streaming a feed. Returns the view id and the lines it has.
pub fn open(app: &AppHandle, name: &str, view: View) -> (u32, Vec<FeedLine>) {
    let feeds = app.state::<Feeds>();
    let id = {
        let mut n = feeds.next_view.lock().unwrap_or_else(|p| p.into_inner());
        *n += 1;
        *n
    };
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    let feed = map.entry(key(name)).or_default();
    feed.views.push((id, view));
    (id, feed.lines.iter().cloned().collect())
}

pub fn close(app: &AppHandle, view: u32) {
    let feeds = app.state::<Feeds>();
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for feed in map.values_mut() {
        feed.views.retain(|(id, _)| *id != view);
    }
}
