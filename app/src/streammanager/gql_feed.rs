//! 1.35 The activity feed with a GQL sign-in (D30): Twitch's dashboard alerts and, for
//! watch streaks, its feed lines, both on the account's signed-in Hermes connection. D1 It
//! opens with the last 100 events. D4 Alert updates say where an alert is in Twitch's alert
//! box. D5, D6 The ad schedule and join requests tell the cards to read again.

use cyex_core::number::grouped;
use cyex_eventsub::alerts::{Feed, Past, parse_alert, parse_alert_update, parse_past, parse_streak};
use cyex_eventsub::events::Who;
use cyex_settings::Partition;
use cyex_settings::schema::StreamManager as Options;
use cyex_twitch::gql::dashboard as dash;
use cyex_twitch::hermes::{Event, Kind};
use tauri::{AppHandle, Emitter, Manager};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::activity::{self, Item, with_text};
use super::{State, lock, push_activity};
use crate::settings::Settings;

const KINDS: [Kind; 4] = [Kind::ActivityAlerts, Kind::ActivityFeed, Kind::AdsManager, Kind::GuestStarUser];

pub fn start(app: &AppHandle, account: &str) {
    for kind in KINDS {
        crate::hermes::add_user(app, account, kind);
    }
    let (app, account) = (app.clone(), account.to_owned());
    tauri::async_runtime::spawn(async move { history(&app, &account).await });
}

pub fn stop(app: &AppHandle, account: &str) {
    for kind in KINDS {
        crate::hermes::remove_user(app, account, kind);
    }
}

/// D1 The last 100 events, merged under what came in live meanwhile.
async fn history(app: &AppHandle, account: &str) {
    let Some((token, client_id)) = crate::auth::gql_token(app, account) else {
        return;
    };
    let http = app.state::<crate::auth::Auth>().http.clone();
    let auth = crate::auth::gql_auth(&token, &client_id);
    match dash::feed::history(&http, auth, account).await {
        Ok(nodes) => super::merge_activity(app, nodes.iter().filter_map(parse_past).map(past_item).collect()),
        Err(e) => eprintln!("stream manager: no feed history: {e}"),
    }
}

fn past_item(p: Past) -> Item {
    let ts = OffsetDateTime::parse(&p.at, &Rfc3339)
        .map(|t| (t.unix_timestamp_nanos() / 1_000_000) as i64)
        .unwrap_or(0);
    let mut it = match &p.what {
        Feed::Activity(a) => activity::item(a, p.id.clone(), ts),
        Feed::Streak { who, streams, text } => streak(p.id.clone(), who, *streams, text, ts),
    };
    if p.status.is_some() {
        it.alert = Some(p.id);
        it.status = p.status;
    }
    it
}

fn streak(id: String, who: &Who, streams: u64, text: &str, ts: i64) -> Item {
    let name = if who.name.is_empty() { &who.login } else { &who.name };
    let line = format!("{name} hit a {}-stream watch streak.", grouped(streams));
    Item {
        id,
        kind: "streak".into(),
        text: with_text(line, text),
        user: Some(who.login.clone()),
        ts,
        alert: None,
        status: None,
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
    let data = &e.payload["data"];
    let id = || data["id"].as_str().map_or_else(cyex_chat::build::local_id, str::to_owned);
    match e.topic.kind {
        Kind::ActivityAlerts => {
            if let Some(a) = parse_alert(&e.payload) {
                let mut it = activity::item(&a, id(), crate::chat::now_ms());
                // D4 With its id, it can be played again or skipped.
                if data["id"].is_string() {
                    it.alert = Some(it.id.clone());
                    it.status = data["status"].as_str().map(str::to_owned);
                }
                push_activity(app, it);
            } else if let Some((alert, status)) = parse_alert_update(&e.payload) {
                moved(app, &alert, &status);
            }
        }
        Kind::ActivityFeed => {
            if let Some((who, streams, text)) = parse_streak(&e.payload) {
                push_activity(app, streak(id(), &who, streams, &text, crate::chat::now_ms()));
            }
        }
        Kind::AdsManager => {
            let _ = app.emit("stream:ads", ());
        }
        Kind::GuestStarUser => {
            let _ = app.emit("stream:together", ());
        }
        _ => {}
    }
}

/// D4 An alert moved in Twitch's alert box. Once one plays, the channel uses it and the
/// feed offers Replay and Skip from then on.
fn moved(app: &AppHandle, alert: &str, status: &str) {
    super::set_status(app, alert, status);
    if matches!(status, "PLAYING" | "PLAYED") {
        let settings = app.state::<Settings>();
        if !settings.read::<Options>(Partition::StreamManager).alert_box {
            let _ = settings.update::<Options>(app, Partition::StreamManager, |o| {
                o.alert_box = true;
                true
            });
        }
    }
}
