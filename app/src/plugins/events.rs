//! S8 Chat events for plugins, after the fact: taken from each room's 50 ms batch (after
//! the C19 hold), from one room per channel, so a channel open twice reports once. Each
//! plugin gets only the kinds it follows, never its own lines. Costs nothing while no
//! plugin follows any. S12 Tab channels going live or offline (from the once-a-minute
//! check) and the tabs changing come the same way.

use std::sync::atomic::Ordering;

use cyex_chat::model::RoomEvent;
use cyex_plugins::{ChatEvent, Event, LiveChange};
use tauri::{AppHandle, Manager};

use super::{PluginState, hook, host, running};

/// Some running plugin follows a chat event.
pub fn wanted(app: &AppHandle) -> bool {
    app.try_state::<PluginState>()
        .is_some_and(|s| s.wants_events.load(Ordering::Relaxed))
}

/// A room's batch, as each plugin sees it.
pub fn room(app: &AppHandle, login: &str, batch: &[RoomEvent]) {
    let events: Vec<ChatEvent> = batch.iter().filter_map(|e| event(login, e)).collect();
    if !events.is_empty() {
        hand_out(app, &events);
    }
}

/// Tab channels renamed on Twitch: (login now, login before, display name).
pub fn renamed(app: &AppHandle, list: Vec<(String, String, String)>) {
    if !wanted(app) || list.is_empty() {
        return;
    }
    let events: Vec<ChatEvent> = list
        .into_iter()
        .map(|(channel, old, name)| ChatEvent::Renamed { channel, old, name })
        .collect();
    hand_out(app, &events);
}

/// S12 Tab channels whose stream started or ended since the last check.
pub fn live(app: &AppHandle, changes: Vec<LiveChange>) {
    if !changes.is_empty() {
        hand_out(app, &changes.into_iter().map(ChatEvent::Live).collect::<Vec<_>>());
    }
}

/// S12 The tabs partition changed. Plugins hear about it when what they see of the tabs
/// did (a resize or a live check's rename doesn't count).
pub fn layout(app: &AppHandle) {
    let Some(state) = app.try_state::<PluginState>() else { return };
    let now = host::layout(app);
    let mut last = state.last_layout.lock().unwrap_or_else(|p| p.into_inner());
    if *last == now {
        return;
    }
    *last = now.clone();
    drop(last);
    hand_out(app, &[ChatEvent::Layout(now)]);
}

fn event(login: &str, e: &RoomEvent) -> Option<ChatEvent> {
    Some(match e {
        RoomEvent::Message { message } => ChatEvent::Appended(hook::to_line(login, message)),
        RoomEvent::Update { message } => ChatEvent::Replaced(hook::to_line(login, message)),
        // V107-R34 A mod feed's repeat (naming the mod) isn't a second delete.
        RoomEvent::Delete { gone: Some(g), .. } | RoomEvent::ClearUser { gone: g, .. } if g.by.is_some() => {
            return None;
        }
        RoomEvent::Delete { id, .. } => ChatEvent::Deleted {
            channel: login.to_owned(),
            id: Some(id.clone()),
            user_id: None,
        },
        RoomEvent::ClearUser { user_id, .. } => ChatEvent::Deleted {
            channel: login.to_owned(),
            id: None,
            user_id: Some(user_id.clone()),
        },
        RoomEvent::Cleared => ChatEvent::Cleared {
            channel: login.to_owned(),
        },
        _ => return None,
    })
}

fn hand_out(app: &AppHandle, events: &[ChatEvent]) {
    for p in running(app) {
        let wants: Vec<Event> = Event::ALL.into_iter().filter(|e| p.has(*e)).collect();
        if wants.is_empty() {
            continue;
        }
        let mine: Vec<ChatEvent> = events
            .iter()
            .filter(|e| wants.contains(&e.kind()) && !own(e, &p.id))
            .cloned()
            .collect();
        if !mine.is_empty() {
            p.chat(mine);
        }
    }
}

/// A plugin never hears about its own lines, so it can't loop.
fn own(e: &ChatEvent, plugin: &str) -> bool {
    matches!(e, ChatEvent::Appended(l) | ChatEvent::Replaced(l) if l.plugin.as_deref() == Some(plugin))
}
