//! V108-C35 Test builds only: the hype train mock (`tools/mock/hypetrain.mjs`), named by
//! `CYEX_HYPE_MOCK=http://127.0.0.1:<port>` on a test data folder. It answers the train reads
//! and the account's Twitch emotes, and stands in for both topics: once a chat follows the
//! train, its messages are asked for every second and handled like Hermes ones.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cyex_twitch::hermes::{Event, Kind, Topic};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::auth::Auth;

pub const VAR: &str = "CYEX_HYPE_MOCK";
const WHAT: &str = "hype train";
const EVERY: Duration = Duration::from_secs(1);

static STARTED: AtomicBool = AtomicBool::new(false);

/// `http` to the mock when it is set (`mock_url::http_via`'s rules), else as is.
pub fn http(app: &AppHandle, http: cyex_twitch::Http) -> cyex_twitch::Http {
    crate::mock_url::http_via(app, http, &[VAR], WHAT)
}

#[derive(Deserialize)]
struct Batch {
    next: u64,
    events: Vec<Message>,
}

#[derive(Deserialize)]
struct Message {
    /// The user's rewards topic, else the channel's train.
    #[serde(default)]
    user: bool,
    /// The channel id, or the user id.
    id: String,
    payload: Value,
}

/// Starts asking the mock for topic messages, once a run.
pub fn start(app: &AppHandle) {
    let Ok(Some(base)) = crate::mock_url::gated(app, &[VAR], WHAT) else { return };
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let http = app.state::<Auth>().http.clone();
        let mut after = 0u64;
        loop {
            tokio::time::sleep(EVERY).await;
            let Ok(batch) = http.get_json::<Batch>(&format!("{base}/_mock/events?after={after}")).await else { continue };
            after = batch.next;
            for m in batch.events {
                let kind = if m.user { Kind::HypeRewards } else { Kind::HypeTrain };
                let e = Event { topic: Topic::new(kind, &m.id), payload: m.payload };
                if m.user {
                    super::hype_unlock::on_user_event(&app, &e);
                } else {
                    super::hypetrain::on_event(&app, &e);
                }
            }
        }
    });
}
