//! F4 Feed tabs: messages that match a rule with "Also show in" are copied into the feed
//! with that name, from every joined chat. Mentions (V104-01) and global mentions from
//! the firehose (V104-02) go to the one named "Mentions". A feed keeps its last lines while no tab shows
//! it, and deletions and timeouts in the source chat apply to the copies too.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use cyex_chat::model::{ChatMessage, Flag, Gone, RoomEvent};
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
    /// V104-02 Found in logs.cyex.app's firehose, not in an open chat.
    pub global: bool,
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
type Hit = (Option<Gone>, Box<dyn Fn(&ChatMessage) -> bool>);

#[derive(Default)]
struct Feed {
    lines: VecDeque<FeedLine>,
    /// View id, its window, its channel.
    views: Vec<(u32, String, View)>,
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
    push(app, channel, message, routes, false);
}

/// V104-02 A global mention's message, marked as such.
pub fn route_global(app: &AppHandle, channel: &str, message: &Arc<ChatMessage>, routes: &[String]) {
    push(app, channel, message, routes, true);
}

fn push(app: &AppHandle, channel: &str, message: &Arc<ChatMessage>, routes: &[String], global: bool) {
    let Some(feeds) = app.try_state::<Feeds>() else {
        return;
    };
    let alive = |w: &str| app.get_webview_window(w).is_some();
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for name in routes {
        let feed = map.entry(key(name)).or_default();
        // The same message from an open chat and from the firehose shows once.
        if feed.lines.iter().any(|l| l.message.id == message.id) {
            continue;
        }
        let line = FeedLine {
            channel: channel.to_owned(),
            message: message.clone(),
            global,
        };
        if feed.lines.len() >= LINES {
            feed.lines.pop_front();
        }
        feed.lines.push_back(line.clone());
        send(feed, FeedEvent::Line { line }, &alive);
    }
}

/// Deletions and timeouts in a chat reach its copies.
pub fn moderate(app: &AppHandle, channel: &str, events: &[RoomEvent]) {
    let hits: Vec<Hit> = events
        .iter()
        .filter_map(|e| -> Option<Hit> {
            match e {
                RoomEvent::Delete { id, gone } => {
                    let id = id.clone();
                    Some((gone.clone(), Box::new(move |m| m.id == id)))
                }
                RoomEvent::ClearUser { user_id, gone } => {
                    let user = user_id.clone();
                    Some((
                        Some(gone.clone()),
                        Box::new(move |m| m.notice.is_none() && m.sender_id() == Some(user.as_str())),
                    ))
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
    let alive = |w: &str| app.get_webview_window(w).is_some();
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for feed in map.values_mut() {
        let mut changed = Vec::new();
        for line in feed.lines.iter_mut().filter(|l| l.channel == channel) {
            // V107-R34 Why it went comes along, for the note.
            let Some((gone, _)) = hits.iter().find(|(_, h)| h(&line.message)) else { continue };
            let mut m = (*line.message).clone();
            let fresh = match gone {
                Some(g) => m.mark_gone(g),
                None => !m.has(Flag::Deleted),
            };
            if fresh {
                m.set(Flag::Deleted);
                line.message = Arc::new(m);
                changed.push(line.clone());
            }
        }
        for line in changed {
            send(feed, FeedEvent::Update { line }, &alive);
        }
    }
}

/// V107-R1 Views whose window is gone leave: a closed window's channel still takes sends
/// without an error, so they never left before.
fn send(feed: &mut Feed, event: FeedEvent, alive: &dyn Fn(&str) -> bool) {
    feed.views.retain(|(_, w, v)| alive(w) && v.send(vec![event.clone()]).is_ok());
}

/// Starts streaming a feed. Returns the view id and the lines it has.
pub fn open(app: &AppHandle, name: &str, window: &str, view: View) -> (u32, Vec<FeedLine>) {
    let feeds = app.state::<Feeds>();
    let id = {
        let mut n = feeds.next_view.lock().unwrap_or_else(|p| p.into_inner());
        *n += 1;
        *n
    };
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    let feed = map.entry(key(name)).or_default();
    feed.views.push((id, window.to_owned(), view));
    (id, feed.lines.iter().cloned().collect())
}

pub fn close(app: &AppHandle, view: u32) {
    let feeds = app.state::<Feeds>();
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for feed in map.values_mut() {
        feed.views.retain(|(id, _, _)| *id != view);
    }
}

/// V107-R1 Closes every view of a window that closed or reloaded.
pub fn close_window(app: &AppHandle, window: &str) {
    let Some(feeds) = app.try_state::<Feeds>() else { return };
    let mut map = feeds.feeds.lock().unwrap_or_else(|p| p.into_inner());
    for feed in map.values_mut() {
        feed.views.retain(|(_, w, _)| w != window);
    }
}

#[cfg(test)]
mod tests {
    use cyex_chat::model::NoticeKind;

    use super::*;

    fn line() -> FeedLine {
        FeedLine {
            channel: "c".into(),
            message: Arc::new(cyex_chat::build::system(NoticeKind::Info, "hi", 0)),
            global: false,
        }
    }

    #[test]
    fn v107_r1_views_of_closed_windows_leave() {
        let mut feed = Feed::default();
        for i in 0..1000 {
            feed.views.push((i, format!("pop{i}"), Channel::new(|_| Ok(()))));
        }
        feed.views.push((1000, "main".into(), Channel::new(|_| Ok(()))));
        send(&mut feed, FeedEvent::Line { line: line() }, &|w| w == "main");
        assert_eq!(feed.views.len(), 1);
        assert_eq!(feed.views[0].1, "main");
    }
}
