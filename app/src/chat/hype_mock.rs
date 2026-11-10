//! V108-C35 Test builds only: the hype train mock (`tools/mock/hypetrain.mjs`), named by
//! `CYEX_HYPE_MOCK=http://127.0.0.1:<port>` on a test data folder. It answers the train reads
//! and the account's Twitch emotes, and stands in for both topics: once a chat follows the
//! train, its messages are asked for every second and handled like Hermes ones.
//! V110-T06 It also carries a channel's pin, poll and prediction messages (`kind`), and answers
//! their reads and the votes and bets (`tools/mock/votes.mjs`), so the pinned and votes banners
//! can be tested offline. No new variable: the same `CYEX_HYPE_MOCK`.

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
    /// The user's rewards topic, else the channel's train (or `kind`'s topic).
    #[serde(default)]
    user: bool,
    /// V110-T06 `pins`, `polls` or `predictions`: that channel topic instead of the train.
    #[serde(default)]
    kind: Option<String>,
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
                let kind = topic(m.user, m.kind.as_deref());
                let e = Event { topic: Topic::new(kind, &m.id), payload: m.payload };
                match kind {
                    Kind::HypeRewards => super::hype_unlock::on_user_event(&app, &e),
                    Kind::Pins => super::live::on_event(&app, &e),
                    Kind::Polls | Kind::Predictions => super::votes::on_event(&app, &e),
                    _ => super::hypetrain::on_event(&app, &e),
                }
            }
        }
    });
}

/// The topic a mock message is for: the user's rewards, a `kind` (V110-T06), else the train.
fn topic(user: bool, kind: Option<&str>) -> Kind {
    match (user, kind) {
        (true, _) => Kind::HypeRewards,
        (_, Some("pins")) => Kind::Pins,
        (_, Some("polls")) => Kind::Polls,
        (_, Some("predictions")) => Kind::Predictions,
        _ => Kind::HypeTrain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t06_messages_go_to_their_topic() {
        assert_eq!(topic(true, None), Kind::HypeRewards);
        assert_eq!(topic(true, Some("polls")), Kind::HypeRewards);
        assert_eq!(topic(false, None), Kind::HypeTrain);
        assert_eq!(topic(false, Some("pins")), Kind::Pins);
        assert_eq!(topic(false, Some("polls")), Kind::Polls);
        assert_eq!(topic(false, Some("predictions")), Kind::Predictions);
        assert_eq!(topic(false, Some("nonsense")), Kind::HypeTrain);
    }

    #[test]
    fn t06_batch_reads_a_kind_and_old_messages() {
        let b: Batch = serde_json::from_str(
            r#"{"next":2,"events":[{"user":false,"id":"1","payload":{},"kind":"polls"},{"id":"2","payload":{}}]}"#,
        )
        .unwrap();
        assert_eq!(b.events[0].kind.as_deref(), Some("polls"));
        assert_eq!(b.events[1].kind, None);
        assert!(!b.events[1].user);
    }
}
