//! Follows on your own channel when the stream manager runs on EventSub (Helix, D30).
//! Twitch keeps one `channel.follow` per channel and account, so with your chat open the
//! mod feed and the stream manager can't both hold it. Whichever holds it hands each
//! follow to both, and when one session ends, the other subscribes it again.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use cyex_eventsub::events::Who;
use cyex_twitch::helix;
use tauri::{AppHandle, Manager};

use super::{activity, lock, push_activity};
use crate::auth;

/// Follows already shown, (channel id, user id); a follow both sessions get shows once.
#[derive(Default)]
pub struct Seen(Mutex<VecDeque<(String, String)>>);

const KEEP: usize = 64;

/// True the first time this follow comes in.
fn first(seen: &Seen, channel_id: &str, user_id: &str) -> bool {
    let mut q = lock(&seen.0);
    if q.iter().any(|(c, u)| c == channel_id && u == user_id) {
        return false;
    }
    q.push_back((channel_id.to_owned(), user_id.to_owned()));
    while q.len() > KEEP {
        q.pop_front();
    }
    true
}

/// A follow from either session: into the mod view and, when the stream manager runs on
/// EventSub for this channel, its activity feed.
pub fn seen(app: &AppHandle, channel_id: &str, user: &Who) {
    if !first(&app.state::<Seen>(), channel_id, &user.id) {
        return;
    }
    let name = if user.name.is_empty() {
        &user.login
    } else {
        &user.name
    };
    let text = format!("{name} followed.");
    crate::moderation::feeds::follow_line(app, channel_id, text.clone(), user.login.clone());
    if super::eventsub_session(app).is_some_and(|(_, cid, _)| cid == channel_id) {
        push_activity(
            app,
            activity::Item {
                id: cyex_chat::build::local_id(),
                kind: "follow".into(),
                text,
                user: Some(user.login.clone()),
                ts: crate::chat::now_ms(),
                alert: None,
                status: None,
            },
        );
    }
}

/// Subscribes `channel.follow` for `channel_id` on `session_id` as `account`, after the
/// session that held it closed. Twitch may still count the old one for a moment, so a
/// duplicate is tried again a few times.
pub fn take_over(app: &AppHandle, account: &str, channel_id: &str, session_id: &str) {
    let Some((token, client_id)) = auth::helix_token(app, account) else {
        return;
    };
    let app = app.clone();
    let cond = serde_json::json!({ "broadcaster_user_id": channel_id, "moderator_user_id": account });
    let session_id = session_id.to_owned();
    tauri::async_runtime::spawn(async move {
        let http = app.state::<auth::Auth>().http.clone();
        let auth = crate::auth::helix_auth(&token, &client_id);
        for _ in 0..4 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            match helix::eventsub::subscribe(&http, auth, &session_id, "channel.follow", "2", &cond).await {
                Err(e) if e.api_message().is_some_and(|m| m.contains("already")) => continue,
                _ => return,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_follow_shows_once() {
        let s = Seen::default();
        assert!(first(&s, "1", "a"));
        assert!(!first(&s, "1", "a"));
        assert!(first(&s, "2", "a"));
        for i in 0..KEEP {
            first(&s, "1", &i.to_string());
        }
        // Old ones age out.
        assert!(first(&s, "1", "a"));
    }
}
