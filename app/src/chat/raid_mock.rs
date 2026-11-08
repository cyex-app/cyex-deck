//! V110-U43 Test builds only: the raid mock (`tools/mock/raid.mjs`), named by
//! `CYEX_RAID_MOCK=http://127.0.0.1:<port>` on a test data folder. It stands in for the
//! `raid.<channel id>` topic (asked for messages every second once a chat follows it, handled
//! like Hermes ones) and answers `joinRaid`, so joining never reaches Twitch in a test.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cyex_twitch::hermes::{Event, Kind, Topic};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::auth::Auth;

pub const VAR: &str = "CYEX_RAID_MOCK";
const WHAT: &str = "raid";
const EVERY: Duration = Duration::from_secs(1);

static STARTED: AtomicBool = AtomicBool::new(false);

/// `http` to the mock when it is set (`mock_url::http_via`'s rules), else as is.
pub fn http(app: &AppHandle, http: cyex_twitch::Http) -> cyex_twitch::Http {
    crate::mock_url::http_via(app, http, &[VAR], WHAT)
}

/// A stand-in sign-in while the mock is set, so a signed-out test copy can press Join.
pub fn fake_token(app: &AppHandle) -> Option<(String, String)> {
    matches!(crate::mock_url::gated(app, &[VAR], WHAT), Ok(Some(_))).then(|| ("mock".to_owned(), "mock".to_owned()))
}

#[derive(Deserialize)]
struct Batch {
    next: u64,
    events: Vec<Message>,
}

#[derive(Deserialize)]
struct Message {
    /// The raiding channel's id.
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
                let e = Event { topic: Topic::new(Kind::Raid, &m.id), payload: m.payload };
                super::raid::on_event(&app, &e);
            }
        }
    });
}
