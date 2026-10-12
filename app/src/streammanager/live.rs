//! D7 Live numbers from Hermes instead of polling: viewers (`video-playback-by-id`, about
//! every 30 s), going live or offline, title and category (`broadcast-settings-update`),
//! and D2 a raid counting down from the channel (`raid`). Anonymous topics, so they work
//! with any sign-in. Followers and subs have no live updates and stay polled (`stats`).

use cyex_twitch::gql::dashboard::raids::{RaidEvent, RaidStatus, parse_raid};
use cyex_twitch::hermes::{Event, Kind};
use tauri::{AppHandle, Emitter, Manager};

use super::{State, lock, push_viewers, stats};

const KINDS: [Kind; 3] = [Kind::Playback, Kind::Raid, Kind::BroadcastSettings];
const RAID_EVENT: &str = "stream:raid";

pub fn start(app: &AppHandle, channel_id: &str) {
    for kind in KINDS {
        crate::hermes::add(app, kind, channel_id);
    }
}

pub fn stop(app: &AppHandle, channel_id: &str) {
    for kind in KINDS {
        crate::hermes::remove(app, kind, channel_id);
    }
    set_raid(app, None);
}

pub fn on_event(app: &AppHandle, e: &Event) {
    let ours = lock(&app.state::<State>().live)
        .as_ref()
        .is_some_and(|l| l.channel_id == e.topic.channel_id);
    if !ours {
        return;
    }
    let p = &e.payload;
    match e.topic.kind {
        Kind::Playback => match p["type"].as_str() {
            Some("viewcount") => {
                if let Some(v) = p["viewers"].as_u64() {
                    stats::pushed(app, |s| {
                        s.live = true;
                        s.viewers = v;
                    });
                    push_viewers(app, crate::auth::unix_now(), v);
                }
            }
            // The start time and the rest come with a full read.
            Some("stream-up" | "stream-down") => stats::refresh(app),
            _ => {}
        },
        Kind::BroadcastSettings if p["type"] == "broadcast_settings_update" => stats::pushed(app, |s| {
            if let Some(t) = p["status"].as_str() {
                s.title = t.to_owned();
            }
            if let Some(g) = p["game"].as_str() {
                s.game = g.to_owned();
            }
        }),
        Kind::Raid => match parse_raid(p, &e.topic.channel_id) {
            Some(RaidEvent::Update(r)) => set_raid(app, Some(r)),
            Some(RaidEvent::Done { id }) => {
                let current = lock(&app.state::<State>().raid).as_ref().map(|r| r.id.clone());
                if current.is_none_or(|c| c == id) {
                    set_raid(app, None);
                }
            }
            None => {}
        },
        _ => {}
    }
}

/// The raid counting down, for a view that opens mid-raid.
pub fn raid(app: &AppHandle) -> Option<RaidStatus> {
    lock(&app.state::<State>().raid).clone()
}

fn set_raid(app: &AppHandle, raid: Option<RaidStatus>) {
    {
        let st = app.state::<State>();
        let mut cur = lock(&st.raid);
        if *cur == raid {
            return;
        }
        cur.clone_from(&raid);
    }
    let _ = app.emit(RAID_EVENT, raid);
}
