//! V108-C37b The go-live message a streamer set for their notifications, anonymous reads.
//! Checked 2026/11/03 (signed out): `User.broadcastSettings.liveUpNotificationInfo
//! { liveUpNotification isDefault }`; an unchanged one reads "<name> went live!" with
//! `isDefault: true`. `users(ids:)` batches the channels that just went live.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::{Auth, query};
use crate::{Http, Result};

/// Ids per call, like the schedule's batch.
const BATCH: usize = 50;

/// The streamer's own go-live message per user id; channels on Twitch's default are left out.
pub async fn live_messages(http: &Http, ids: &[String]) -> Result<HashMap<String, String>> {
    let q = "query($ids: [ID!]) { users(ids: $ids) { id broadcastSettings { liveUpNotificationInfo { liveUpNotification isDefault } } } }";
    let mut out = HashMap::new();
    for chunk in ids.chunks(BATCH) {
        // One failed chunk keeps what the others gave (V108-F56g).
        let data: Many = match query(http, Auth::PUBLIC, q, json!({ "ids": chunk })).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("live_messages: chunk failed: {e}");
                continue;
            }
        };
        out.extend(data.users.into_iter().flatten().filter_map(custom));
    }
    Ok(out)
}

fn custom(u: User) -> Option<(String, String)> {
    let info = u.broadcast_settings?.live_up_notification_info?;
    let text = info.live_up_notification?.trim().to_owned();
    (!info.is_default.unwrap_or(false) && !text.is_empty()).then_some((u.id, text))
}

#[derive(Deserialize)]
struct Many {
    users: Vec<Option<User>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    id: String,
    broadcast_settings: Option<Settings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    live_up_notification_info: Option<Info>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    live_up_notification: Option<String>,
    is_default: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_messages_the_streamer_set() {
        let d: Many = serde_json::from_str(
            r#"{"users":[
              {"id":"1","broadcastSettings":{"liveUpNotificationInfo":{"liveUpNotification":" Ranked grind! ","isDefault":false}}},
              {"id":"2","broadcastSettings":{"liveUpNotificationInfo":{"liveUpNotification":"shroud went live!","isDefault":true}}},
              {"id":"3","broadcastSettings":{"liveUpNotificationInfo":{"liveUpNotification":"","isDefault":false}}},
              {"id":"4","broadcastSettings":null},
              null]}"#,
        )
        .unwrap();
        let got: HashMap<_, _> = d.users.into_iter().flatten().filter_map(custom).collect();
        assert_eq!(got.len(), 1);
        assert_eq!(got["1"], "Ranked grind!");
    }
}
