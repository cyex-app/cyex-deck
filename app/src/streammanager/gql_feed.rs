//! 1.35 The activity feed with a GQL sign-in (D30): Twitch's dashboard alerts and, for
//! watch streaks, its feed lines, both on the account's signed-in Hermes connection.

use cyex_core::number::grouped;
use cyex_eventsub::alerts::{parse_alert, parse_streak};
use cyex_twitch::hermes::{Event, Kind};
use tauri::{AppHandle, Manager};

use super::activity::{self, Item, with_text};
use super::{State, lock, push_activity};

const KINDS: [Kind; 2] = [Kind::ActivityAlerts, Kind::ActivityFeed];

pub fn start(app: &AppHandle, account: &str) {
    for kind in KINDS {
        crate::hermes::add_user(app, account, kind);
    }
}

pub fn stop(app: &AppHandle, account: &str) {
    for kind in KINDS {
        crate::hermes::remove_user(app, account, kind);
    }
}

/// A dashboard message, when the manager runs on GQL for that channel.
pub fn on_event(app: &AppHandle, e: &Event) {
    let ours = lock(&app.state::<State>().live)
        .as_ref()
        .is_some_and(|l| l.session.is_none() && l.channel_id == e.topic.channel_id);
    if !ours {
        return;
    }
    match e.topic.kind {
        Kind::ActivityAlerts => {
            if let Some(a) = parse_alert(&e.payload) {
                activity::push(app, &a);
            }
        }
        Kind::ActivityFeed => {
            if let Some((who, streams, text)) = parse_streak(&e.payload) {
                let name = if who.name.is_empty() {
                    &who.login
                } else {
                    &who.name
                };
                let line = format!("{name} hit a {}-stream watch streak.", grouped(streams));
                push_activity(
                    app,
                    Item {
                        id: cyex_chat::build::local_id(),
                        kind: "streak".into(),
                        text: with_text(line, &text),
                        user: Some(who.login),
                        ts: crate::chat::now_ms(),
                    },
                );
            }
        }
        _ => {}
    }
}
